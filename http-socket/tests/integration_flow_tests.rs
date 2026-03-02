use std::collections::BTreeSet;
use std::sync::Arc;

use http_socket::core::session::attach::attach_transport;
use http_socket::core::session::core::SessionCore;
use http_socket::core::session::resume::AllowAllResumeTokenValidator;
use http_socket::core::session::swap::swap_transport;
use http_socket::protocol::handshake::{
    CapabilityContract, ClientAdvertise, ServerPreferencePolicy, negotiate,
};
use http_socket::transport::handle::MockTransportHandle;
use http_socket::{
    CapabilityKey, CapabilityMap, CapabilityValue, ProtocolVersion, SessionId, SessionState,
    TransportKind,
};

#[test]
fn negotiation_attach_and_reconnect_flow() {
    let mut offered_capabilities = CapabilityMap::new();
    offered_capabilities.insert(CapabilityKey::new("resume"), CapabilityValue::Bool(true));

    let advertise = ClientAdvertise::from_required_keys(
        vec![TransportKind::Ws, TransportKind::Sse],
        vec![ProtocolVersion::new(1)],
        offered_capabilities,
        BTreeSet::new(),
    );
    let policy = ServerPreferencePolicy::new(
        vec![ProtocolVersion::new(1)],
        vec![TransportKind::Ws, TransportKind::Sse],
    );
    let selected = negotiate(&advertise, &policy).expect("negotiation should pass");
    let contract = CapabilityContract::from(selected.clone());

    let mut session = SessionCore::new(SessionId::new(401));
    attach_transport(
        &mut session,
        contract.clone(),
        Arc::new(MockTransportHandle::new(11, selected.transport)),
    )
    .expect("attach should pass");
    assert_eq!(session.state(), SessionState::Active);

    swap_transport(
        &mut session,
        contract,
        Arc::new(MockTransportHandle::new(12, selected.transport)),
        "resume-ok",
        &AllowAllResumeTokenValidator,
    )
    .expect("swap should pass");
    assert_eq!(session.state(), SessionState::Active);
    assert_eq!(
        session.transport().expect("transport should exist").id(),
        12
    );
}
