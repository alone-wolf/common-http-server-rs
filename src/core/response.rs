use axum::{Json, http::StatusCode, response::IntoResponse};
use serde::{Deserialize, Serialize};

/// 统一的 API 响应格式
#[derive(Debug, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
    pub request_id: Option<String>,
    pub status_code: Option<u16>,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            request_id: None,
            status_code: Some(StatusCode::OK.as_u16()),
        }
    }

    pub fn success_with_request_id(data: T, request_id: String) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            request_id: Some(request_id),
            status_code: Some(StatusCode::OK.as_u16()),
        }
    }

    pub fn error(error: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(error),
            request_id: None,
            status_code: Some(StatusCode::INTERNAL_SERVER_ERROR.as_u16()),
        }
    }

    pub fn error_with_request_id(error: String, request_id: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(error),
            request_id: Some(request_id),
            status_code: Some(StatusCode::INTERNAL_SERVER_ERROR.as_u16()),
        }
    }

    pub fn error_with_status(error: String, status: StatusCode) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(error),
            request_id: None,
            status_code: Some(status.as_u16()),
        }
    }
}

impl<T: Serialize> IntoResponse for ApiResponse<T> {
    fn into_response(self) -> axum::response::Response {
        let default_status = if self.success {
            StatusCode::OK
        } else {
            StatusCode::INTERNAL_SERVER_ERROR
        };
        let status = self
            .status_code
            .and_then(|code| StatusCode::from_u16(code).ok())
            .unwrap_or(default_status);

        (status, Json(self)).into_response()
    }
}

/// 健康检查响应
#[derive(Debug, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub message: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl HealthResponse {
    pub fn healthy() -> Self {
        Self {
            status: "ok".to_string(),
            message: "Service is running".to_string(),
            timestamp: chrono::Utc::now(),
        }
    }

    pub fn unhealthy(message: String) -> Self {
        Self {
            status: "error".to_string(),
            message,
            timestamp: chrono::Utc::now(),
        }
    }
}

impl IntoResponse for HealthResponse {
    fn into_response(self) -> axum::response::Response {
        let status = if self.status == "ok" {
            StatusCode::OK
        } else {
            StatusCode::SERVICE_UNAVAILABLE
        };

        (status, Json(self)).into_response()
    }
}
