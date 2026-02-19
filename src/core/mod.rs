pub mod app;
pub mod client_ip;
pub mod cors;
pub mod health;
pub mod logging;
pub mod response;
pub mod runtime_ui;
pub mod server;

pub use app::AppBuilder;
pub use cors::{CorsConfig, presets};
pub use health::health_check;
pub use logging::{
    LogFormat, LoggingConfig, REQUEST_ID_HEADER, RequestId, current_log_filter, init_logging,
    structured_logging_middleware, update_log_filter,
};
pub use response::{ApiResponse, HealthResponse};
pub use runtime_ui::{
    ACTION_ITEMS, AboutInfo, ActionEvent, ActionKind, LogEntry, LogLevel, RuntimeTab,
    RuntimeUiActionHandler, RuntimeUiActionStream, RuntimeUiConfig, RuntimeUiError,
    RuntimeUiHandle, RuntimeUiRuntime, RuntimeUiService, RuntimeUiServiceConfig, StatusSnapshot,
    UiStateUpdate, spawn_runtime_ui, start_terminal_ui_simple, start_terminal_ui_with_monitoring,
};
pub use server::{AppConfig, ConfigError, Server, ServerConfig};
