use crate::core::response::HealthResponse;
use axum::response::IntoResponse;

/// 健康检查端点
pub async fn health_check() -> impl IntoResponse {
    tracing::info!("Health check requested");
    HealthResponse::healthy()
}

/// 详细健康检查端点（可扩展检查数据库连接等）
pub async fn detailed_health_check() -> impl IntoResponse {
    tracing::info!("Detailed health check requested");

    // 这里可以添加更多的健康检查逻辑
    // 例如：数据库连接、外部服务状态等

    HealthResponse::healthy()
}
