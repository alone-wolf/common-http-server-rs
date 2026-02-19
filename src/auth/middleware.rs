//! Authentication middleware implementations.
//!
//! Each middleware validates credentials and writes `AuthUser` into request
//! extensions for downstream handlers/guards.

use crate::AuthError;
use crate::auth::types::{AuthType, AuthUser, User};
use crate::auth::{AuthConfig, HttpsPolicy, JwtUtils};
use axum::{
    extract::{Request, State},
    http::header,
    middleware::Next,
    response::Response,
};
use base64::Engine;
use ipnet::IpNet;
use std::collections::HashSet;
use std::future::Future;
use std::net::{IpAddr, SocketAddr};
use std::pin::Pin;
use std::sync::Arc;
use tracing::info;

/// 提取Basic认证凭据
fn extract_basic_credentials(auth_str: &str) -> Result<(String, String), AuthError> {
    let encoded = auth_str.trim_start_matches("Basic ");
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(encoded.as_bytes())
        .map_err(|_| AuthError::InvalidAuthFormat)?;

    let credentials = String::from_utf8(decoded).map_err(|_| AuthError::InvalidAuthFormat)?;

    credentials
        .split_once(':')
        .map(|(u, p)| (u.to_string(), p.to_string()))
        .ok_or(AuthError::InvalidAuthFormat)
}

/// 提取Bearer token
fn extract_bearer_token(request: &Request) -> Result<String, AuthError> {
    let auth_header = request
        .headers()
        .get(header::AUTHORIZATION)
        .ok_or(AuthError::MissingAuthHeader)?;

    let auth_str = auth_header
        .to_str()
        .map_err(|_| AuthError::InvalidAuthFormat)?;

    if !auth_str.starts_with("Bearer ") {
        return Err(AuthError::InvalidAuthFormat);
    }

    Ok(auth_str.trim_start_matches("Bearer ").to_string())
}

fn is_trusted_proxy(ip: IpAddr, trusted_proxies: &[IpNet]) -> bool {
    trusted_proxies.iter().any(|net| net.contains(&ip))
}

fn is_request_secure(request: &Request, trusted_proxies: &[IpNet]) -> bool {
    if request
        .uri()
        .scheme_str()
        .is_some_and(|scheme| scheme.eq_ignore_ascii_case("https"))
    {
        return true;
    }

    let peer_ip = request
        .extensions()
        .get::<axum::extract::ConnectInfo<SocketAddr>>()
        .map(|connect_info| connect_info.0.ip());

    // Forwarded protocol headers can be spoofed by clients, so only trust them
    // when the direct peer is an explicitly trusted reverse proxy.
    let trust_forwarded_headers = peer_ip.is_some_and(|ip| is_trusted_proxy(ip, trusted_proxies));
    if !trust_forwarded_headers {
        return false;
    }

    if request
        .headers()
        .get("x-forwarded-proto")
        .and_then(|value| value.to_str().ok())
        .is_some_and(|proto| {
            proto
                .split(',')
                .any(|item| item.trim().eq_ignore_ascii_case("https"))
        })
    {
        return true;
    }

    request
        .headers()
        .get("forwarded")
        .and_then(|value| value.to_str().ok())
        .is_some_and(|forwarded| {
            forwarded.split(',').any(|part| {
                part.split(';').any(|token| {
                    token
                        .trim()
                        .strip_prefix("proto=")
                        .is_some_and(|proto| proto.trim_matches('"').eq_ignore_ascii_case("https"))
                })
            })
        })
}

fn enforce_https(config: &AuthConfig, request: &Request) -> Result<(), AuthError> {
    // Keep transport checks explicit and policy-driven so deployments behind
    // TLS-terminating proxies can opt in intentionally.
    match config.https_policy {
        HttpsPolicy::Disabled => Ok(()),
        HttpsPolicy::RequireSecureTransport => {
            if is_request_secure(request, &config.trusted_proxies) {
                Ok(())
            } else {
                Err(AuthError::InsecureTransport)
            }
        }
    }
}

fn mask_api_key_for_log(api_key: &str) -> String {
    let suffix: String = api_key
        .chars()
        .rev()
        .take(4)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    format!("***{}", suffix)
}

