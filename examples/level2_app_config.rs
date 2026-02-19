use axum::{Json, routing::get};
use common_http_server::{
    AppBuilder, AppConfig, CorsConfig, LogFormat, LoggingConfig, Server, ServerConfig,
};
use serde::Serialize;

#[derive(Debug, Serialize)]
struct StatusPayload {
    service: &'static str,
    version: &'static str,
}

async fn status() -> Json<StatusPayload> {
    Json(StatusPayload {
        service: "common-http-server",
        version: "level2",
    })
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;

    runtime.block_on(async {
        let cors = CorsConfig::new()
            .allowed_origins(vec!["http://localhost:3000", "http://localhost:5173"])
            .allowed_methods(vec!["GET", "POST"])
            .allowed_headers(vec!["Content-Type", "Authorization"])
            .allow_credentials(true);

        let app_config = AppConfig::new()
            .with_logging(true)
            .with_logging_config(
                LoggingConfig::default()
                    .with_format(LogFormat::Pretty)
                    .with_thread_ids(false)
                    .with_source_location(false),
            )
            .with_tracing(true)
            .with_cors_config(cors);

        let app_builder = AppBuilder::new(app_config)
            .route("/api/v1/status", get(status))
            .route("/api/v1/ping", get(|| async { "pong" }));

        let server_config = ServerConfig::new(3001).with_host("0.0.0.0");
        let server = Server::new(server_config, app_builder);
        server.start().await
    })
}
