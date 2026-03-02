use std::collections::BTreeSet;

use http_socket::protocol::handshake::{CapabilityContract, ClientAdvertise, ServerSelect};
use http_socket::{
    CapabilityKey, CapabilityMap, CapabilityRequirement, CapabilityValue, ProtocolVersion,
    TransportKind,
};

#[test]
fn handshake_message_builds_with_required_fields() {
    let mut capabilities = CapabilityMap::new();
    capabilities.insert(
        CapabilityKey::new("codec.json"),
        CapabilityValue::Bool(true),
    );
    let mut required = BTreeSet::new();
    required.insert(CapabilityKey::new("codec.json"));

    let advertise = ClientAdvertise::from_required_keys(
        vec![TransportKind::Ws],
        vec![ProtocolVersion::new(1)],
        capabilities,
        required,
    );

    assert_eq!(advertise.transports, vec![TransportKind::Ws]);
    assert_eq!(advertise.versions, vec![ProtocolVersion::new(1)]);
    assert_eq!(
        advertise
            .required_capabilities
            .get(&CapabilityKey::new("codec.json")),
        Some(&CapabilityRequirement::BoolTrue)
    );
}

#[test]
fn capability_contract_can_be_derived_from_server_select() {
    let mut enabled = CapabilityMap::new();
    enabled.insert(CapabilityKey::new("resume"), CapabilityValue::Bool(true));
    let select = ServerSelect {
        transport: TransportKind::Sse,
        version: ProtocolVersion::new(1),
        enabled_capabilities: enabled.clone(),
    };

    let contract = CapabilityContract::from(select);
    assert_eq!(contract.transport, TransportKind::Sse);
    assert_eq!(contract.version, ProtocolVersion::new(1));
    assert_eq!(contract.enabled_capabilities, enabled);
}
