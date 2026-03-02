#[cfg(feature = "client")]
use crate::api::client::Client;
#[cfg(feature = "server")]
use crate::api::server::Server;
use crate::core::session::core::SessionCore;
use crate::core::session::types::SessionId;
#[cfg(any(feature = "client", feature = "server"))]
use crate::error::{FrameworkError, NegotiationError};
use crate::protocol::capability::CapabilityMap;
use crate::protocol::version::ProtocolVersion;
use crate::transport::types::TransportKind;

fn default_transport_order() -> Vec<TransportKind> {
    vec![TransportKind::Ws, TransportKind::Sse, TransportKind::Poll]
}

#[cfg(any(feature = "client", feature = "server"))]
fn ensure_non_empty_transports(transports: &[TransportKind]) -> Result<(), FrameworkError> {
    if transports.is_empty() {
        Err(FrameworkError::Negotiation(
            NegotiationError::NoTransportIntersection,
        ))
    } else {
        Ok(())
    }
}

#[cfg(any(feature = "client", feature = "server"))]
fn ensure_non_empty_versions(versions: &[ProtocolVersion]) -> Result<(), FrameworkError> {
    if versions.is_empty() {
        Err(FrameworkError::Negotiation(
            NegotiationError::NoVersionIntersection,
        ))
    } else {
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct ServerBuilder {
    pub supported_transports: Vec<TransportKind>,
    pub supported_versions: Vec<ProtocolVersion>,
    pub capabilities: CapabilityMap,
}

impl Default for ServerBuilder {
    fn default() -> Self {
        Self {
            supported_transports: default_transport_order(),
            supported_versions: vec![ProtocolVersion::new(1)],
            capabilities: CapabilityMap::default(),
        }
    }
}

impl ServerBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn supported_transports(mut self, transports: Vec<TransportKind>) -> Self {
        self.supported_transports = transports;
        self
    }

    pub fn supported_versions(mut self, versions: Vec<ProtocolVersion>) -> Self {
        self.supported_versions = versions;
        self
    }

    pub fn capabilities(mut self, capabilities: CapabilityMap) -> Self {
        self.capabilities = capabilities;
        self
    }

    #[cfg(feature = "server")]
    pub fn build(self) -> Result<Server, FrameworkError> {
        ensure_non_empty_transports(&self.supported_transports)?;
        ensure_non_empty_versions(&self.supported_versions)?;

        Ok(Server {
            supported_transports: self.supported_transports,
            supported_versions: self.supported_versions,
            capabilities: self.capabilities,
        })
    }
}

#[derive(Debug, Clone)]
pub struct ClientBuilder {
    pub preferred_transports: Vec<TransportKind>,
    pub supported_versions: Vec<ProtocolVersion>,
    pub capabilities: CapabilityMap,
}

impl Default for ClientBuilder {
    fn default() -> Self {
        Self {
            preferred_transports: default_transport_order(),
            supported_versions: vec![ProtocolVersion::new(1)],
            capabilities: CapabilityMap::default(),
        }
    }
}

impl ClientBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn preferred_transports(mut self, transports: Vec<TransportKind>) -> Self {
        self.preferred_transports = transports;
        self
    }

    pub fn supported_versions(mut self, versions: Vec<ProtocolVersion>) -> Self {
        self.supported_versions = versions;
        self
    }

    pub fn capabilities(mut self, capabilities: CapabilityMap) -> Self {
        self.capabilities = capabilities;
        self
    }

    #[cfg(feature = "client")]
    pub fn build(self) -> Result<Client, FrameworkError> {
        ensure_non_empty_transports(&self.preferred_transports)?;
        ensure_non_empty_versions(&self.supported_versions)?;

        Ok(Client {
            preferred_transports: self.preferred_transports,
            supported_versions: self.supported_versions,
            capabilities: self.capabilities,
        })
    }
}

#[derive(Debug, Clone)]
pub struct SessionBuilder {
    pub session_id: Option<SessionId>,
}

impl Default for SessionBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionBuilder {
    pub fn new() -> Self {
        Self { session_id: None }
    }

    pub fn session_id(mut self, session_id: SessionId) -> Self {
        self.session_id = Some(session_id);
        self
    }

    pub fn build(self) -> SessionCore {
        SessionCore::new(self.session_id.unwrap_or_else(SessionId::next))
    }
}