/// Basic 认证中间件
pub async fn basic_auth_middleware(
    State(config): State<Arc<AuthConfig>>,
    mut request: Request,
    next: Next,
) -> Result<Response, AuthError> {
    enforce_https(&config, &request)?;

    let auth_header = request
        .headers()
        .get(header::AUTHORIZATION)
        .ok_or(AuthError::MissingAuthHeader)?;

    let auth_str = auth_header
        .to_str()
        .map_err(|_| AuthError::InvalidAuthFormat)?;

    if !auth_str.starts_with("Basic ") {
        return Err(AuthError::InvalidAuthFormat);
    }

    let (username, password) = extract_basic_credentials(auth_str)?;

    let user = config
        .basic_users
        .iter()
        .find(|u| u.username == username)
        .ok_or(AuthError::InvalidCredentials)?;

    let is_valid = user
        .verify_password(&password)
        .map_err(|_| AuthError::InvalidCredentials)?;
    if !is_valid {
        return Err(AuthError::InvalidCredentials);
    }

    let auth_user = AuthUser {
        user: User {
            id: username.clone(),
            username,
            roles: user.roles.clone(),
            permissions: vec![],
        },
        auth_type: AuthType::Basic,
    };

    let username_for_log = auth_user.user.username.clone();
    request.extensions_mut().insert(auth_user);
    info!(username = %username_for_log, "Basic authentication successful");

    Ok(next.run(request).await)
}

/// API Key 认证中间件
pub async fn api_key_auth_middleware(
    State(config): State<Arc<AuthConfig>>,
    mut request: Request,
    next: Next,
) -> Result<Response, AuthError> {
    enforce_https(&config, &request)?;

    let api_key = extract_bearer_token(&request)?;

    if !config.api_keys.contains(&api_key) {
        return Err(AuthError::InvalidCredentials);
    }

    let auth_user = AuthUser {
        user: User {
            id: api_key.clone(),
            username: "api_user".to_string(),
            roles: vec!["api".to_string()],
            permissions: vec!["api_access".to_string()],
        },
        auth_type: AuthType::ApiKey,
    };

    request.extensions_mut().insert(auth_user);
    info!(api_key_suffix = %mask_api_key_for_log(&api_key), "API Key authentication successful");

    Ok(next.run(request).await)
}

/// JWT 认证中间件
pub async fn jwt_auth_middleware(
    State(config): State<Arc<AuthConfig>>,
    mut request: Request,
    next: Next,
) -> Result<Response, AuthError> {
    enforce_https(&config, &request)?;

    let token = extract_bearer_token(&request)?;
    let claims = JwtUtils::verify_token(&token, &config)?;

    let auth_user = AuthUser {
        user: User {
            id: claims.sub,
            username: claims.username.clone(),
            roles: claims.roles,
            permissions: vec![],
        },
        auth_type: AuthType::Jwt,
    };

    let username_for_log = auth_user.user.username.clone();
    request.extensions_mut().insert(auth_user);
    info!(username = %username_for_log, "JWT authentication successful");

    Ok(next.run(request).await)
}

/// 检查用户权限的通用函数
fn check_user_permissions(
    auth_user: &AuthUser,
    required: &HashSet<String>,
    field: impl Fn(&User) -> &Vec<String>,
) -> bool {
    let user_items: HashSet<String> = field(&auth_user.user).iter().cloned().collect();
    !required.is_disjoint(&user_items)
}

/// 角色检查中间件
pub fn require_roles(
    roles: Vec<&str>,
) -> impl Fn(Request, Next) -> Pin<Box<dyn Future<Output = Result<Response, AuthError>> + Send>> + Clone
{
    let required_roles: HashSet<String> = roles.into_iter().map(|r| r.to_string()).collect();

    move |request: Request, next: Next| {
        let required_roles = required_roles.clone();

        Box::pin(async move {
            let auth_user = request
                .extensions()
                .get::<AuthUser>()
                .ok_or(AuthError::MissingAuthHeader)?;

            if !check_user_permissions(auth_user, &required_roles, |user| &user.roles) {
                return Err(AuthError::InsufficientPermissions);
            }

            Ok(next.run(request).await)
        })
    }
}

