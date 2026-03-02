use crate::core::session::types::SessionId;
use crate::error::StoreError;
use crate::protocol::handshake::CapabilityContract;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionSnapshot {
    pub session_id: SessionId,
    pub contract: CapabilityContract,
}

pub trait SessionStore: Send + Sync {
    fn save(&self, snapshot: SessionSnapshot) -> Result<(), StoreError>;
    fn load(&self, session_id: SessionId) -> Result<Option<SessionSnapshot>, StoreError>;
    fn delete(&self, session_id: SessionId) -> Result<(), StoreError>;
}
