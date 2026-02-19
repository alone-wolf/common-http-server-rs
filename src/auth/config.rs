//! Authentication configuration models and presets.

use crate::auth::types::{AuthError, BasicUser};
use ipnet::IpNet;
use jsonwebtoken::Algorithm;
use std::sync::Arc;
use uuid::Uuid;

pub type SharedAuthConfig = Arc<AuthConfig>;
const INSECURE_JWT_SECRET_PLACEHOLDER: &str = "your-secret-key";
const MIN_JWT_SECRET_LEN: usize = 32;

/// HTTPS 传输策略
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpsPolicy {
    /// 不做 HTTPS 约束（适合在受控网络或外部网关已处理的场景）
    Disabled,
    /// 要求请求必须是安全传输（直连 HTTPS 或受信代理标记为 HTTPS）
    RequireSecureTransport,
}

/// 认证配置
#[derive(Debug, Clone)]
pub struct AuthConfig {
    pub jwt_secret: String,
    pub jwt_expiration_hours: i64,
    pub api_keys: Vec<String>,
    pub basic_users: Vec<BasicUser>,
    pub https_policy: HttpsPolicy,
    pub jwt_algorithm: Algorithm,
    pub jwt_issuer: Option<String>,
    pub jwt_audience: Option<String>,
    pub jwt_leeway_seconds: u64,
    pub trusted_proxies: Vec<IpNet>,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            jwt_secret: format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple()),
            jwt_expiration_hours: 24,
            api_keys: vec![],
            basic_users: vec![],
            https_policy: HttpsPolicy::Disabled,
            jwt_algorithm: Algorithm::HS256,
            jwt_issuer: None,
            jwt_audience: None,
            jwt_leeway_seconds: 0,
            trusted_proxies: vec![],
        }
    }
}

impl AuthConfig {
    pub fn with_jwt_secret(mut self, secret: impl Into<String>) -> Self {
        self.jwt_secret = secret.into();
        self
    }

    pub fn validate_jwt_secret(&self) -> Result<(), AuthError> {
        let secret = self.jwt_secret.trim();

        if secret.is_empty() {
            return Err(AuthError::InsecureJwtSecret(
                "JWT secret must not be empty".to_string(),
            ));
        }

        if secret == INSECURE_JWT_SECRET_PLACEHOLDER {
            return Err(AuthError::InsecureJwtSecret(
                "JWT secret is still using the insecure placeholder".to_string(),
            ));
        }

        if secret.len() < MIN_JWT_SECRET_LEN {
            return Err(AuthError::InsecureJwtSecret(format!(
                "JWT secret must be at least {} characters long",
                MIN_JWT_SECRET_LEN
            )));
        }

        Ok(())
    }

    pub fn validate(&self) -> Result<(), AuthError> {
        self.validate_jwt_secret()?;
        if self.jwt_expiration_hours <= 0 {
            return Err(AuthError::InvalidAuthConfig(
                "JWT expiration must be greater than 0 hours".to_string(),
            ));
        }
        Ok(())
    }

    pub fn with_jwt_algorithm(mut self, algorithm: Algorithm) -> Self {
        self.jwt_algorithm = algorithm;
        self
    }

    pub fn with_jwt_issuer(mut self, issuer: impl Into<String>) -> Self {
        self.jwt_issuer = Some(issuer.into());
        self
    }

    pub fn with_jwt_audience(mut self, audience: impl Into<String>) -> Self {
        self.jwt_audience = Some(audience.into());
        self
    }

    pub fn with_jwt_leeway_seconds(mut self, leeway_seconds: u64) -> Self {
        self.jwt_leeway_seconds = leeway_seconds;
        self
    }

    /// Configure how authentication middlewares treat transport security.
    pub fn with_https_policy(mut self, policy: HttpsPolicy) -> Self {
        self.https_policy = policy;
        self
    }

    /// Add one trusted reverse-proxy CIDR.
    ///
    /// Trusted proxies are only used when HTTPS policy requires secure
    /// transport and proxy headers need to be validated.
    pub fn with_trusted_proxy(mut self, proxy: IpNet) -> Self {
        self.trusted_proxies.push(proxy);
        self
    }

    /// Add multiple trusted reverse-proxy CIDRs.
    pub fn with_trusted_proxies(mut self, proxies: Vec<IpNet>) -> Self {
        self.trusted_proxies.extend(proxies);
        self
    }

    pub fn shared(self) -> SharedAuthConfig {
        Arc::new(self)
    }
}

/// 预定义的认证配置
pub mod presets {
    use super::*;

    /// 开发环境配置
    pub fn development() -> AuthConfig {
        AuthConfig {
            jwt_secret: format!("dev-{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple()),
            jwt_expiration_hours: 24,
            api_keys: vec!["dev-api-key-1".to_string(), "dev-api-key-2".to_string()],
            basic_users: vec![
                BasicUser::new("admin", "admin123", vec!["admin", "user"]).unwrap(),
                BasicUser::new("user", "user123", vec!["user"]).unwrap(),
            ],
            https_policy: HttpsPolicy::Disabled,
            jwt_algorithm: Algorithm::HS256,
            jwt_issuer: Some("common-http-server-dev".to_string()),
            jwt_audience: Some("common-http-server-clients".to_string()),
            jwt_leeway_seconds: 30,
            trusted_proxies: vec![],
        }
    }

    /// 生产环境配置
    pub fn production(jwt_secret: String) -> AuthConfig {
        AuthConfig {
            jwt_secret,
            jwt_expiration_hours: 1, // 生产环境建议较短的过期时间
            api_keys: vec![],        // 生产环境应该从安全存储加载
            basic_users: vec![],     // 生产环境应该从数据库加载
            // 生产环境常见 TLS 终止于网关/Nginx，是否强制 HTTPS 由使用方显式配置。
            https_policy: HttpsPolicy::Disabled,
            jwt_algorithm: Algorithm::HS256,
            jwt_issuer: Some("common-http-server".to_string()),
            jwt_audience: Some("common-http-server-clients".to_string()),
            jwt_leeway_seconds: 30,
            trusted_proxies: vec![],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_secret_is_not_placeholder_and_long_enough() {
        let config = AuthConfig::default();
        assert_ne!(config.jwt_secret, INSECURE_JWT_SECRET_PLACEHOLDER);
        assert!(config.jwt_secret.len() >= MIN_JWT_SECRET_LEN);
    }

    #[test]
    fn placeholder_secret_is_rejected() {
        let config = AuthConfig::default().with_jwt_secret(INSECURE_JWT_SECRET_PLACEHOLDER);
        assert!(matches!(
            config.validate_jwt_secret(),
            Err(AuthError::InsecureJwtSecret(_))
        ));
    }

    #[test]
    fn non_positive_expiration_is_rejected() {
        let config = AuthConfig {
            jwt_expiration_hours: 0,
            ..AuthConfig::default()
        };
        assert!(matches!(
            config.validate(),
            Err(AuthError::InvalidAuthConfig(_))
        ));
    }
}