/// 权限检查中间件
pub fn require_permissions(
    permissions: Vec<&str>,
) -> impl Fn(Request, Next) -> Pin<Box<dyn Future<Output = Result<Response, AuthError>> + Send>> + Clone
{
    let required_permissions: HashSet<String> =
        permissions.into_iter().map(|p| p.to_string()).collect();

    move |request: Request, next: Next| {
        let required_permissions = required_permissions.clone();

        Box::pin(async move {
            let auth_user = request
                .extensions()
                .get::<AuthUser>()
                .ok_or(AuthError::MissingAuthHeader)?;

            if !check_user_permissions(auth_user, &required_permissions, |user| &user.permissions) {
                return Err(AuthError::InsufficientPermissions);
            }

            Ok(next.run(request).await)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::presets;
    use axum::{
        Router,
        body::Body,
        http::{Request as HttpRequest, StatusCode, header::AUTHORIZATION},
        middleware,
        routing::get,
    };
    use base64::Engine;
    use ipnet::IpNet;
    use std::net::SocketAddr;
    use tower::ServiceExt;

    async fn ok_handler() -> &'static str {
        "ok"
    }

    fn setup_auth() -> Arc<AuthConfig> {
        presets::development().shared()
    }

    #[tokio::test]
    async fn basic_auth_rejects_invalid_password() {
        let config = setup_auth();
        let app = Router::new()
            .route("/", get(ok_handler))
            .layer(middleware::from_fn_with_state(
                config,
                basic_auth_middleware,
            ));

        let token = base64::engine::general_purpose::STANDARD.encode("admin:wrong-password");
        let request = HttpRequest::builder()
            .uri("/")
            .header(AUTHORIZATION, format!("Basic {token}"))
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn basic_auth_accepts_valid_password() {
        let config = setup_auth();
        let app = Router::new()
            .route("/", get(ok_handler))
            .layer(middleware::from_fn_with_state(
                config,
                basic_auth_middleware,
            ));

        let token = base64::engine::general_purpose::STANDARD.encode("admin:admin123");
        let request = HttpRequest::builder()
            .uri("/")
            .header(AUTHORIZATION, format!("Basic {token}"))
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn basic_auth_rejects_insecure_transport_when_required() {
        let mut config = presets::development();
        config.https_policy = HttpsPolicy::RequireSecureTransport;
        let app = Router::new()
            .route("/", get(ok_handler))
            .layer(middleware::from_fn_with_state(
                config.shared(),
                basic_auth_middleware,
            ));

        let token = base64::engine::general_purpose::STANDARD.encode("admin:admin123");
        let request = HttpRequest::builder()
            .uri("/")
            .header(AUTHORIZATION, format!("Basic {token}"))
            .header("x-forwarded-proto", "http")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::UPGRADE_REQUIRED);
    }

    #[tokio::test]
    async fn basic_auth_accepts_forwarded_https_from_trusted_proxy() {
        let mut config = presets::development();
        config.https_policy = HttpsPolicy::RequireSecureTransport;
        config.trusted_proxies = vec!["10.0.0.0/8".parse::<IpNet>().unwrap()];
        let app = Router::new()
            .route("/", get(ok_handler))
            .layer(middleware::from_fn_with_state(
                config.shared(),
                basic_auth_middleware,
            ));

        let token = base64::engine::general_purpose::STANDARD.encode("admin:admin123");
        let request = HttpRequest::builder()
            .uri("/")
            .header(AUTHORIZATION, format!("Basic {token}"))
            .header("x-forwarded-proto", "https")
            .extension(axum::extract::ConnectInfo(SocketAddr::from((
                [10, 1, 1, 2],
                443,
            ))))
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn jwt_auth_rejects_insecure_default_secret() {
        let mut config = presets::development();
        config.jwt_secret = "your-secret-key".to_string();
        let app = Router::new()
            .route("/", get(ok_handler))
            .layer(middleware::from_fn_with_state(
                config.shared(),
                jwt_auth_middleware,
            ));

        let request = HttpRequest::builder()
            .uri("/")
            .header(AUTHORIZATION, "Bearer dummy-token")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }
}
