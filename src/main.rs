use axum::{Json, routing::get};
use common_http_server::{AppBuilder, AppConfig, CorsConfig, Server, ServerConfig};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct TestResponse {
    message: String,
    timestamp: chrono::DateTime<chrono::Utc>,
}

async fn test_endpoint() -> impl axum::response::IntoResponse {
    tracing::info!("Test endpoint called");

    Json(TestResponse {
        message: "Hello from enhanced CORS configuration!".to_string(),
        timestamp: chrono::Utc::now(),
    })
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 创建详细的 CORS 配置
    let cors_config = CorsConfig::new()
        .allowed_origins(vec![
            "http://localhost:3000",
            "http://localhost:8080",
            "https://yourdomain.com",
        ])
        .allowed_methods(vec!["GET", "POST", "PUT", "DELETE"])
        .allowed_headers(vec![
            "Content-Type",
            "Authorization",
            "X-Requested-With",
            "X-Request-ID",
        ])
        .exposed_headers(vec!["X-Total-Count", "X-Request-ID"])
        .allow_credentials(true)
        .max_age(7200) // 2 hours
        .dev_mode(cfg!(debug_assertions));

    // 或者使用预设配置
    // let cors_config = presets::development();
    // let cors_config = presets::web_api();
    // let cors_config = presets::mobile_app();

    // 创建服务器配置
    let server_config = ServerConfig::new(3000).with_host("0.0.0.0");
    let app_config = AppConfig::new()
        .with_logging(true)
        .with_tracing(true)
        .with_cors_config(cors_config);

    // 构建应用
    let app_builder = AppBuilder::new(app_config).route("/test", get(test_endpoint));

    // 创建并启动服务器
    let server = Server::new(server_config, app_builder);
    server.start().await
}
