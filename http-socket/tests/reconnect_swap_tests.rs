use std::sync::Arc;

use http_socket::core::session::attach::attach_transport;
use http_socket::core::session::core::SessionCore;
use http_socket::core::session::resume::ResumeTokenValidator;
use http_socket::core::session::swap::swap_transport;
use http_socket::error::{AuthError, FrameworkError, StateError};
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

#[derive(Debug)]
struct RejectValidator;

impl ResumeTokenValidator for RejectValidator {
    fn validate(&self, _session: &SessionCore, _token: &str) -> Result<(), AuthError> {
        Err(AuthError::InvalidToken)
    }
}

#[derive(Debug)]
struct AllowValidator;

impl ResumeTokenValidator for AllowValidator {
    fn validate(&self, _session: &SessionCore, _token: &str) -> Result<(), AuthError> {
        Ok(())
    }
}

#[test]
fn reconnect_swap_success_keeps_session_identity() {
    let mut session = SessionCore::new(SessionId::new(201));
    let old = Arc::new(MockTransportHandle::new(1, TransportKind::Ws));
    attach_transport(&mut session, sample_contract(), old).expect("attach should pass");

    let new_transport = Arc::new(MockTransportHandle::new(2, TransportKind::Ws));
    swap_transport(
        &mut session,
        sample_contract(),
        new_transport.clone(),
        "ok",
        &AllowValidator,
    )
    .expect("swap should pass");

    assert_eq!(session.session_id(), SessionId::new(201));
    assert_eq!(session.state(), SessionState::Active);
    let active = session.transport().expect("transport should exist");
    assert_eq!(active.id(), 2);
}

#[test]
fn reconnect_swap_rejects_invalid_resume_token() {
    let mut session = SessionCore::new(SessionId::new(202));
    let old = Arc::new(MockTransportHandle::new(1, TransportKind::Ws));
    attach_transport(&mut session, sample_contract(), old).expect("attach should pass");

    let new_transport = Arc::new(MockTransportHandle::new(3, TransportKind::Ws));
    let err = swap_transport(
        &mut session,
        sample_contract(),
        new_transport,
        "bad",
        &RejectValidator,
    )
    .expect_err("invalid token should fail");
    match err {
        FrameworkError::Auth(_) => {}
        _ => panic!("expected auth error"),
    }
}

#[test]
fn reconnect_swap_rolls_back_when_new_transport_is_closed() {
    let mut session = SessionCore::new(SessionId::new(203));
    let old = Arc::new(MockTransportHandle::new(1, TransportKind::Ws));
    attach_transport(&mut session, sample_contract(), old).expect("attach should pass");

    let mut closed = MockTransportHandle::new(4, TransportKind::Ws);
    closed.close();
    let closed = Arc::new(closed);

    let err = swap_transport(
        &mut session,
        sample_contract(),
        closed,
        "ok",
        &AllowValidator,
    )
    .expect_err("closed transport should fail");
    match err {
        FrameworkError::Transport(_) => {}
        _ => panic!("expected transport error"),
    }

    let active = session
        .transport()
        .expect("previous transport should be restored");
    assert_eq!(active.id(), 1);
    assert!(active.is_open());
}

#[test]
fn reconnect_swap_rejects_transport_kind_mismatch() {
    let mut session = SessionCore::new(SessionId::new(204));
    let old = Arc::new(MockTransportHandle::new(1, TransportKind::Ws));
    attach_transport(&mut session, sample_contract(), old).expect("attach should pass");

    let new_transport = Arc::new(MockTransportHandle::new(5, TransportKind::Sse));
    let err = swap_transport(
        &mut session,
        sample_contract(),
        new_transport,
        "ok",
        &AllowValidator,
    )
    .expect_err("mismatch should fail");

    match err {
        FrameworkError::State(StateError::TransportKindMismatch { .. }) => {}
        _ => panic!("expected transport kind mismatch"),
    }

    let active = session.transport().expect("old transport should remain");
    assert_eq!(active.id(), 1);
    assert_eq!(session.state(), SessionState::Active);
}
