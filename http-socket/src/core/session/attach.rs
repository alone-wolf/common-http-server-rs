use std::sync::Arc;

use crate::core::session::core::SessionCore;
use crate::core::state::SessionState;
use crate::error::{FrameworkError, StateError};
use crate::protocol::handshake::CapabilityContract;
use crate::transport::handle::TransportHandle;

pub fn attach_transport(
    session: &mut SessionCore,
    contract: CapabilityContract,
    transport: Arc<dyn TransportHandle>,
) -> Result<(), FrameworkError> {
    if session.transport().is_some() {
        return Err(FrameworkError::State(StateError::TransportAlreadyAttached));
    }

    if session.state() == SessionState::Closed {
        return Err(FrameworkError::State(StateError::SessionClosed));
    }

    let actual_transport = transport.kind();
    if actual_transport != contract.transport {
        return Err(FrameworkError::State(StateError::TransportKindMismatch {
            expected: contract.transport,
            actual: actual_transport,
        }));
    }

    session.transition_to(SessionState::Negotiating)?;
    session.set_contract(contract);
    session.set_transport(transport);
    session.transition_to(SessionState::Active)
}
