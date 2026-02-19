//! Rate-limiting middleware built on top of `governor`.

use axum::{
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use governor::{
    Quota, RateLimiter,
    clock::{Clock, DefaultClock},
    middleware::NoOpMiddleware,
};
use ipnet::IpNet;
use std::{
    hash::{Hash, Hasher},
    net::{IpAddr, SocketAddr},
    num::NonZeroU32,
    sync::Arc,
    time::Duration,
};
use tracing::{debug, warn};

pub type DefaultRateLimiter = RateLimiter<
    String,
    governor::state::keyed::DefaultKeyedStateStore<String>,
    DefaultClock,
    NoOpMiddleware,
>;

#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    pub max_requests: NonZeroU32,
    pub window: Duration,
    pub burst_size: Option<NonZeroU32>,
    pub vary_by: RateLimitVaryBy,
    pub trusted_proxies: Vec<IpNet>,
    pub cleanup_interval: Duration,
    pub max_tracked_keys: usize,
}

#[derive(Debug, Clone)]
pub enum RateLimitVaryBy {
    Ip,
    UserAgent,
    Custom(String),
    None,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            max_requests: NonZeroU32::new(100).unwrap(),
            window: Duration::from_secs(60),
            burst_size: None,
            vary_by: RateLimitVaryBy::Ip,
            trusted_proxies: vec![],
            cleanup_interval: Duration::from_secs(60),
            max_tracked_keys: 50_000,
        }
    }
}

impl RateLimitConfig {
    pub fn new(max_requests: u32, window_seconds: u64) -> Self {
        Self {
            max_requests: NonZeroU32::new(max_requests.max(1)).unwrap(),
            window: Duration::from_secs(window_seconds.max(1)),
            burst_size: None,
            vary_by: RateLimitVaryBy::Ip,
            trusted_proxies: vec![],
            cleanup_interval: Duration::from_secs(60),
            max_tracked_keys: 50_000,
        }
    }

    pub fn burst_size(mut self, burst_size: u32) -> Self {
        self.burst_size = Some(NonZeroU32::new(burst_size.max(1)).unwrap());
        self
    }

    pub fn vary_by_ip(mut self) -> Self {
        self.vary_by = RateLimitVaryBy::Ip;
        self
    }

    pub fn vary_by_user_agent(mut self) -> Self {
        self.vary_by = RateLimitVaryBy::UserAgent;
        self
    }

    pub fn vary_by_custom(mut self, header: impl Into<String>) -> Self {
        self.vary_by = RateLimitVaryBy::Custom(header.into());
        self
    }

    pub fn no_variation(mut self) -> Self {
        self.vary_by = RateLimitVaryBy::None;
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

    pub fn cleanup_interval(mut self, interval: Duration) -> Self {
        self.cleanup_interval = interval;
        self
    }

    pub fn max_tracked_keys(mut self, max_tracked_keys: usize) -> Self {
        self.max_tracked_keys = max_tracked_keys.max(1);
        self
    }

    pub fn build(self) -> Arc<RateLimitService> {
        Arc::new(RateLimitService::new(self))
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.window.is_zero() {
            return Err("rate-limit window must be greater than 0".to_string());
        }
        if self.cleanup_interval.is_zero() {
            return Err("rate-limit cleanup interval must be greater than 0".to_string());
        }
        if self.max_tracked_keys == 0 {
            return Err("rate-limit max_tracked_keys must be greater than 0".to_string());
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct RateLimitService {
    config: RateLimitConfig,
    limiter: Arc<DefaultRateLimiter>,
}

impl RateLimitService {
    pub fn new(config: RateLimitConfig) -> Self {
        let replenish_per_request = config.window / config.max_requests.get();
        let burst_size = config.burst_size.unwrap_or(config.max_requests);
        let quota = Quota::with_period(replenish_per_request)
            .unwrap_or_else(|| Quota::per_second(config.max_requests))
            .allow_burst(burst_size);

        let limiter = Arc::new(RateLimiter::keyed(quota));

        let service = Self { config, limiter };

        if tokio::runtime::Handle::try_current().is_ok() {
            service.start_cleanup_task();
        } else {
            warn!("Rate-limit cleanup task is disabled because no Tokio runtime is available");
        }

        service
    }

    pub fn check_rate_limit(&self, key: &str) -> Result<(), RateLimitError> {
        self.compact_limiter_if_needed();

        let key = key.to_string();
        let key_fingerprint = fingerprint_key(&key);
        let key_len = key.len();
        match self.limiter.check_key(&key) {
            Ok(_) => {
                debug!(key_fingerprint, key_len, "Rate limit check passed");
                Ok(())
            }
            Err(negative) => {
                let now = DefaultClock::default().now();
                let wait_time = negative.wait_time_from(now);
                warn!(
                    key_fingerprint,
                    key_len,
                    wait_time_ms = wait_time.as_millis() as u64,
                    "Rate limit exceeded"
                );
                Err(RateLimitError::RateLimited { wait_time })
            }
        }
    }

    fn compact_limiter_if_needed(&self) {
        let tracked_keys = self.limiter.len();
        if tracked_keys < self.config.max_tracked_keys {
            return;
        }

        self.limiter.retain_recent();
        self.limiter.shrink_to_fit();
        let remaining = self.limiter.len();

        warn!(
            tracked_keys,
            remaining,
            max_tracked_keys = self.config.max_tracked_keys,
            "Rate-limit key store reached capacity threshold and was compacted"
        );
    }

    fn start_cleanup_task(&self) {
        let limiter = self.limiter.clone();
        let cleanup_interval = self.config.cleanup_interval;

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(cleanup_interval);
            loop {
                interval.tick().await;
                limiter.retain_recent();
                limiter.shrink_to_fit();
            }
        });
    }

    fn extract_key(&self, headers: &HeaderMap, remote_addr: Option<IpAddr>) -> Option<String> {
        match &self.config.vary_by {
            RateLimitVaryBy::Ip => remote_addr.map(|addr| addr.to_string()),
            RateLimitVaryBy::UserAgent => Some(
                headers
                    .get("user-agent")
                    .and_then(|h| h.to_str().ok())
                    .unwrap_or("unknown")
                    .to_string(),
            ),
            RateLimitVaryBy::Custom(header_name) => Some(
                headers
                    .get(header_name)
                    .and_then(|h| h.to_str().ok())
                    .unwrap_or("unknown")
                    .to_string(),
            ),
            RateLimitVaryBy::None => Some("global".to_string()),
        }
    }
}

fn fingerprint_key(key: &str) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    key.hash(&mut hasher);
    hasher.finish()
}

#[derive(Debug, thiserror::Error)]
pub enum RateLimitError {
    #[error("Rate limit exceeded, wait for {wait_time:?}")]
    RateLimited { wait_time: Duration },
}

impl IntoResponse for RateLimitError {
    fn into_response(self) -> axum::response::Response {
        crate::core::response::ApiResponse::<()>::error_with_status(
            self.to_string(),
            StatusCode::TOO_MANY_REQUESTS,
        )
        .into_response()
    }
}

pub async fn rate_limit_middleware(
    State(service): State<Arc<RateLimitService>>,
    request: Request,
    next: Next,
) -> Result<Response, RateLimitError> {
    let headers = request.headers().clone();
    let peer_ip = request
        .extensions()
        .get::<axum::extract::ConnectInfo<SocketAddr>>()
        .map(|connect_info| connect_info.0.ip());
    let remote_addr = crate::core::client_ip::extract_client_ip_with_trusted_proxies(
        &headers,
        peer_ip,
        &service.config.trusted_proxies,
    );

    if let Some(key) = service.extract_key(&headers, remote_addr) {
        service.check_rate_limit(&key)?;
    } else {
        warn!(
            "Skipping IP-based rate limit because client IP could not be determined; check trusted proxy settings"
        );
    }

    Ok(next.run(request).await)
}

pub mod presets {
    use super::*;

