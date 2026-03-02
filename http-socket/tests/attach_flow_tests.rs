use std::sync::Arc;

use http_socket::core::session::attach::attach_transport;
use http_socket::core::session::core::SessionCore;
use http_socket::error::{FrameworkError, StateError};
use http_socket::protocol::handshake::CapabilityContract;
use http_socket::transport::handle::MockTransportHandle;
use http_socket::{CapabilityMap, ProtocolVersion, SessionId, SessionState, TransportKind};

fn sample_contract() -> CapabilityContract {
    CapabilityContract {
        transport: TransportKind::Ws,
        version: ProtocolVersion::new(1),
        enabled_capabilities: CapabilityMap::new(),
    }
}

#[test]
fn attach_transport_activates_session() {
    let mut session = SessionCore::new(SessionId::new(101));
    let contract = sample_contract();
    let transport = Arc::new(MockTransportHandle::new(1, TransportKind::Ws));

    attach_transport(&mut session, contract, transport).expect("attach should succeed");
    assert_eq!(session.state(), SessionState::Active);
    assert!(session.transport().is_some());
    assert!(session.contract().is_some());
}

#[test]
fn attach_transport_fails_when_closed() {
    let mut session = SessionCore::new(SessionId::new(102));
    session
        .transition_to(SessionState::Closed)
        .expect("connecting -> closed should succeed");
    let contract = sample_contract();
    let transport = Arc::new(MockTransportHandle::new(2, TransportKind::Ws));

    let err = attach_transport(&mut session, contract, transport).expect_err("should fail");
    match err {
        FrameworkError::State(_) => {}
        _ => panic!("expected state error"),
    }
}

#[test]
fn attach_transport_fails_when_transport_kind_mismatches_contract() {
    let mut session = SessionCore::new(SessionId::new(103));
    let contract = sample_contract();
    let transport = Arc::new(MockTransportHandle::new(3, TransportKind::Sse));

    let err = attach_transport(&mut session, contract, transport).expect_err("should fail");
    match err {
        FrameworkError::State(StateError::TransportKindMismatch { .. }) => {}
        _ => panic!("expected transport kind mismatch"),
    }
}
