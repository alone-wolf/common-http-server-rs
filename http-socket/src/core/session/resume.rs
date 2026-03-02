use crate::core::session::core::SessionCore;
use crate::error::AuthError;

pub trait ResumeTokenValidator: Send + Sync {
    fn validate(&self, session: &SessionCore, token: &str) -> Result<(), AuthError>;
}

#[derive(Debug, Default)]
pub struct AllowAllResumeTokenValidator;

impl ResumeTokenValidator for AllowAllResumeTokenValidator {
    fn validate(&self, _session: &SessionCore, _token: &str) -> Result<(), AuthError> {
        Ok(())
    }
}
