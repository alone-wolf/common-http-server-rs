use crate::protocol::handshake::CapabilityContract;

pub fn contracts_compatible(current: &CapabilityContract, candidate: &CapabilityContract) -> bool {
    if current.version != candidate.version {
        return false;
    }

    if current.transport != candidate.transport {
        return false;
    }

    current.enabled_capabilities == candidate.enabled_capabilities
}
