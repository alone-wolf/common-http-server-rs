use crate::core::session::types::SessionId;
use crate::core::state::SessionState;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionEvent {
    NegotiationStarted {
        session_id: SessionId,
    },
    NegotiationSucceeded {
        session_id: SessionId,
    },
    NegotiationFailed {
        session_id: SessionId,
        reason: String,
    },
    StateTransition {
        session_id: SessionId,
        from: SessionState,
        to: SessionState,
    },
    TransportSwapped {
        session_id: SessionId,
        transport: String,
    },
}

pub trait ObservabilityHook: Send + Sync {
    fn on_event(&self, _event: &SessionEvent) {}
    fn on_metric(&self, _name: &str, _value: u64) {}
}

#[derive(Debug, Default)]
pub struct NoopObservabilityHook;

impl ObservabilityHook for NoopObservabilityHook {}
