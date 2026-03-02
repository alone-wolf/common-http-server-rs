use http_socket::core::session::core::SessionCore;
use http_socket::error::FrameworkError;
use http_socket::{SessionId, SessionState};

#[test]
fn legal_state_transition_path_works() {
    let mut session = SessionCore::new(SessionId::new(7));
    session
        .transition_to(SessionState::Negotiating)
        .expect("connecting -> negotiating should pass");
    session
        .transition_to(SessionState::Active)
        .expect("negotiating -> active should pass");
    assert_eq!(session.state(), SessionState::Active);
}

#[test]
fn illegal_state_transition_is_rejected() {
    let mut session = SessionCore::new(SessionId::new(8));
    let err = session
        .transition_to(SessionState::Active)
        .expect_err("connecting -> active should fail");
    match err {
        FrameworkError::State(_) => {}
        _ => panic!("unexpected error type"),
    }
}
