use crate::auth::{
    AuthConfig,
    types::{AuthError, Claims, User},
};
use chrono::{Duration, Utc};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use tracing::error;

/// JWT 工具函数
pub struct JwtUtils;

impl JwtUtils {
    /// 生成 JWT Token
    pub fn generate_token(
        user: &User,
        config: &AuthConfig,
    ) -> Result<String, jsonwebtoken::errors::Error> {
        if let Err(validation_error) = config.validate_jwt_secret() {
            error!(
                error = %validation_error,
                "Rejected JWT token generation because secret configuration is insecure"
            );
            return Err(jsonwebtoken::errors::ErrorKind::InvalidKeyFormat.into());
        }

        let now = Utc::now();
        let exp = now + Duration::hours(config.jwt_expiration_hours);

        let claims = Claims {
            sub: user.id.clone(),
            username: user.username.clone(),
            roles: user.roles.clone(),
            iss: config.jwt_issuer.clone(),
            aud: config.jwt_audience.clone(),
            exp: exp.timestamp() as usize,
            iat: now.timestamp() as usize,
        };

        let header = Header {
            alg: config.jwt_algorithm,
            ..Header::default()
        };

        encode(
            &header,
            &claims,
            &EncodingKey::from_secret(config.jwt_secret.as_ref()),
        )
    }

    /// 验证 JWT Token
    pub fn verify_token(token: &str, config: &AuthConfig) -> Result<Claims, AuthError> {
        config.validate_jwt_secret()?;

        let mut validation = Validation::new(config.jwt_algorithm);
        validation.leeway = config.jwt_leeway_seconds;
        if let Some(issuer) = &config.jwt_issuer {
            validation.set_issuer(&[issuer.as_str()]);
        }
        if let Some(audience) = &config.jwt_audience {
            validation.set_audience(&[audience.as_str()]);
        }
        let claims = decode::<Claims>(
            token,
            &DecodingKey::from_secret(config.jwt_secret.as_ref()),
            &validation,
        )
        .map_err(|e| match e.kind() {
            jsonwebtoken::errors::ErrorKind::ExpiredSignature => AuthError::TokenExpired,
            _ => AuthError::InvalidToken,
        })?
        .claims;

        Ok(claims)
    }
}
