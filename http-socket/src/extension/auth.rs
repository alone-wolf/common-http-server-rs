use crate::core::session::types::SessionId;
use crate::error::AuthError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthContext {
    pub session_id: SessionId,
    pub principal: String,
    pub token: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthDecision {
    pub accepted: bool,
    pub reason: Option<String>,
}

pub trait Authenticator: Send + Sync {
    fn authenticate(&self, context: &AuthContext) -> Result<AuthDecision, AuthError>;
}

#[derive(Debug, Default)]
pub struct AllowAllAuthenticator;

impl Authenticator for AllowAllAuthenticator {
    fn authenticate(&self, _context: &AuthContext) -> Result<AuthDecision, AuthError> {
        Ok(AuthDecision {
            accepted: true,
            reason: None,
        })
    }
}
