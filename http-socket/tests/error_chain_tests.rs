use std::error::Error;

use http_socket::SessionState;
use http_socket::error::{FrameworkError, NegotiationError, StateError};

#[test]
fn framework_error_preserves_source() {
    let err = FrameworkError::Negotiation(NegotiationError::NoVersionIntersection);
    let source = err.source().expect("source should exist");
    assert_eq!(source.to_string(), "no protocol version intersection");
}

#[test]
fn state_error_includes_transition_context() {
    let err = FrameworkError::State(StateError::InvalidTransition {
        from: SessionState::Active,
        to: SessionState::Connecting,
    });
    assert!(err.to_string().contains("Active"));
    assert!(err.to_string().contains("Connecting"));
}
