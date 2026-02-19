pub mod actions;
pub mod app;
pub mod event;
pub mod state;
pub mod ui;

pub use actions::{ACTION_ITEMS, ActionEvent, ActionKind};
pub use app::{
    RuntimeUiActionHandler, RuntimeUiActionStream, RuntimeUiConfig, RuntimeUiError,
    RuntimeUiHandle, RuntimeUiRuntime, RuntimeUiService, RuntimeUiServiceConfig, spawn_runtime_ui,
    start_terminal_ui_with_monitoring,
};
pub use state::{
    AboutInfo, AppState, LogEntry, LogLevel, RuntimeTab, StatusSnapshot, UiStateUpdate,
};

/// Start terminal UI with default about metadata.
///
/// This is a thin facade for callers that want a one-line startup API while
/// keeping the internal runtime-ui layering unchanged.
pub fn start_terminal_ui_simple(config: RuntimeUiConfig) -> RuntimeUiRuntime {
    spawn_runtime_ui(config, AboutInfo::default())
}
