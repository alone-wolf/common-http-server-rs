//! DDoS protection middleware and service.
//!
//! The service combines short-window burst checks and longer sustained checks,
//! plus optional auto-ban and integration with other protection modules.

use axum::{
    extract::{Request, State},
    http::{HeaderMap, HeaderValue, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use dashmap::DashMap;
use ipnet::IpNet;
use std::{
    collections::VecDeque,
    net::{IpAddr, SocketAddr},
    sync::{
        Arc,
        atomic::{AtomicU32, Ordering},
    },
    time::{Duration, Instant},
};
use tokio::time::interval;
use tracing::{debug, error, info, warn};

use super::ip_filter::IpFilterService;
use super::rate_limit::RateLimitService;
use super::size_limit::SizeLimitService;

const GLOBAL_METRICS_KEY: &str = "global";

#[derive(Debug, Clone)]
pub struct DdosProtectionConfig {
    // Rate limiting settings
    pub burst_threshold: u32,
    pub sustained_threshold: u32,
    pub burst_window: Duration,
    pub sustained_window: Duration,

    // IP tracking settings
    pub tracking_window: Duration,
    pub max_concurrent_connections: u32,

    // Auto-ban settings
    pub auto_ban_enabled: bool,
    pub auto_ban_threshold: u32,
    pub auto_ban_duration: Duration,

    // Response settings
    pub challenge_enabled: bool,
    pub slow_down_enabled: bool,
    pub slow_down_delay: Duration,

    // Monitoring settings
    pub enable_metrics: bool,
    pub log_suspicious_activity: bool,
    pub trusted_proxies: Vec<IpNet>,
}

impl Default for DdosProtectionConfig {
    fn default() -> Self {
        Self {
            burst_threshold: 100,
            sustained_threshold: 500,
            burst_window: Duration::from_secs(10),
            sustained_window: Duration::from_secs(60),
            tracking_window: Duration::from_secs(300), // 5 minutes
            max_concurrent_connections: 50,
            auto_ban_enabled: true,
            auto_ban_threshold: 1000,
            auto_ban_duration: Duration::from_secs(3600), // 1 hour
            challenge_enabled: false,
            slow_down_enabled: true,
            slow_down_delay: Duration::from_millis(100),
            enable_metrics: true,
            log_suspicious_activity: true,
            trusted_proxies: vec![],
        }
    }
}

impl DdosProtectionConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn burst_threshold(mut self, threshold: u32) -> Self {
        self.burst_threshold = threshold;
        self
    }

    pub fn sustained_threshold(mut self, threshold: u32) -> Self {
        self.sustained_threshold = threshold;
        self
    }

    pub fn auto_ban(mut self, enabled: bool, threshold: u32, duration: Duration) -> Self {
        self.auto_ban_enabled = enabled;
        self.auto_ban_threshold = threshold;
        self.auto_ban_duration = duration;
        self
    }

    pub fn challenge_enabled(mut self, enabled: bool) -> Self {
        self.challenge_enabled = enabled;
        self
    }

    pub fn slow_down(mut self, enabled: bool, delay: Duration) -> Self {
        self.slow_down_enabled = enabled;
        self.slow_down_delay = delay;
        self
    }

    pub fn max_concurrent_connections(mut self, max_connections: u32) -> Self {
        self.max_concurrent_connections = max_connections.max(1);
        self
    }

    pub fn enable_metrics(mut self, enabled: bool) -> Self {
        self.enable_metrics = enabled;
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

    pub fn build(self) -> Arc<DdosProtectionService> {
        Arc::new(DdosProtectionService::new(self))
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.burst_window.is_zero() {
            return Err("burst_window must be greater than 0".to_string());
        }
        if self.sustained_window.is_zero() {
            return Err("sustained_window must be greater than 0".to_string());
        }
        if self.tracking_window.is_zero() {
            return Err("tracking_window must be greater than 0".to_string());
        }
        if self.max_concurrent_connections == 0 {
            return Err("max_concurrent_connections must be greater than 0".to_string());
        }
        if self.auto_ban_enabled && self.auto_ban_threshold == 0 {
            return Err(
                "auto_ban_threshold must be greater than 0 when auto ban is enabled".to_string(),
            );
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct IpMetrics {
    pub request_count: u32,
    pub last_request: Instant,
    pub first_request: Instant,
    pub request_timestamps: VecDeque<Instant>,
    pub is_banned: bool,
    pub ban_expires: Option<Instant>,
    pub suspicious_score: u32,
}

impl Default for IpMetrics {
    fn default() -> Self {
        let now = Instant::now();
        Self {
            request_count: 0,
            last_request: now,
            first_request: now,
            request_timestamps: VecDeque::new(),
            is_banned: false,
            ban_expires: None,
            suspicious_score: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DdosMetrics {
    pub total_requests: u64,
    pub blocked_requests: u64,
    pub banned_ips: usize,
    pub active_connections: usize,
    pub start_time: Instant,
}

impl Default for DdosMetrics {
    fn default() -> Self {
        Self {
            total_requests: 0,
            blocked_requests: 0,
            banned_ips: 0,
            active_connections: 0,
            start_time: Instant::now(),
        }
    }
}

#[derive(Debug)]
pub struct DdosProtectionService {
    config: DdosProtectionConfig,
    ip_metrics: Arc<DashMap<IpAddr, IpMetrics>>,
    metrics: Arc<DashMap<String, DdosMetrics>>,
    active_connections: Arc<AtomicU32>,
    rate_limiter: Option<Arc<RateLimitService>>,
    ip_filter: Option<Arc<IpFilterService>>,
    size_limiter: Option<Arc<SizeLimitService>>,
}

impl DdosProtectionService {
    pub fn new(config: DdosProtectionConfig) -> Self {
        let metrics = Arc::new(DashMap::new());
        metrics.insert(GLOBAL_METRICS_KEY.to_string(), DdosMetrics::default());

        let service = Self {
            config,
            ip_metrics: Arc::new(DashMap::new()),
            metrics,
            active_connections: Arc::new(AtomicU32::new(0)),
            rate_limiter: None,
            ip_filter: None,
            size_limiter: None,
        };

        // Start cleanup task only when running inside a Tokio runtime.
        if tokio::runtime::Handle::try_current().is_ok() {
            service.start_cleanup_task();
        } else if service.config.log_suspicious_activity {
            warn!("DDoS cleanup task is disabled because no Tokio runtime is available");
        }

        service
    }

    pub fn with_rate_limiter(mut self, limiter: Arc<RateLimitService>) -> Self {
        self.rate_limiter = Some(limiter);
        self
    }

    pub fn with_ip_filter(mut self, filter: Arc<IpFilterService>) -> Self {
        self.ip_filter = Some(filter);
        self
    }

    pub fn with_size_limiter(mut self, limiter: Arc<SizeLimitService>) -> Self {
        self.size_limiter = Some(limiter);
        self
    }

    pub async fn check_request(&self, ip: IpAddr, _headers: &HeaderMap) -> Result<(), DdosError> {
        // Check if IP is banned
        if let Some(metrics) = self.ip_metrics.get(&ip)
            && metrics.is_banned
            && let Some(expires) = metrics.ban_expires
        {
            if expires > Instant::now() {
                if self.config.log_suspicious_activity {
                    warn!(
                        "Banned IP {} attempted access, ban expires in {:?}",
                        ip,
                        expires - Instant::now()
                    );
                }
                return Err(DdosError::IpBanned {
                    expires: Some(expires),
                });
            } else {
                // Ban expired, unban the IP
                self.unban_ip(ip);
            }
        }

        // Check IP filter if configured
        if let Some(ip_filter) = &self.ip_filter
            && !ip_filter.is_allowed(ip)
        {
            return Err(DdosError::IpNotAllowed);
        }

        // Update IP metrics
        self.update_ip_metrics(ip);

        // Check for DDoS patterns
        self.check_ddos_patterns(ip).await?;

        // Check rate limiter if configured
        if let Some(rate_limiter) = &self.rate_limiter {
            let key = ip.to_string();
            rate_limiter.check_rate_limit(&key)?;
        }

        Ok(())
    }

    fn increment_total_requests(&self) {
        if !self.config.enable_metrics {
            return;
        }

        if let Some(mut metrics) = self.metrics.get_mut(GLOBAL_METRICS_KEY) {
            metrics.total_requests = metrics.total_requests.saturating_add(1);
        } else {
            self.metrics.insert(
                GLOBAL_METRICS_KEY.to_string(),
                DdosMetrics {
                    total_requests: 1,
                    ..DdosMetrics::default()
                },
            );
        }
    }

    fn increment_blocked_requests(&self) {
        if !self.config.enable_metrics {
            return;
        }

        if let Some(mut metrics) = self.metrics.get_mut(GLOBAL_METRICS_KEY) {
            metrics.blocked_requests = metrics.blocked_requests.saturating_add(1);
        } else {
            self.metrics.insert(
                GLOBAL_METRICS_KEY.to_string(),
                DdosMetrics {
                    blocked_requests: 1,
                    ..DdosMetrics::default()
                },
            );
        }
    }

    fn increment_active_connections_metric(&self) {
        if !self.config.enable_metrics {
            return;
        }

        if let Some(mut metrics) = self.metrics.get_mut(GLOBAL_METRICS_KEY) {
            metrics.active_connections = metrics.active_connections.saturating_add(1);
        } else {
            self.metrics.insert(
                GLOBAL_METRICS_KEY.to_string(),
                DdosMetrics {
                    active_connections: 1,
                    ..DdosMetrics::default()
                },
            );
        }
    }

    fn decrement_active_connections_metric(&self) {
        if !self.config.enable_metrics {
            return;
        }

        if let Some(mut metrics) = self.metrics.get_mut(GLOBAL_METRICS_KEY) {
            metrics.active_connections = metrics.active_connections.saturating_sub(1);
        }
    }

    fn try_acquire_connection(&self) -> Result<(), DdosError> {
        let current = self.active_connections.fetch_add(1, Ordering::AcqRel) + 1;
        let limit = self.config.max_concurrent_connections;
        if current > limit {
            self.active_connections.fetch_sub(1, Ordering::AcqRel);
            if self.config.log_suspicious_activity {
                warn!(
                    current_connections = current,
                    connection_limit = limit,
                    "Concurrent connection limit exceeded"
                );
            }
            return Err(DdosError::TooManyConnections {
                current,
                limit,
                retry_after_seconds: retry_after_seconds_from_duration(self.config.slow_down_delay),
            });
        }

        self.increment_active_connections_metric();
        Ok(())
    }

    fn release_connection(&self) {
        let previous = self.active_connections.fetch_sub(1, Ordering::AcqRel);
        if previous == 0 {
            self.active_connections.store(0, Ordering::Release);
            return;
        }

        self.decrement_active_connections_metric();
    }

    fn update_ip_metrics(&self, ip: IpAddr) {
        let now = Instant::now();
        let max_window = self
            .config
            .tracking_window
            .max(self.config.sustained_window)
            .max(self.config.burst_window);

        let mut metrics = self.ip_metrics.entry(ip).or_default();
        metrics.request_count += 1;
        metrics.last_request = now;
        if metrics.request_count == 1 {
            metrics.first_request = now;
        }
        metrics.request_timestamps.push_back(now);

        // Keep only the timestamps needed by the longest active window to cap
        // per-IP memory usage.
        while let Some(oldest) = metrics.request_timestamps.front().copied() {
            if now.duration_since(oldest) > max_window {
                metrics.request_timestamps.pop_front();
            } else {
                break;
            }
        }
    }

    async fn check_ddos_patterns(&self, ip: IpAddr) -> Result<(), DdosError> {
        let now = Instant::now();
        let (burst_requests, sustained_requests) = if let Some(metrics) = self.ip_metrics.get(&ip) {
            let burst = metrics
                .request_timestamps
                .iter()
                .rev()
                .take_while(|timestamp| now.duration_since(**timestamp) <= self.config.burst_window)
                .count() as u32;
            let sustained = metrics
                .request_timestamps
                .iter()
                .rev()
                .take_while(|timestamp| {
                    now.duration_since(**timestamp) <= self.config.sustained_window
                })
                .count() as u32;
            (burst, sustained)
        } else {
            return Ok(());
        };

        // Check burst threshold
        if burst_requests > self.config.burst_threshold {
            if self.config.log_suspicious_activity {
                warn!(
                    "Burst threshold exceeded for IP {}: {} requests in {:?}",
                    ip, burst_requests, self.config.burst_window
                );
            }

            // Increase suspicious score
            self.increase_suspicious_score(ip, 10);

            if self.config.challenge_enabled {
                return Err(DdosError::ChallengeRequired {
                    retry_after_seconds: retry_after_seconds_from_duration(
                        self.config.slow_down_delay,
                    ),
                });
            }

            if self.config.slow_down_enabled {
                tokio::time::sleep(self.config.slow_down_delay).await;
            }
        }

        // Check sustained threshold
        if sustained_requests > self.config.sustained_threshold {
            if self.config.log_suspicious_activity {
                error!(
                    "Sustained threshold exceeded for IP {}: {} requests in {:?}",
                    ip, sustained_requests, self.config.sustained_window
                );
            }

            // Increase suspicious score significantly
            let score = self.increase_suspicious_score(ip, 50);

            // Consider auto-ban
            if self.config.auto_ban_enabled && score >= self.config.auto_ban_threshold {
                self.ban_ip(ip);
                return Err(DdosError::IpBanned {
                    expires: Some(Instant::now() + self.config.auto_ban_duration),
                });
            }

            if self.config.challenge_enabled {
                return Err(DdosError::ChallengeRequired {
                    retry_after_seconds: retry_after_seconds_from_duration(
                        self.config.sustained_window,
                    ),
                });
            }
        }

        Ok(())
    }

    fn increase_suspicious_score(&self, ip: IpAddr, amount: u32) -> u32 {
        if let Some(mut metrics) = self.ip_metrics.get_mut(&ip) {
            metrics.suspicious_score = metrics.suspicious_score.saturating_add(amount);
            metrics.suspicious_score
        } else {
            0
        }
    }

    fn ban_ip(&self, ip: IpAddr) {
        if let Some(mut metrics) = self.ip_metrics.get_mut(&ip) {
            metrics.is_banned = true;
            metrics.ban_expires = Some(Instant::now() + self.config.auto_ban_duration);
        }

        if self.config.log_suspicious_activity {
            error!(
                "IP {} automatically banned for {:?}",
                ip, self.config.auto_ban_duration
            );
        }
    }

    fn unban_ip(&self, ip: IpAddr) {
        if let Some(mut metrics) = self.ip_metrics.get_mut(&ip) {
            metrics.is_banned = false;
            metrics.ban_expires = None;
            metrics.request_count = 0;
            metrics.suspicious_score = 0;
            metrics.request_timestamps.clear();
            let now = Instant::now();
            metrics.first_request = now;
            metrics.last_request = now;
        }

        info!("IP {} unbanned", ip);
    }

    fn extract_ip(&self, headers: &HeaderMap, peer_ip: Option<IpAddr>) -> Option<IpAddr> {
        crate::core::client_ip::extract_client_ip_with_trusted_proxies(
            headers,
            peer_ip,
            &self.config.trusted_proxies,
        )
    }

    fn start_cleanup_task(&self) {
        let ip_metrics = self.ip_metrics.clone();
        let config = self.config.clone();

        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(60)); // Cleanup every minute

            loop {
                interval.tick().await;

                let now = Instant::now();
                let mut to_remove = Vec::new();

                for entry in ip_metrics.iter() {
                    let (ip, metrics) = entry.pair();

                    // Remove expired bans
                    if metrics.is_banned
                        && let Some(expires) = metrics.ban_expires
                        && expires <= now
                    {
                        to_remove.push(*ip);
                    }

                    // Remove old metrics
                    if now.duration_since(metrics.last_request) > config.tracking_window {
                        to_remove.push(*ip);
                    }
                }

                for ip in to_remove {
                    ip_metrics.remove(&ip);
                }
            }
        });
    }

    pub fn get_metrics(&self) -> DdosMetrics {
        let mut metrics = self
            .metrics
            .get(GLOBAL_METRICS_KEY)
            .map(|m| m.clone())
            .unwrap_or_default();
        metrics.active_connections = self.active_connections.load(Ordering::Acquire) as usize;
        let now = Instant::now();
        metrics.banned_ips = self
            .ip_metrics
            .iter()
            .filter(|entry| {
                let ip_metrics = entry.value();
                ip_metrics.is_banned
                    && ip_metrics
                        .ban_expires
                        .map(|expires| expires > now)
                        .unwrap_or(true)
            })
            .count();
        metrics
    }

    pub fn get_ip_metrics(&self, ip: IpAddr) -> Option<IpMetrics> {
        self.ip_metrics.get(&ip).map(|m| m.clone())
    }

    pub fn clear_metrics(&self) {
        self.ip_metrics.clear();
        self.metrics.clear();
        self.active_connections.store(0, Ordering::Release);
        self.metrics
            .insert(GLOBAL_METRICS_KEY.to_string(), DdosMetrics::default());
        info!("DDoS protection metrics cleared");
    }
}

#[derive(Debug, thiserror::Error)]
pub enum DdosError {
    #[error("IP is banned until {expires:?}")]
    IpBanned { expires: Option<Instant> },

    #[error("IP is not allowed")]
    IpNotAllowed,

    #[error("Request rate exceeded")]
    RateLimited,

    #[error("Challenge required before continuing")]
    ChallengeRequired { retry_after_seconds: u64 },

    #[error("Too many concurrent connections: {current} (limit: {limit})")]
    TooManyConnections {
        current: u32,
        limit: u32,
        retry_after_seconds: u64,
    },

    #[error("Suspicious activity detected")]
    SuspiciousActivity,

    #[error("Request too large")]
    RequestTooLarge,

    #[error("Rate limit error: {0}")]
    RateLimit(#[from] super::rate_limit::RateLimitError),
}

impl IntoResponse for DdosError {
    fn into_response(self) -> axum::response::Response {
        let retry_after_seconds = match &self {
            DdosError::ChallengeRequired {
                retry_after_seconds,
            } => Some(*retry_after_seconds),
            DdosError::TooManyConnections {
                retry_after_seconds,
                ..
            } => Some(*retry_after_seconds),
            DdosError::RateLimit(super::rate_limit::RateLimitError::RateLimited { wait_time }) => {
                Some(retry_after_seconds_from_duration(*wait_time))
            }
            _ => None,
        };

        let status = match &self {
            DdosError::IpBanned { .. } => StatusCode::FORBIDDEN,
            DdosError::IpNotAllowed => StatusCode::FORBIDDEN,
            DdosError::RateLimited => StatusCode::TOO_MANY_REQUESTS,
            DdosError::ChallengeRequired { .. } => StatusCode::TOO_MANY_REQUESTS,
            DdosError::TooManyConnections { .. } => StatusCode::TOO_MANY_REQUESTS,
            DdosError::SuspiciousActivity => StatusCode::TOO_MANY_REQUESTS,
            DdosError::RequestTooLarge => StatusCode::PAYLOAD_TOO_LARGE,
            DdosError::RateLimit(_) => StatusCode::TOO_MANY_REQUESTS,
        };

        let mut response =
            crate::core::response::ApiResponse::<()>::error_with_status(self.to_string(), status)
                .into_response();

        if let Some(retry_after_seconds) = retry_after_seconds
            && let Ok(value) = HeaderValue::from_str(&retry_after_seconds.to_string())
        {
            response.headers_mut().insert("retry-after", value);
        }

        response
    }
}

fn retry_after_seconds_from_duration(duration: Duration) -> u64 {
    let seconds = duration.as_secs();
    if duration.subsec_nanos() > 0 {
        seconds.saturating_add(1).max(1)
    } else {
        seconds.max(1)
    }
}

fn is_protection_block_status(status: StatusCode) -> bool {
    matches!(
        status,
        StatusCode::FORBIDDEN
            | StatusCode::PAYLOAD_TOO_LARGE
            | StatusCode::URI_TOO_LONG
            | StatusCode::TOO_MANY_REQUESTS
            | StatusCode::REQUEST_HEADER_FIELDS_TOO_LARGE
    )
}

pub async fn ddos_protection_middleware(
    State(service): State<Arc<DdosProtectionService>>,
    request: Request,
    next: Next,
) -> Result<Response, DdosError> {
    service.increment_total_requests();
    if let Err(err) = service.try_acquire_connection() {
        service.increment_blocked_requests();
        return Err(err);
    }

    let headers = request.headers().clone();
    let peer_ip = request
        .extensions()
        .get::<axum::extract::ConnectInfo<SocketAddr>>()
        .map(|connect_info| connect_info.0.ip());

    let result = async {
        if let Some(size_limiter) = &service.size_limiter {
            size_limiter
                .check_request_size(&request)
                .map_err(|_| DdosError::RequestTooLarge)?;
        }

        // Extract IP from headers
        let ip = service.extract_ip(&headers, peer_ip);

        if let Some(ip) = ip {
            // Check request against DDoS protection
            service.check_request(ip, &headers).await?;
        } else {
            // If we can't determine the IP, still process but log
            debug!("Could not determine client IP for DDoS protection");
        }

        Ok(next.run(request).await)
    }
    .await;

    // Always release connection accounting, even when middleware returns early.
    service.release_connection();
    match result {
        Ok(response) => {
            if is_protection_block_status(response.status()) {
                service.increment_blocked_requests();
            }
            Ok(response)
        }
        Err(err) => {
            service.increment_blocked_requests();
            Err(err)
        }
    }
}

pub mod presets {
    use super::*;

    pub fn strict() -> DdosProtectionConfig {
        DdosProtectionConfig::new()
            .burst_threshold(50)
            .sustained_threshold(200)
            .auto_ban(true, 500, Duration::from_secs(7200)) // 2 hours
            .slow_down(true, Duration::from_millis(200))
    }

    pub fn moderate() -> DdosProtectionConfig {
        DdosProtectionConfig::new()
            .burst_threshold(100)
            .sustained_threshold(500)
            .auto_ban(true, 1000, Duration::from_secs(3600)) // 1 hour
            .slow_down(true, Duration::from_millis(100))
    }

    pub fn lenient() -> DdosProtectionConfig {
        DdosProtectionConfig::new()
            .burst_threshold(500)
            .sustained_threshold(2000)
            .auto_ban(false, 0, Duration::from_secs(0))
            .slow_down(false, Duration::from_millis(0))
    }

    pub fn api_protection() -> DdosProtectionConfig {
        DdosProtectionConfig::new()
            .burst_threshold(200)
            .sustained_threshold(1000)
            .auto_ban(true, 1500, Duration::from_secs(1800)) // 30 minutes
            .slow_down(true, Duration::from_millis(50))
            .challenge_enabled(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protection::size_limit::SizeLimitConfig;
    use axum::{
        Router,
        body::Body,
        http::{Request as HttpRequest, StatusCode},
        middleware,
        routing::get,
    };
    use std::net::SocketAddr;
    use tokio::time::{Duration, sleep};
    use tower::ServiceExt;

    async fn ok_handler() -> &'static str {
        "ok"
    }

    async fn slow_handler() -> &'static str {
        sleep(Duration::from_millis(200)).await;
        "ok"
    }

    #[test]
    fn test_ddos_config() {
        let config = DdosProtectionConfig::new()
            .burst_threshold(100)
            .sustained_threshold(500);

        assert_eq!(config.burst_threshold, 100);
        assert_eq!(config.sustained_threshold, 500);
    }

    #[test]
    fn test_ddos_config_validation() {
        let mut config = DdosProtectionConfig::new();
        config.max_concurrent_connections = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_ip_metrics() {
        let metrics = IpMetrics::default();
        assert_eq!(metrics.request_count, 0);
        assert!(!metrics.is_banned);
    }

    #[test]
    fn test_ddos_service_creation_without_runtime() {
        let _ = DdosProtectionService::new(DdosProtectionConfig::new());
    }

    #[tokio::test]
    async fn test_ddos_service() {
        let config = DdosProtectionConfig::new();
        let service = DdosProtectionService::new(config);

        let ip = "127.0.0.1".parse().unwrap();
        service.update_ip_metrics(ip);

        let metrics = service.get_ip_metrics(ip);
        assert!(metrics.is_some());
        assert_eq!(metrics.unwrap().request_count, 1);
    }

    #[tokio::test]
    async fn test_ddos_middleware_updates_global_metrics() {
        let service = Arc::new(DdosProtectionService::new(DdosProtectionConfig::new()));
        let app = Router::new()
            .route("/", get(ok_handler))
            .layer(middleware::from_fn_with_state(
                service.clone(),
                ddos_protection_middleware,
            ));

        let request = HttpRequest::builder()
            .uri("/")
            .extension(axum::extract::ConnectInfo(SocketAddr::from((
                [127, 0, 0, 1],
                8080,
            ))))
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let metrics = service.get_metrics();
        assert!(metrics.total_requests >= 1);
        assert_eq!(metrics.blocked_requests, 0);
        assert_eq!(metrics.active_connections, 0);
    }

    #[tokio::test]
    async fn test_ddos_middleware_enforces_size_limiter() {
        let size_limiter = SizeLimitConfig::new(1).build();
        let service = Arc::new(
            DdosProtectionService::new(DdosProtectionConfig::new()).with_size_limiter(size_limiter),
        );
        let app = Router::new()
            .route("/", get(ok_handler))
            .layer(middleware::from_fn_with_state(
                service.clone(),
                ddos_protection_middleware,
            ));

        let request = HttpRequest::builder()
            .uri("/")
            .header("content-length", "10")
            .extension(axum::extract::ConnectInfo(SocketAddr::from((
                [127, 0, 0, 1],
                8080,
            ))))
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);

        let metrics = service.get_metrics();
        assert!(metrics.total_requests >= 1);
        assert!(metrics.blocked_requests >= 1);
    }

    #[tokio::test]
    async fn challenge_mode_returns_too_many_requests() {
        let config = DdosProtectionConfig::new()
            .burst_threshold(0)
            .challenge_enabled(true)
            .slow_down(false, Duration::from_millis(0));
        let service = Arc::new(DdosProtectionService::new(config));
        let app = Router::new()
            .route("/", get(ok_handler))
            .layer(middleware::from_fn_with_state(
                service,
                ddos_protection_middleware,
            ));

        let request = HttpRequest::builder()
            .uri("/")
            .extension(axum::extract::ConnectInfo(SocketAddr::from((
                [127, 0, 0, 1],
                8080,
            ))))
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(
            response
                .headers()
                .get("retry-after")
                .and_then(|value| value.to_str().ok()),
            Some("1")
        );
    }

    #[tokio::test]
    async fn concurrent_connection_limit_is_enforced() {
        let config = DdosProtectionConfig::new().max_concurrent_connections(1);
        let service = Arc::new(DdosProtectionService::new(config));
        let app =
            Router::new()
                .route("/", get(slow_handler))
                .layer(middleware::from_fn_with_state(
                    service,
                    ddos_protection_middleware,
                ));

        let first_request = HttpRequest::builder()
            .uri("/")
            .extension(axum::extract::ConnectInfo(SocketAddr::from((
                [127, 0, 0, 1],
                8080,
            ))))
            .body(Body::empty())
            .unwrap();
        let second_request = HttpRequest::builder()
            .uri("/")
            .extension(axum::extract::ConnectInfo(SocketAddr::from((
                [127, 0, 0, 1],
                8081,
            ))))
            .body(Body::empty())
            .unwrap();

        let first_app = app.clone();
        let first_handle = tokio::spawn(async move { first_app.oneshot(first_request).await });
        sleep(Duration::from_millis(40)).await;

        let second_response = app.oneshot(second_request).await.unwrap();
        assert_eq!(second_response.status(), StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(
            second_response
                .headers()
                .get("retry-after")
                .and_then(|value| value.to_str().ok()),
            Some("1")
        );

        let first_response = first_handle.await.unwrap().unwrap();
        assert_eq!(first_response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn metrics_can_be_disabled() {
        let config = DdosProtectionConfig::new().enable_metrics(false);
        let service = Arc::new(DdosProtectionService::new(config));
        let app = Router::new()
            .route("/", get(ok_handler))
            .layer(middleware::from_fn_with_state(
                service.clone(),
                ddos_protection_middleware,
            ));

        let request = HttpRequest::builder()
            .uri("/")
            .extension(axum::extract::ConnectInfo(SocketAddr::from((
                [127, 0, 0, 1],
                8080,
            ))))
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let metrics = service.get_metrics();
        assert_eq!(metrics.total_requests, 0);
        assert_eq!(metrics.blocked_requests, 0);
    }
}
