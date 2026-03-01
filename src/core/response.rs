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
    pub fn success_with_status(data: T, status: StatusCode) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            request_id: None,
            status_code: Some(status.as_u16()),
        }
    }

    pub fn success(data: T) -> Self {
        Self::success_with_status(data, StatusCode::OK)
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

    pub fn ok(data: T) -> Self {
        Self::success_with_status(data, StatusCode::OK)
    }

    pub fn created(data: T) -> Self {
        Self::success_with_status(data, StatusCode::CREATED)
    }

    pub fn accepted(data: T) -> Self {
        Self::success_with_status(data, StatusCode::ACCEPTED)
    }
}

impl ApiResponse<()> {
    pub fn no_content() -> Self {
        Self {
            success: true,
            data: None,
            error: None,
            request_id: None,
            status_code: Some(StatusCode::NO_CONTENT.as_u16()),
        }
    }

    pub fn bad_request(error: impl Into<String>) -> Self {
        Self::error_with_status(error.into(), StatusCode::BAD_REQUEST)
    }

    pub fn unauthorized(error: impl Into<String>) -> Self {
        Self::error_with_status(error.into(), StatusCode::UNAUTHORIZED)
    }

    pub fn forbidden(error: impl Into<String>) -> Self {
        Self::error_with_status(error.into(), StatusCode::FORBIDDEN)
    }

    pub fn not_found(error: impl Into<String>) -> Self {
        Self::error_with_status(error.into(), StatusCode::NOT_FOUND)
    }

    pub fn conflict(error: impl Into<String>) -> Self {
        Self::error_with_status(error.into(), StatusCode::CONFLICT)
    }

    pub fn unprocessable_entity(error: impl Into<String>) -> Self {
        Self::error_with_status(error.into(), StatusCode::UNPROCESSABLE_ENTITY)
    }

    pub fn too_many_requests(error: impl Into<String>) -> Self {
        Self::error_with_status(error.into(), StatusCode::TOO_MANY_REQUESTS)
    }

    pub fn internal_server_error(error: impl Into<String>) -> Self {
        Self::error_with_status(error.into(), StatusCode::INTERNAL_SERVER_ERROR)
    }

    pub fn service_unavailable(error: impl Into<String>) -> Self {
        Self::error_with_status(error.into(), StatusCode::SERVICE_UNAVAILABLE)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ok_shortcut_sets_success_and_200() {
        let response = ApiResponse::ok("ok");
        assert!(response.success);
        assert_eq!(response.status_code, Some(StatusCode::OK.as_u16()));
    }

    #[test]
    fn created_shortcut_sets_201() {
        let response = ApiResponse::created("created");
        assert_eq!(response.status_code, Some(StatusCode::CREATED.as_u16()));
    }

    #[test]
    fn no_content_shortcut_sets_204_without_data() {
        let response = ApiResponse::no_content();
        assert!(response.success);
        assert!(response.data.is_none());
        assert_eq!(response.status_code, Some(StatusCode::NO_CONTENT.as_u16()));
    }

    #[test]
    fn not_found_shortcut_sets_error_and_404() {
        let response = ApiResponse::not_found("missing");
        assert!(!response.success);
        assert_eq!(response.error.as_deref(), Some("missing"));
        assert_eq!(response.status_code, Some(StatusCode::NOT_FOUND.as_u16()));
    }

    #[test]
    fn too_many_requests_shortcut_sets_429() {
        let response = ApiResponse::too_many_requests("limit exceeded");
        assert_eq!(
            response.status_code,
            Some(StatusCode::TOO_MANY_REQUESTS.as_u16())
        );
    }
}
