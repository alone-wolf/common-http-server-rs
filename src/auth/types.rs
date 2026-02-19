use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};

/// 用户信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub username: String,
    pub roles: Vec<String>,
    pub permissions: Vec<String>,
}

/// 认证用户信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthUser {
    pub user: User,
    pub auth_type: AuthType,
}

/// 认证类型
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AuthType {
    Basic,
    ApiKey,
    Jwt,
}

/// JWT Claims
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub username: String,
    pub roles: Vec<String>,
    pub iss: Option<String>,
    pub aud: Option<String>,
    pub exp: usize,
    pub iat: usize,
}

/// Basic 认证用户
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BasicUser {
    pub username: String,
    pub password_hash: String,
    pub roles: Vec<String>,
}

impl BasicUser {
    pub fn new(
        username: &str,
        password: &str,
        roles: Vec<&str>,
    ) -> Result<Self, bcrypt::BcryptError> {
        Ok(Self {
            username: username.to_string(),
            password_hash: bcrypt::hash(password, bcrypt::DEFAULT_COST)?,
            roles: roles.into_iter().map(|r| r.to_string()).collect(),
        })
    }

    pub fn verify_password(&self, password: &str) -> Result<bool, bcrypt::BcryptError> {
        bcrypt::verify(password, &self.password_hash)
    }
}

/// 认证错误
#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("Invalid credentials")]
    InvalidCredentials,
    #[error("Missing authorization header")]
    MissingAuthHeader,
    #[error("Invalid authorization header format")]
    InvalidAuthFormat,
    #[error("Token expired")]
    TokenExpired,
    #[error("Invalid token")]
    InvalidToken,
    #[error("Insufficient permissions")]
    InsufficientPermissions,
    #[error("User not found")]
    UserNotFound,
    #[error("HTTPS is required for authentication")]
    InsecureTransport,
    #[error("Insecure JWT secret configuration: {0}")]
    InsecureJwtSecret(String),
    #[error("Invalid auth configuration: {0}")]
    InvalidAuthConfig(String),
}

impl From<AuthError> for axum::http::StatusCode {
    fn from(error: AuthError) -> Self {
        match error {
            AuthError::InsufficientPermissions => axum::http::StatusCode::FORBIDDEN,
            AuthError::InsecureTransport => axum::http::StatusCode::UPGRADE_REQUIRED,
            AuthError::InsecureJwtSecret(_) => axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            AuthError::InvalidAuthConfig(_) => axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            _ => axum::http::StatusCode::UNAUTHORIZED,
        }
    }
}

impl IntoResponse for AuthError {
    fn into_response(self) -> axum::response::Response {
        let error_message = self.to_string();
        let status: axum::http::StatusCode = self.into();
        crate::core::response::ApiResponse::<()>::error_with_status(error_message, status)
            .into_response()
    }
}
