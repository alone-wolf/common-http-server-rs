//! `http-socket` provides a capability-negotiated session layer for HTTP-oriented transports.
//!
//! Core pieces:
//! - API builders (`ClientBuilder`, `ServerBuilder`, `SessionBuilder`)
//! - handshake negotiation (`ClientAdvertise`, `ServerPreferencePolicy`, `negotiate`)
//! - session lifecycle and transport swap safety checks
//! - optional Axum middleware integration (`HttpSocketAxumLayer`)

pub mod api;
pub mod core;
pub mod error;
pub mod extension;
#[cfg(feature = "axum")]
pub mod integration;
pub mod protocol;
pub mod transport;

pub use api::builder::{ClientBuilder, ServerBuilder, SessionBuilder};
pub use core::session::types::SessionId;
pub use core::state::SessionState;
pub use error::FrameworkError;
#[cfg(feature = "axum")]
pub use integration::axum::{
    AxumMiddlewareError, AxumRequestContext, HttpSocketAxumLayer, MiddlewareRejectionMode,
    RouterHttpSocketExt,
};
pub use protocol::capability::{
    CapabilityKey, CapabilityMap, CapabilityRequirement, CapabilityRequirementMap, CapabilityValue,
};
pub use protocol::handshake::{
    CapabilityContract, ClientAdvertise, NegotiationPolicy, ServerPreferencePolicy, ServerSelect,
    negotiate,
};
pub use protocol::version::ProtocolVersion;
pub use transport::types::TransportKind;
