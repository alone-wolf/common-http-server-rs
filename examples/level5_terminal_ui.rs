use axum::{Json, Router, middleware, routing::get};
use common_http_server::{
    AboutInfo, ActionKind, AppBuilder, AppConfig, LogEntry, LogLevel, MonitoringState,
    RuntimeUiConfig, RuntimeUiServiceConfig, Server, ServerConfig,
    performance_monitoring_middleware, setup_metrics_recorder, start_terminal_ui_with_monitoring,
};
use serde::Serialize;
use std::time::Duration;

#[derive(Debug, Serialize)]
struct Message {
    message: &'static str,
}

async fn hello() -> Json<Message> {
    Json(Message {
        message: "hello from level5 terminal ui sample",
    })
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;

    runtime.block_on(async {
        let monitoring = MonitoringState::new();
        setup_metrics_recorder(monitoring.clone());

        let ui = start_terminal_ui_with_monitoring(
            monitoring.clone(),
            RuntimeUiServiceConfig::default()
                .with_ui_config(
                    RuntimeUiConfig::default()
                        .enabled(true)
                        .title("common-http-server terminal ui"),
                )
                .with_about(AboutInfo {
                    app_name: "common-http-server".to_string(),
                    version: env!("CARGO_PKG_VERSION").to_string(),
                    developer: "team".to_string(),
                    build_time: option_env!("BUILD_TIME").unwrap_or("dev-build").to_string(),
                    git_commit: option_env!("GIT_COMMIT").map(str::to_string),
                })
                .with_action_handler(|action| async move {
                    match action.kind {
                        ActionKind::RestartService => {
                            tracing::warn!(
                                requested_at = %action.requested_at,
                                "restart requested from terminal UI"
                            );
                        }
                        ActionKind::ShutdownService => {
                            tracing::warn!(
                                requested_at = %action.requested_at,
                                "shutdown requested from terminal UI"
                            );
                        }
                    }
                }),
        );

        let ui_log = ui.handle.clone();
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(Duration::from_secs(5));
            loop {
                ticker.tick().await;
                if ui_log
                    .send_log(LogEntry::new(
                        LogLevel::Info,
                        "runtime",
                        "heartbeat: terminal ui sample is alive",
                    ))
                    .is_err()
                {
                    break;
                }
            }
        });

        let business_router =
            Router::new()
                .route("/hello", get(hello))
                .layer(middleware::from_fn_with_state(
                    monitoring.clone(),
                    performance_monitoring_middleware,
                ));

        let app_config = AppConfig::new().with_logging(true).with_tracing(true);
        let app_builder = AppBuilder::new(app_config).nest("/api", business_router);
        let server_config = ServerConfig::new(3005).with_host("0.0.0.0");
        let server = Server::new(server_config, app_builder);
        server.start().await
    })
}
