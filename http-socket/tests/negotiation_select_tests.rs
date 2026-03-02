use std::collections::BTreeSet;

use http_socket::error::NegotiationError;
use http_socket::protocol::handshake::{
    ClientAdvertise, ServerPreferencePolicy, negotiate, select_transport, select_version,
};
use http_socket::{CapabilityKey, CapabilityMap, CapabilityValue, ProtocolVersion, TransportKind};

#[test]
fn select_version_picks_highest_server_preference_in_intersection() {
    let selected = select_version(
        &[ProtocolVersion::new(1), ProtocolVersion::new(2)],
        &[ProtocolVersion::new(3), ProtocolVersion::new(2)],
    )
    .expect("version should be selected");
    assert_eq!(selected, ProtocolVersion::new(2));
}

#[test]
fn select_transport_returns_intersection() {
    let selected = select_transport(
        &[TransportKind::Sse, TransportKind::Poll],
        &[TransportKind::Ws, TransportKind::Poll],
    )
    .expect("transport should be selected");
    assert_eq!(selected, TransportKind::Poll);
}

#[test]
fn negotiation_fails_when_required_capability_is_missing() {
    let mut capabilities = CapabilityMap::new();
    capabilities.insert(
        CapabilityKey::new("codec.json"),
        CapabilityValue::Bool(true),
    );
    let mut required = BTreeSet::new();
    required.insert(CapabilityKey::new("codec.binary"));

    let advertise = ClientAdvertise::from_required_keys(
        vec![TransportKind::Ws],
        vec![ProtocolVersion::new(1)],
        capabilities,
        required,
    );
    let policy =
        ServerPreferencePolicy::new(vec![ProtocolVersion::new(1)], vec![TransportKind::Ws]);
    let err = negotiate(&advertise, &policy).expect_err("missing capability should fail");
    assert_eq!(
        err,
        NegotiationError::MissingRequiredCapability("codec.binary".to_string())
    );
}

#[test]
fn negotiation_fails_when_no_version_intersection() {
    let advertise = ClientAdvertise::from_required_keys(
        vec![TransportKind::Ws],
        vec![ProtocolVersion::new(1)],
        CapabilityMap::new(),
        BTreeSet::new(),
    );
    let policy =
        ServerPreferencePolicy::new(vec![ProtocolVersion::new(2)], vec![TransportKind::Ws]);
    let err = negotiate(&advertise, &policy).expect_err("version mismatch should fail");
    assert_eq!(err, NegotiationError::NoVersionIntersection);
}

#[test]
fn negotiation_fails_when_required_bool_capability_is_false() {
    let mut capabilities = CapabilityMap::new();
    capabilities.insert(CapabilityKey::new("resume"), CapabilityValue::Bool(false));
    let mut required = BTreeSet::new();
    required.insert(CapabilityKey::new("resume"));

    let advertise = ClientAdvertise::from_required_keys(
        vec![TransportKind::Ws],
        vec![ProtocolVersion::new(1)],
        capabilities,
        required,
    );
    let policy =
        ServerPreferencePolicy::new(vec![ProtocolVersion::new(1)], vec![TransportKind::Ws]);
    let err = negotiate(&advertise, &policy).expect_err("false bool should fail");
    assert_eq!(
        err,
        NegotiationError::RequiredCapabilityNotSatisfied("resume".to_string())
    );
}
