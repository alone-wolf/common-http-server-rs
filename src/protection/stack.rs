//! One-shot protection stack assembly helpers.

use crate::core::server::ConfigError;
use crate::protection::{
    DdosProtectionConfig, DdosProtectionService, IpFilterConfig, IpFilterService, RateLimitConfig,
    RateLimitService, SizeLimitConfig, SizeLimitService, content_length_middleware,
    ddos_protection_middleware, ip_filter_middleware, rate_limit_middleware, size_limit_middleware,
};
use axum::{Router, middleware};
use std::sync::Arc;

#[derive(Debug, Clone, Copy, Default)]
pub enum SizeLimitMode {
    #[default]
    FullBody,
    ContentLengthOnly,
}

#[derive(Debug, Clone, Default)]
pub struct ProtectionStackBuilder {
    ddos_config: Option<DdosProtectionConfig>,
    ip_filter_config: Option<IpFilterConfig>,
    rate_limit_config: Option<RateLimitConfig>,
    size_limit_config: Option<SizeLimitConfig>,
    size_limit_mode: SizeLimitMode,
}

impl ProtectionStackBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_ddos(mut self, config: DdosProtectionConfig) -> Self {
        self.ddos_config = Some(config);
        self
    }

    pub fn with_ip_filter(mut self, config: IpFilterConfig) -> Self {
        self.ip_filter_config = Some(config);
        self
    }

    pub fn with_rate_limit(mut self, config: RateLimitConfig) -> Self {
        self.rate_limit_config = Some(config);
        self
    }

    pub fn with_size_limit(mut self, config: SizeLimitConfig) -> Self {
        self.size_limit_config = Some(config);
        self.size_limit_mode = SizeLimitMode::FullBody;
        self
    }

    pub fn with_size_limit_content_length_only(mut self, config: SizeLimitConfig) -> Self {
        self.size_limit_config = Some(config);
        self.size_limit_mode = SizeLimitMode::ContentLengthOnly;
        self
    }

    pub fn build(self) -> Result<ProtectionStack, ConfigError> {
        if let Some(config) = &self.ddos_config {
            config.validate().map_err(ConfigError::InvalidProtection)?;
        }
        if let Some(config) = &self.ip_filter_config {
            config.validate().map_err(ConfigError::InvalidProtection)?;
        }
        if let Some(config) = &self.rate_limit_config {
            config.validate().map_err(ConfigError::InvalidProtection)?;
        }
        if let Some(config) = &self.size_limit_config {
            config.validate().map_err(ConfigError::InvalidProtection)?;
        }

        let rate_limit_service = self
            .rate_limit_config
            .map(|config| Arc::new(RateLimitService::new(config)));
        let ip_filter_service = self
            .ip_filter_config
            .map(|config| Arc::new(IpFilterService::new(config)));
        let size_limit_service = self
            .size_limit_config
            .map(|config| Arc::new(SizeLimitService::new(config)));

        let ddos_service = self
            .ddos_config
            .map(|config| Arc::new(DdosProtectionService::new(config)));

        Ok(ProtectionStack {
            ddos_service,
            ip_filter_service,
            rate_limit_service,
            size_limit_service,
            size_limit_mode: self.size_limit_mode,
        })
    }
}

#[derive(Debug, Clone, Default)]
pub struct ProtectionStack {
    ddos_service: Option<Arc<DdosProtectionService>>,
    ip_filter_service: Option<Arc<IpFilterService>>,
    rate_limit_service: Option<Arc<RateLimitService>>,
    size_limit_service: Option<Arc<SizeLimitService>>,
    size_limit_mode: SizeLimitMode,
}

impl ProtectionStack {
    pub fn apply_to_router(self, mut router: Router) -> Router {
        // The required runtime order is DDoS -> IP Filter -> Rate Limit -> Size
        // Limit. Because Axum layers are outermost-last, apply in reverse here.
        if let Some(size_limit_service) = self.size_limit_service {
            router = match self.size_limit_mode {
                SizeLimitMode::FullBody => router.layer(middleware::from_fn_with_state(
                    size_limit_service,
                    size_limit_middleware,
                )),
                SizeLimitMode::ContentLengthOnly => router.layer(middleware::from_fn_with_state(
                    size_limit_service,
                    content_length_middleware,
                )),
            };
        }

        if let Some(rate_limit_service) = self.rate_limit_service {
            router = router.layer(middleware::from_fn_with_state(
                rate_limit_service,
                rate_limit_middleware,
            ));
        }

        if let Some(ip_filter_service) = self.ip_filter_service {
            router = router.layer(middleware::from_fn_with_state(
                ip_filter_service,
                ip_filter_middleware,
            ));
        }

        if let Some(ddos_service) = self.ddos_service {
            router = router.layer(middleware::from_fn_with_state(
                ddos_service,
                ddos_protection_middleware,
            ));
        }

        router
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
        routing::post,
    };
    use tower::ServiceExt;

    async fn ok_handler() -> &'static str {
        "ok"
    }

    #[test]
    fn invalid_size_limit_config_fails_build() {
        let result = ProtectionStackBuilder::new()
            .with_size_limit(SizeLimitConfig::new(0))
            .build();
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn content_length_mode_enforces_payload_limit() {
        let stack = ProtectionStackBuilder::new()
            .with_size_limit_content_length_only(SizeLimitConfig::new(1))
            .build()
            .expect("stack should build");

        let app = stack.apply_to_router(Router::new().route("/", post(ok_handler)));
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/")
                    .header("content-length", "10")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
    }

    #[tokio::test]
    async fn ddos_metrics_count_rate_limited_requests_in_stack_mode() {
        let stack = ProtectionStackBuilder::new()
            .with_ddos(
                DdosProtectionConfig::new()
                    .burst_threshold(10_000)
                    .sustained_threshold(10_000)
                    .auto_ban(false, 0, std::time::Duration::from_secs(0))
                    .slow_down(false, std::time::Duration::from_millis(0)),
            )
            .with_rate_limit(RateLimitConfig::new(1, 60))
            .build()
            .expect("stack should build");
        let ddos_service = stack
            .ddos_service
            .as_ref()
            .expect("ddos service should exist")
            .clone();

        let app = stack.apply_to_router(Router::new().route("/", post(ok_handler)));
        let first_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/")
                    .extension(axum::extract::ConnectInfo(std::net::SocketAddr::from((
                        [127, 0, 0, 1],
                        3000,
                    ))))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(first_response.status(), StatusCode::OK);

        let second_response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/")
                    .extension(axum::extract::ConnectInfo(std::net::SocketAddr::from((
                        [127, 0, 0, 1],
                        3000,
                    ))))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(second_response.status(), StatusCode::TOO_MANY_REQUESTS);

        let metrics = ddos_service.get_metrics();
        assert!(metrics.blocked_requests >= 1);
    }
}
