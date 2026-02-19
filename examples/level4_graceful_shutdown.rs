use axum::{Json, extract::Path, routing::get};
use common_http_server::{AppBuilder, AppConfig, Server, ServerConfig};
use serde::Serialize;
use std::time::Duration;

#[derive(Debug, Serialize)]
struct DemoResponse {
    message: String,
}

async fn fast() -> Json<DemoResponse> {
    Json(DemoResponse {
        message: "fast response".to_string(),
    })
}

async fn slow(Path(seconds): Path<u64>) -> Json<DemoResponse> {
    let seconds = seconds.clamp(1, 30);
    tracing::info!(seconds, "slow request started");
    tokio::time::sleep(Duration::from_secs(seconds)).await;
    tracing::info!(seconds, "slow request finished");

    Json(DemoResponse {
        message: format!("slow response done in {seconds}s"),
    })
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;

    runtime.block_on(async {
        println!("Graceful shutdown demo:");
        println!("1) curl http://127.0.0.1:3003/slow/10");
        println!("2) While it is running, press Ctrl+C in this terminal");
        println!("3) Server will stop accepting new requests and wait for in-flight request");

        let app_builder = AppBuilder::new(AppConfig::new().with_logging(true).with_tracing(true))
            .route("/fast", get(fast))
            .route("/slow/{seconds}", get(slow));
        let server_config = ServerConfig::new(3003).with_host("0.0.0.0");
        let server = Server::new(server_config, app_builder);
        server.start().await
    })
}
