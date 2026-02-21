use axum::Router;
use common_http_server_rs::{AppBuilder, AppConfig, Server, ServerConfig, auth_presets};
use websocket::{WebSocketAuthMode, WebSocketHub, websocket_router_with_auth};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;

    runtime.block_on(async {
        let hub = WebSocketHub::new();
        let mut auth = auth_presets::development();
        auth.https_policy = common_http_server_rs::HttpsPolicy::Disabled;
        let shared_auth = auth.shared();

        let ws_router =
            websocket_router_with_auth("/ws", hub, shared_auth.clone(), WebSocketAuthMode::ApiKey);

        let app_builder = AppBuilder::new(AppConfig::new().with_logging(true).with_tracing(true))
            .validate_auth_config(shared_auth)
            .nest("/realtime", Router::new().merge(ws_router))
            .route(
                "/",
                axum::routing::get(|| async {
                    "websocket demo running, connect to /realtime/ws with Authorization: Bearer dev-api-key-1"
                }),
            );

        let server = Server::new(ServerConfig::new(3006).with_host("0.0.0.0"), app_builder);
        server.start().await
    })
}
