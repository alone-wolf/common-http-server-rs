use crate::protocol::capability::{
    CapabilityMap, CapabilityRequirement, CapabilityRequirementMap, CapabilitySet,
};
use crate::protocol::version::ProtocolVersion;
use crate::transport::types::TransportKind;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientAdvertise {
    pub transports: Vec<TransportKind>,
    pub versions: Vec<ProtocolVersion>,
    pub capabilities: CapabilityMap,
    pub required_capabilities: CapabilityRequirementMap,
}

impl ClientAdvertise {
    pub fn new(
        transports: Vec<TransportKind>,
        versions: Vec<ProtocolVersion>,
        capabilities: CapabilityMap,
        required_capabilities: CapabilityRequirementMap,
    ) -> Self {
        Self {
            transports,
            versions,
            capabilities,
            required_capabilities,
        }
    }

    pub fn from_required_keys(
        transports: Vec<TransportKind>,
        versions: Vec<ProtocolVersion>,
        capabilities: CapabilityMap,
        required_capabilities: CapabilitySet,
    ) -> Self {
        let required_capabilities = required_capabilities
            .into_iter()
            .map(|key| (key, CapabilityRequirement::BoolTrue))
            .collect();
        Self::new(transports, versions, capabilities, required_capabilities)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerSelect {
    pub transport: TransportKind,
    pub version: ProtocolVersion,
    pub enabled_capabilities: CapabilityMap,
}
