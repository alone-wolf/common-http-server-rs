//! `common-http-server` provides a reusable Axum-based HTTP server scaffold.
//!
//! It focuses on four areas:
//! - core bootstrap (`Server`, `ServerConfig`, `AppBuilder`, `AppConfig`)
//! - authentication middleware (Basic/API Key/JWT)
//! - protection middleware (rate limit/IP filter/body limit/DDoS)
//! - monitoring (Prometheus metrics and health checks)

pub mod auth;
pub mod core;
pub mod monitoring;
pub mod protection;

// Re-export core modules
pub use core::{
    ACTION_ITEMS, AboutInfo, ActionEvent, ActionKind, ApiResponse, AppBuilder, AppConfig,
    ConfigError, CorsConfig, HealthResponse, LogEntry, LogFormat, LogLevel, LoggingConfig,
    REQUEST_ID_HEADER, RequestId, RuntimeTab, RuntimeUiActionHandler, RuntimeUiActionStream,
    RuntimeUiConfig, RuntimeUiError, RuntimeUiHandle, RuntimeUiRuntime, RuntimeUiService,
    RuntimeUiServiceConfig, Server, ServerConfig, StatusSnapshot, UiStateUpdate, cors::presets,
    current_log_filter, health_check, init_logging, spawn_runtime_ui, start_terminal_ui_simple,
    start_terminal_ui_with_monitoring, structured_logging_middleware, update_log_filter,
};

// Re-export auth modules
pub use auth::{
    AuthConfig, AuthError, AuthType, AuthUser, BasicUser, Claims, HttpsPolicy, JwtUtils,
    SharedAuthConfig, api_key_auth_middleware, basic_auth_middleware, get_auth_user,
    jwt_auth_middleware, require_permissions, require_roles, user_has_permission, user_has_role,
};

pub use auth::presets as auth_presets;

// Re-export protection modules
pub use protection::{
    DdosError, DdosMetrics, DdosProtectionConfig, DdosProtectionService, IpMetrics, ddos_presets,
    ddos_protection_middleware,
};
pub use protection::{
    DefaultPolicy, IpFilterConfig, IpFilterError, IpFilterService, ip_filter_middleware,
    ip_filter_presets,
};
pub use protection::{ProtectionStack, ProtectionStackBuilder, SizeLimitMode};
pub use protection::{
    RateLimitConfig, RateLimitError, RateLimitService, RateLimitVaryBy, rate_limit_middleware,
    rate_limit_presets,
};
pub use protection::{
    SizeLimitConfig, SizeLimitError, SizeLimitService, content_length_middleware,
    size_limit_middleware, size_limit_presets,
};

// Re-export monitoring modules
pub use monitoring::{
    HealthCheckConfig, HealthCheckResult, HealthStatus, MetricsCollector, MonitoringInfo,
    MonitoringState, RequestStats, SharedMetrics, SharedStats, enhanced_health_check,
    metrics_endpoint, monitoring_info_endpoint, performance_monitoring_middleware,
    setup_metrics_recorder,
};

/// 便捷函数：快速创建并启动服务器
pub async fn quick_start(port: u16) -> Result<(), Box<dyn std::error::Error>> {
    let server_config = ServerConfig::new(port);
    let app_builder = AppBuilder::new(AppConfig::default());
    let server = Server::new(server_config, app_builder);
    server.start().await
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