    pub fn strict() -> RateLimitConfig {
        RateLimitConfig::new(10, 60)
    }

    pub fn moderate() -> RateLimitConfig {
        RateLimitConfig::new(100, 60)
    }

    pub fn lenient() -> RateLimitConfig {
        RateLimitConfig::new(1000, 60)
    }

    pub fn api() -> RateLimitConfig {
        RateLimitConfig::new(60, 60).burst_size(10).vary_by_ip()
    }

    pub fn web() -> RateLimitConfig {
        RateLimitConfig::new(200, 60).vary_by_ip()
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
    fn test_rate_limit_config_creation() {
        let config = RateLimitConfig::new(100, 60);
        assert_eq!(config.max_requests.get(), 100);
        assert_eq!(config.window, Duration::from_secs(60));
    }

    #[test]
    fn test_rate_limit_service() {
        let config = RateLimitConfig::new(10, 1);
        let service = RateLimitService::new(config);

        // Should pass for first few requests
        for _ in 0..5 {
            assert!(service.check_rate_limit("test_key").is_ok());
        }
    }

    #[test]
    fn test_rate_limit_is_keyed() {
        let config = RateLimitConfig::new(1, 60);
        let service = RateLimitService::new(config);

        assert!(service.check_rate_limit("client-a").is_ok());
        assert!(service.check_rate_limit("client-b").is_ok());
    }

    #[test]
    fn test_rate_limit_respects_window() {
        let config = RateLimitConfig::new(2, 2);
        let service = RateLimitService::new(config);

        assert!(service.check_rate_limit("window-key").is_ok());
        assert!(service.check_rate_limit("window-key").is_ok());

        let wait_time = match service.check_rate_limit("window-key") {
            Err(RateLimitError::RateLimited { wait_time }) => wait_time,
            Ok(_) => panic!("third request should be rate limited"),
        };
        assert!(wait_time >= Duration::from_millis(900));
    }

    #[test]
    fn test_rate_limit_config_validation() {
        let config = RateLimitConfig {
            window: Duration::ZERO,
            ..RateLimitConfig::default()
        };
        assert!(config.validate().is_err());

        let config = RateLimitConfig {
            cleanup_interval: Duration::ZERO,
            ..RateLimitConfig::default()
        };
        assert!(config.validate().is_err());
    }

    #[tokio::test]
    async fn ip_vary_mode_without_connect_info_does_not_share_unknown_bucket() {
        let service = Arc::new(RateLimitService::new(RateLimitConfig::new(1, 60)));
        let app = Router::new()
            .route("/", get(ok_handler))
            .layer(middleware::from_fn_with_state(
                service,
                rate_limit_middleware,
            ));

        let first = app
            .clone()
            .oneshot(
                HttpRequest::builder()
                    .uri("/")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("request should succeed");
        assert_eq!(first.status(), StatusCode::OK);

        let second = app
            .oneshot(
                HttpRequest::builder()
                    .uri("/")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("request should succeed");
        assert_eq!(second.status(), StatusCode::OK);
    }
}
