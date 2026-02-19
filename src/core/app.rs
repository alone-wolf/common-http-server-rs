//! Application router builder.
//!
//! `AppBuilder` only focuses on route composition. Runtime concerns such as
//! logging/tracing/cors layers are applied later by `Server::start`.

use crate::core::{
    health::{detailed_health_check, health_check},
    response::ApiResponse,
    server::{AppConfig, ConfigError, StartupValidation},
};
use axum::{
    Router, extract::Request, http::StatusCode, middleware::Next, response::Response, routing::get,
};

/// 应用构建器
pub struct AppBuilder {
    router: Router,
    app_config: AppConfig,
    startup_validations: Vec<StartupValidation>,
}

impl AppBuilder {
    /// 创建新的应用构建器
    pub fn new(app_config: AppConfig) -> Self {
        Self {
            router: Router::new()
                .route("/health", get(health_check))
                .route("/health/detailed", get(detailed_health_check))
                .route("/api/v1/status", get(health_check)),
            app_config,
            startup_validations: Vec::new(),
        }
    }

    /// 添加路由
    pub fn route(mut self, path: &str, method: axum::routing::MethodRouter) -> Self {
        self.router = self.router.route(path, method);
        self
    }

    /// 嵌套路由
    pub fn nest(mut self, path: &str, router: Router) -> Self {
        self.router = self.router.nest(path, router);
        self
    }

    /// Apply a prebuilt one-shot protection stack (DDoS/IP filter/rate
    /// limit/size limit) to this app in the recommended order.
    pub fn with_protection(mut self, protection: crate::protection::ProtectionStack) -> Self {
        self.router = protection.apply_to_router(self.router);
        self
    }

    /// 添加中间件
    pub fn middleware<F>(mut self, middleware: F) -> Self
    where
        F: Fn(
                Request,
                Next,
            )
                -> std::pin::Pin<Box<dyn Future<Output = Result<Response, StatusCode>> + Send>>
            + Clone
            + Send
            + Sync
            + 'static,
    {
        self.router = self.router.layer(axum::middleware::from_fn(middleware));
        self
    }

    /// Register a startup validation check that runs before binding the server
    /// socket. Use this for fail-fast checks on external/auth/protection config.
    pub fn startup_validation<F>(mut self, validation: F) -> Self
    where
        F: Fn() -> Result<(), ConfigError> + Send + Sync + 'static,
    {
        self.startup_validations.push(Box::new(validation));
        self
    }

    /// Validate shared auth config (JWT secret/expiry etc.) during startup.
    pub fn validate_auth_config(self, auth_config: crate::auth::SharedAuthConfig) -> Self {
        self.startup_validation(move || {
            auth_config
                .validate()
                .map_err(|e| ConfigError::InvalidAuth(e.to_string()))
        })
    }

    /// Validate rate-limit config during startup.
    pub fn validate_rate_limit_config(
        self,
        rate_limit_config: crate::protection::RateLimitConfig,
    ) -> Self {
        self.startup_validation(move || {
            rate_limit_config
                .validate()
                .map_err(ConfigError::InvalidProtection)
        })
    }

    /// Validate IP filter config during startup.
    pub fn validate_ip_filter_config(
        self,
        ip_filter_config: crate::protection::IpFilterConfig,
    ) -> Self {
        self.startup_validation(move || {
            ip_filter_config
                .validate()
                .map_err(ConfigError::InvalidProtection)
        })
    }

    /// Validate size-limit config during startup.
    pub fn validate_size_limit_config(
        self,
        size_limit_config: crate::protection::SizeLimitConfig,
    ) -> Self {
        self.startup_validation(move || {
            size_limit_config
                .validate()
                .map_err(ConfigError::InvalidProtection)
        })
    }

    /// Validate DDoS config during startup.
    pub fn validate_ddos_config(
        self,
        ddos_config: crate::protection::DdosProtectionConfig,
    ) -> Self {
        self.startup_validation(move || {
            ddos_config
                .validate()
                .map_err(ConfigError::InvalidProtection)
        })
    }

    /// Consume the builder and return:
    /// - the finalized router (with fallback route)
    /// - the app runtime configuration used by `Server`
    pub(crate) fn into_parts(self) -> (Router, AppConfig, Vec<StartupValidation>) {
        (
            self.router.with_state(()).fallback(fallback_handler),
            self.app_config,
            self.startup_validations,
        )
    }
}

/// 默认的 fallback 处理器
async fn fallback_handler() -> (StatusCode, ApiResponse<()>) {
    (
        StatusCode::NOT_FOUND,
        ApiResponse::error("Endpoint not found".to_string()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use tower::ServiceExt;

    async fn custom_handler() -> &'static str {
        "ok"
    }

    #[tokio::test]
    async fn keeps_custom_routes_when_building() {
        let (app, _, _) = AppBuilder::new(AppConfig::default())
            .route("/custom", get(custom_handler))
            .into_parts();

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/custom")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[test]
    fn keeps_startup_validations() {
        let (_, _, startup_validations) = AppBuilder::new(AppConfig::default())
            .startup_validation(|| Ok(()))
            .into_parts();

        assert_eq!(startup_validations.len(), 1);
    }
}
