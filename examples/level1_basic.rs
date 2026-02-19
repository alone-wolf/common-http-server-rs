use axum::{Json, routing::get};
use common_http_server::{AppBuilder, AppConfig, Server, ServerConfig};
use serde::Serialize;

#[derive(Debug, Serialize)]
struct Message {
    message: &'static str,
}

async fn hello() -> Json<Message> {
    Json(Message {
        message: "hello from level1 sample",
    })
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;

    runtime.block_on(async {
        let server_config = ServerConfig::new(3000).with_host("0.0.0.0");
        let app_builder = AppBuilder::new(AppConfig::default()).route("/hello", get(hello));
        let server = Server::new(server_config, app_builder);
        server.start().await
    })
}
