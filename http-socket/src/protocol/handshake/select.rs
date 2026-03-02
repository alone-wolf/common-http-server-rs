use crate::error::NegotiationError;
use crate::protocol::capability::{CapabilityMap, CapabilityRequirementMap};
use crate::protocol::handshake::{ClientAdvertise, NegotiationPolicy, ServerSelect};
use crate::protocol::version::ProtocolVersion;
use crate::transport::types::TransportKind;

fn select_preferred_intersection<T: Copy + PartialEq>(
    client_items: &[T],
    server_preferred_items: &[T],
) -> Option<T> {
    server_preferred_items
        .iter()
        .copied()
        .find(|item| client_items.contains(item))
}

fn ensure_required_capabilities_satisfied(
    enabled: &CapabilityMap,
    required: &CapabilityRequirementMap,
) -> Result<(), NegotiationError> {
    for (capability, requirement) in required {
        let Some(value) = enabled.get(capability) else {
            return Err(NegotiationError::MissingRequiredCapability(
                capability.to_string(),
            ));
        };
        if !requirement.is_satisfied_by(value) {
            return Err(NegotiationError::RequiredCapabilityNotSatisfied(
                capability.to_string(),
            ));
        };
    }

    Ok(())
}

pub fn negotiate(
    advertise: &ClientAdvertise,
    policy: &dyn NegotiationPolicy,
) -> Result<ServerSelect, NegotiationError> {
    let version = select_version(advertise.versions.as_slice(), policy.version_preference())?;
    let transport = select_transport(
        advertise.transports.as_slice(),
        policy.transport_preference(),
    )?;
    let enabled_capabilities = select_capabilities(
        &advertise.capabilities,
        &advertise.required_capabilities,
        policy.required_capabilities(),
        policy,
    )?;

    Ok(ServerSelect {
        transport,
        version,
        enabled_capabilities,
    })
}

pub fn select_version(
    client_versions: &[ProtocolVersion],
    server_preferred_versions: &[ProtocolVersion],
) -> Result<ProtocolVersion, NegotiationError> {
    if let Some(version) = select_preferred_intersection(client_versions, server_preferred_versions)
    {
        return Ok(version);
    }

    Err(NegotiationError::NoVersionIntersection)
}

pub fn select_transport(
    client_transports: &[TransportKind],
    server_preferred_transports: &[TransportKind],
) -> Result<TransportKind, NegotiationError> {
    if let Some(transport) =
        select_preferred_intersection(client_transports, server_preferred_transports)
    {
        return Ok(transport);
    }

    Err(NegotiationError::NoTransportIntersection)
}

pub fn select_capabilities(
    client_capabilities: &CapabilityMap,
    client_required: &CapabilityRequirementMap,
    server_required: &CapabilityRequirementMap,
    policy: &dyn NegotiationPolicy,
) -> Result<CapabilityMap, NegotiationError> {
    let enabled = policy.filter_server_capabilities(client_capabilities);

    ensure_required_capabilities_satisfied(&enabled, client_required)?;
    ensure_required_capabilities_satisfied(&enabled, server_required)?;

    Ok(enabled)
}
