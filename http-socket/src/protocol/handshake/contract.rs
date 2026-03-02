use crate::protocol::capability::CapabilityMap;
use crate::protocol::handshake::ServerSelect;
use crate::protocol::version::ProtocolVersion;
use crate::transport::types::TransportKind;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityContract {
    pub transport: TransportKind,
    pub version: ProtocolVersion,
    pub enabled_capabilities: CapabilityMap,
}

impl From<ServerSelect> for CapabilityContract {
    fn from(value: ServerSelect) -> Self {
        Self {
            transport: value.transport,
            version: value.version,
            enabled_capabilities: value.enabled_capabilities,
        }
    }
}
