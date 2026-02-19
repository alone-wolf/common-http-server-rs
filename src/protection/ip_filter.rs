//! IP allow/deny filtering middleware.

use axum::{
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use dashmap::DashMap;
use ipnet::IpNet;
use std::{
    net::{IpAddr, SocketAddr},
    sync::Arc,
};
use tracing::{debug, info, warn};

#[derive(Debug, Clone)]
pub struct IpFilterConfig {
    pub whitelist: Vec<IpNet>,
    pub blacklist: Vec<IpNet>,
    pub default_policy: DefaultPolicy,
    pub log_blocked_requests: bool,
    pub trusted_proxies: Vec<IpNet>,
    pub max_cache_entries: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub enum DefaultPolicy {
    Allow,
    Deny,
}

impl Default for IpFilterConfig {
    fn default() -> Self {
        Self {
            whitelist: vec![],
            blacklist: vec![],
            default_policy: DefaultPolicy::Allow,
            log_blocked_requests: true,
            trusted_proxies: vec![],
            max_cache_entries: 10_000,
        }
    }
}

impl IpFilterConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn allow_by_default() -> Self {
        Self {
            default_policy: DefaultPolicy::Allow,
            ..Default::default()
        }
    }

    pub fn deny_by_default() -> Self {
        Self {
            default_policy: DefaultPolicy::Deny,
            ..Default::default()
        }
    }

    pub fn add_whitelist_ip(mut self, ip: impl Into<IpNet>) -> Self {
        self.whitelist.push(ip.into());
        self
    }

    pub fn add_whitelist_ips(mut self, ips: Vec<impl Into<IpNet>>) -> Self {
        for ip in ips {
            self.whitelist.push(ip.into());
        }
        self
    }

    pub fn add_blacklist_ip(mut self, ip: impl Into<IpNet>) -> Self {
        self.blacklist.push(ip.into());
        self
    }

    pub fn add_blacklist_ips(mut self, ips: Vec<impl Into<IpNet>>) -> Self {
        for ip in ips {
            self.blacklist.push(ip.into());
        }
        self
    }

    pub fn log_blocked(mut self, log: bool) -> Self {
        self.log_blocked_requests = log;
        self
    }

    pub fn trust_proxy(mut self, proxy: impl Into<IpNet>) -> Self {
        self.trusted_proxies.push(proxy.into());
        self
    }

    pub fn trust_proxies(mut self, proxies: Vec<impl Into<IpNet>>) -> Self {
        for proxy in proxies {
            self.trusted_proxies.push(proxy.into());
        }
        self
    }

    pub fn max_cache_entries(mut self, max_cache_entries: usize) -> Self {
        self.max_cache_entries = max_cache_entries.max(1);
        self
    }

    pub fn build(self) -> Arc<IpFilterService> {
        Arc::new(IpFilterService::new(self))
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.max_cache_entries == 0 {
            return Err("ip filter max_cache_entries must be greater than 0".to_string());
        }

        for whitelist_net in &self.whitelist {
            if self
                .blacklist
                .iter()
                .any(|blacklist_net| networks_overlap(whitelist_net, blacklist_net))
            {
                return Err(format!(
                    "whitelist network {} overlaps with blacklist",
                    whitelist_net
                ));
            }
        }
        Ok(())
    }
}

fn networks_overlap(a: &IpNet, b: &IpNet) -> bool {
    match (a, b) {
        (IpNet::V4(a), IpNet::V4(b)) => a.contains(&b.network()) || b.contains(&a.network()),
        (IpNet::V6(a), IpNet::V6(b)) => a.contains(&b.network()) || b.contains(&a.network()),
        _ => false,
    }
}

#[derive(Debug, Clone)]
pub struct IpFilterService {
    config: IpFilterConfig,
    // Cache for recently checked IPs to avoid repeated network operations
    cache: Arc<DashMap<IpAddr, bool>>,
}

impl IpFilterService {
    pub fn new(config: IpFilterConfig) -> Self {
        Self {
            config,
            cache: Arc::new(DashMap::new()),
        }
    }

    pub fn is_allowed(&self, ip: IpAddr) -> bool {
        // Check cache first
        if let Some(allowed) = self.cache.get(&ip) {
            return *allowed;
        }

        let allowed = self.check_ip_rules(ip);

        // Cache the result
        if self.cache.len() >= self.config.max_cache_entries {
            self.evict_cache_entries();
            warn!(
                max_cache_entries = self.config.max_cache_entries,
                cache_size = self.cache.len(),
                "IP filter cache reached capacity and evicted old entries"
            );
        }
        self.cache.insert(ip, allowed);

        allowed
    }

    fn check_ip_rules(&self, ip: IpAddr) -> bool {
        // Check blacklist first - if IP is in blacklist, deny immediately
        for network in &self.config.blacklist {
            if network.contains(&ip) {
                if self.config.log_blocked_requests {
                    warn!("IP {} blocked by blacklist: {}", ip, network);
                }
                return false;
            }
        }

        // Check whitelist
        let in_whitelist = self
            .config
            .whitelist
            .iter()
            .any(|network| network.contains(&ip));

        match (self.config.default_policy, in_whitelist) {
            (DefaultPolicy::Allow, false) => {
                debug!("IP {} allowed by default policy", ip);
                true
            }
            (DefaultPolicy::Allow, true) => {
                debug!("IP {} allowed by whitelist", ip);
                true
            }
            (DefaultPolicy::Deny, false) => {
                if self.config.log_blocked_requests {
                    warn!("IP {} denied by default policy (not in whitelist)", ip);
                }
                false
            }
            (DefaultPolicy::Deny, true) => {
                debug!("IP {} allowed by whitelist", ip);
                true
            }
        }
    }

    fn extract_ip(&self, headers: &HeaderMap, peer_ip: Option<IpAddr>) -> Option<IpAddr> {
        crate::core::client_ip::extract_client_ip_with_trusted_proxies(
            headers,
            peer_ip,
            &self.config.trusted_proxies,
        )
    }

    pub fn clear_cache(&self) {
        self.cache.clear();
        info!("IP filter cache cleared");
    }

    pub fn cache_size(&self) -> usize {
        self.cache.len()
    }

    fn evict_cache_entries(&self) {
        let current_len = self.cache.len();
        if current_len < self.config.max_cache_entries {
            return;
        }

        // Evict a chunk to avoid repeated full-cache churn under high-cardinality traffic.
        let target_evict = (self.config.max_cache_entries / 5).max(1);
        let keys_to_remove: Vec<IpAddr> = self
            .cache
            .iter()
            .take(target_evict)
            .map(|entry| *entry.key())
            .collect();
        for key in keys_to_remove {
            self.cache.remove(&key);
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum IpFilterError {
    #[error("IP address is not allowed")]
    IpNotAllowed,
}

impl IntoResponse for IpFilterError {
    fn into_response(self) -> axum::response::Response {
        crate::core::response::ApiResponse::<()>::error_with_status(
            self.to_string(),
            StatusCode::FORBIDDEN,
        )
        .into_response()
    }
}

pub async fn ip_filter_middleware(
    State(service): State<Arc<IpFilterService>>,
    request: Request,
    next: Next,
) -> Result<Response, IpFilterError> {
    let headers = request.headers().clone();
    let peer_ip = request
        .extensions()
        .get::<axum::extract::ConnectInfo<SocketAddr>>()
        .map(|connect_info| connect_info.0.ip());

    // Try to extract IP from headers first
    let ip = service.extract_ip(&headers, peer_ip);

    if let Some(ip) = ip {
        if service.is_allowed(ip) {
            Ok(next.run(request).await)
        } else {
            Err(IpFilterError::IpNotAllowed)
        }
    } else {
        match service.config.default_policy {
            DefaultPolicy::Allow => {
                debug!("Could not determine client IP, allowing request by default policy");
                Ok(next.run(request).await)
            }
            DefaultPolicy::Deny => {
                warn!("Could not determine client IP, denying request by default policy");
                Err(IpFilterError::IpNotAllowed)
            }
        }
    }
}

pub mod presets {
    use super::*;

    pub fn localhost_only() -> IpFilterConfig {
        IpFilterConfig::deny_by_default()
            .add_whitelist_ip("127.0.0.0/8".parse::<IpNet>().unwrap())
            .add_whitelist_ip("::1".parse::<IpNet>().unwrap())
    }

    pub fn private_networks() -> IpFilterConfig {
        IpFilterConfig::deny_by_default().add_whitelist_ips(
            vec![
                "127.0.0.0/8",    // Loopback
                "10.0.0.0/8",     // Private Class A
                "172.16.0.0/12",  // Private Class B
                "192.168.0.0/16", // Private Class C
                "::1/128",        // IPv6 loopback
                "fc00::/7",       // IPv6 private
            ]
            .into_iter()
            .map(|s| s.parse::<IpNet>().unwrap())
            .collect::<Vec<_>>(),
        )
    }

    pub fn block_known_malicious() -> IpFilterConfig {
        // Example blocking some known malicious ranges
        IpFilterConfig::allow_by_default().add_blacklist_ips(
            vec![
                // These are example ranges - replace with actual malicious IPs
                "0.0.0.0/8",      // Reserved
                "169.254.0.0/16", // Link-local
                "224.0.0.0/4",    // Multicast
            ]
            .into_iter()
            .map(|s| s.parse::<IpNet>().unwrap())
            .collect::<Vec<_>>(),
        )
    }

    pub fn corporate_network() -> IpFilterConfig {
        // Example corporate network ranges - customize as needed
        IpFilterConfig::deny_by_default().add_whitelist_ips(
            vec![
                "192.168.1.0/24", // Office network
                "10.0.0.0/8",     // VPN
                "203.0.113.0/24", // Public office IP
            ]
            .into_iter()
            .map(|s| s.parse::<IpNet>().unwrap())
            .collect::<Vec<_>>(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        Router,
        body::Body,
        http::{Request as HttpRequest, StatusCode},
        middleware,
        routing::get,
    };
    use tower::ServiceExt;

    async fn ok_handler() -> &'static str {
        "ok"
    }

    #[test]
    fn test_ip_filter_service() {
        let config = IpFilterConfig::deny_by_default()
            .add_whitelist_ip("127.0.0.1".parse::<IpAddr>().unwrap())
            .build();

        assert!(config.is_allowed("127.0.0.1".parse::<IpAddr>().unwrap()));
        assert!(!config.is_allowed("192.168.1.1".parse::<IpAddr>().unwrap()));
    }

    #[test]
    fn test_ip_filter_policies() {
        let allow_config = IpFilterConfig::allow_by_default()
            .add_blacklist_ip("192.168.1.1".parse::<IpAddr>().unwrap())
            .build();

        assert!(!allow_config.is_allowed("192.168.1.1".parse::<IpAddr>().unwrap()));
        assert!(allow_config.is_allowed("192.168.1.2".parse::<IpAddr>().unwrap()));
    }

    #[test]
    fn test_network_ranges() {
        let config = IpFilterConfig::deny_by_default()
            .add_whitelist_ip("192.168.1.0/24".parse::<IpNet>().unwrap())
            .build();

        assert!(config.is_allowed("192.168.1.100".parse::<IpAddr>().unwrap()));
        assert!(!config.is_allowed("192.168.2.100".parse::<IpAddr>().unwrap()));
    }

    #[test]
    fn test_overlapping_lists_are_rejected_by_validation() {
        let config = IpFilterConfig::new()
            .add_whitelist_ip("10.0.0.0/8".parse::<IpNet>().unwrap())
            .add_blacklist_ip("10.1.0.0/16".parse::<IpNet>().unwrap());

        assert!(config.validate().is_err());
    }

    #[test]
    fn test_cache_has_capacity_guard() {
        let service = IpFilterConfig::allow_by_default()
            .max_cache_entries(2)
            .build();

        let _ = service.is_allowed("10.0.0.1".parse().unwrap());
        let _ = service.is_allowed("10.0.0.2".parse().unwrap());
        let _ = service.is_allowed("10.0.0.3".parse().unwrap());

        assert!(service.cache_size() <= 2);
    }

    #[tokio::test]
    async fn deny_by_default_blocks_when_ip_is_missing() {
        let service = IpFilterConfig::deny_by_default().build();
        let app = Router::new()
            .route("/", get(ok_handler))
            .layer(middleware::from_fn_with_state(
                service,
                ip_filter_middleware,
            ));

        let response = app
            .oneshot(
                HttpRequest::builder()
                    .uri("/")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("request should run");

        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }
}
