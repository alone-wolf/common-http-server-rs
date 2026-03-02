use std::sync::Arc;

use crate::core::session::consistency::contracts_compatible;
use crate::core::session::core::SessionCore;
use crate::core::session::resume::ResumeTokenValidator;
use crate::core::state::SessionState;
use crate::error::{FrameworkError, StateError, TransportError};
use crate::protocol::handshake::CapabilityContract;
use crate::transport::handle::TransportHandle;

pub fn swap_transport(
    session: &mut SessionCore,
    new_contract: CapabilityContract,
    new_transport: Arc<dyn TransportHandle>,
    resume_token: &str,
    validator: &dyn ResumeTokenValidator,
) -> Result<(), FrameworkError> {
    let current_contract = session
        .contract()
        .ok_or(FrameworkError::State(StateError::MissingCapabilityContract))?
        .clone();

    validator
        .validate(session, resume_token)
        .map_err(FrameworkError::Auth)?;

    if !contracts_compatible(&current_contract, &new_contract) {
        return Err(FrameworkError::State(StateError::IncompatibleContract));
    }

    let actual_transport = new_transport.kind();
    if actual_transport != new_contract.transport {
        return Err(FrameworkError::State(StateError::TransportKindMismatch {
            expected: new_contract.transport,
            actual: actual_transport,
        }));
    }

    session.transition_to(SessionState::Resuming)?;

    let previous = session.take_transport();
    session.set_transport(new_transport);

    if !session
        .transport()
        .is_some_and(|transport| transport.is_open())
    {
        if let Some(prev) = previous {
            session.set_transport(prev);
        }
        session.transition_to(SessionState::Active).ok();
        return Err(FrameworkError::Transport(TransportError::TransportClosed));
    }

    let transition_result = session.transition_to(SessionState::Active);
    if transition_result.is_err() {
        if let Some(prev) = previous {
            session.set_transport(prev);
        }
        session.transition_to(SessionState::Closed).ok();
    }

    transition_result
}
