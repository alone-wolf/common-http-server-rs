//! Client-side capability and preference model.

use crate::protocol::capability::CapabilityMap;
use crate::protocol::version::ProtocolVersion;
use crate::transport::types::TransportKind;

#[derive(Debug, Clone)]
/// Immutable client preferences used during handshake negotiation.
pub struct Client {
    pub preferred_transports: Vec<TransportKind>,
    pub supported_versions: Vec<ProtocolVersion>,
    pub capabilities: CapabilityMap,
}
