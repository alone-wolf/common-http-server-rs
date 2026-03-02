use std::sync::Arc;

use crate::core::session::types::SessionId;
use crate::core::state::SessionState;
use crate::error::{FrameworkError, StateError};
use crate::protocol::handshake::CapabilityContract;
use crate::transport::handle::TransportHandle;

#[derive(Debug)]
pub struct SessionCore {
    session_id: SessionId,
    state: SessionState,
    contract: Option<CapabilityContract>,
    transport: Option<Arc<dyn TransportHandle>>,
    outbound_queue: Vec<Vec<u8>>,
}

impl SessionCore {
    pub fn new(session_id: SessionId) -> Self {
        Self {
            session_id,
            state: SessionState::Connecting,
            contract: None,
            transport: None,
            outbound_queue: Vec::new(),
        }
    }

    pub fn session_id(&self) -> SessionId {
        self.session_id
    }

    pub fn state(&self) -> SessionState {
        self.state
    }

    pub fn contract(&self) -> Option<&CapabilityContract> {
        self.contract.as_ref()
    }

    pub fn transport(&self) -> Option<&Arc<dyn TransportHandle>> {
        self.transport.as_ref()
    }

    pub fn outbound_len(&self) -> usize {
        self.outbound_queue.len()
    }

    pub fn push_outbound(&mut self, payload: Vec<u8>) {
        self.outbound_queue.push(payload);
    }

    pub fn set_contract(&mut self, contract: CapabilityContract) {
        self.contract = Some(contract);
    }

    pub fn transition_to(&mut self, next: SessionState) -> Result<(), FrameworkError> {
        if self.state.can_transition_to(next) {
            self.state = next;
            Ok(())
        } else {
            Err(FrameworkError::State(StateError::InvalidTransition {
                from: self.state,
                to: next,
            }))
        }
    }

    pub fn set_transport(&mut self, transport: Arc<dyn TransportHandle>) {
        self.transport = Some(transport);
    }

    pub fn take_transport(&mut self) -> Option<Arc<dyn TransportHandle>> {
        self.transport.take()
    }
}
