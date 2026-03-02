use std::error::Error;
use std::fmt;

use crate::core::state::SessionState;
use crate::transport::types::TransportKind;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransportError {
    UnsupportedTransport(String),
    TransportClosed,
    AttachFailed(String),
}

impl fmt::Display for TransportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedTransport(kind) => write!(f, "unsupported transport: {kind}"),
            Self::TransportClosed => write!(f, "transport is closed"),
            Self::AttachFailed(reason) => write!(f, "transport attach failed: {reason}"),
        }
    }
}

impl Error for TransportError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NegotiationError {
    NoVersionIntersection,
    NoTransportIntersection,
    MissingRequiredCapability(String),
    RequiredCapabilityNotSatisfied(String),
}

impl fmt::Display for NegotiationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoVersionIntersection => write!(f, "no protocol version intersection"),
            Self::NoTransportIntersection => write!(f, "no transport intersection"),
            Self::MissingRequiredCapability(capability) => {
                write!(f, "missing required capability: {capability}")
            }
            Self::RequiredCapabilityNotSatisfied(capability) => {
                write!(f, "required capability not satisfied: {capability}")
            }
        }
    }
}

impl Error for NegotiationError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProtocolError {
    InvalidFrame(String),
    UnsupportedCodec(String),
}

impl fmt::Display for ProtocolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidFrame(reason) => write!(f, "invalid frame: {reason}"),
            Self::UnsupportedCodec(codec) => write!(f, "unsupported codec: {codec}"),
        }
    }
}

impl Error for ProtocolError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthError {
    InvalidToken,
    AccessDenied(String),
}

impl fmt::Display for AuthError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidToken => write!(f, "invalid token"),
            Self::AccessDenied(reason) => write!(f, "access denied: {reason}"),
        }
    }
}

impl Error for AuthError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoreError {
    NotFound,
    BackendUnavailable(String),
}

impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound => write!(f, "session not found"),
            Self::BackendUnavailable(reason) => write!(f, "store backend unavailable: {reason}"),
        }
    }
}

impl Error for StoreError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExtensionError {
    MiddlewareRejected(String),
    HookFailed(String),
}

impl fmt::Display for ExtensionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MiddlewareRejected(reason) => write!(f, "middleware rejected request: {reason}"),
            Self::HookFailed(reason) => write!(f, "observability hook failed: {reason}"),
        }
    }
}

impl Error for ExtensionError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StateError {
    InvalidTransition {
        from: SessionState,
        to: SessionState,
    },
    MissingCapabilityContract,
    IncompatibleContract,
    TransportKindMismatch {
        expected: TransportKind,
        actual: TransportKind,
    },
    TransportAlreadyAttached,
    SessionClosed,
}

impl fmt::Display for StateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidTransition { from, to } => {
                write!(f, "invalid transition: {from:?} -> {to:?}")
            }
            Self::MissingCapabilityContract => write!(f, "missing capability contract"),
            Self::IncompatibleContract => write!(f, "incompatible capability contract"),
            Self::TransportKindMismatch { expected, actual } => {
                write!(
                    f,
                    "transport kind mismatch: expected {expected}, got {actual}"
                )
            }
            Self::TransportAlreadyAttached => write!(f, "transport already attached"),
            Self::SessionClosed => write!(f, "session is already closed"),
        }
    }
}

impl Error for StateError {}

#[derive(Debug)]
pub enum FrameworkError {
    Transport(TransportError),
    Negotiation(NegotiationError),
    Protocol(ProtocolError),
    Auth(AuthError),
    Store(StoreError),
    State(StateError),
    Extension(ExtensionError),
}

impl fmt::Display for FrameworkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Transport(err) => write!(f, "transport error: {err}"),
            Self::Negotiation(err) => write!(f, "negotiation error: {err}"),
            Self::Protocol(err) => write!(f, "protocol error: {err}"),
            Self::Auth(err) => write!(f, "auth error: {err}"),
            Self::Store(err) => write!(f, "store error: {err}"),
            Self::State(err) => write!(f, "state error: {err}"),
            Self::Extension(err) => write!(f, "extension error: {err}"),
        }
    }
}

impl Error for FrameworkError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Transport(err) => Some(err),
            Self::Negotiation(err) => Some(err),
            Self::Protocol(err) => Some(err),
            Self::Auth(err) => Some(err),
            Self::Store(err) => Some(err),
            Self::State(err) => Some(err),
            Self::Extension(err) => Some(err),
        }
    }
}

impl From<TransportError> for FrameworkError {
    fn from(value: TransportError) -> Self {
        Self::Transport(value)
    }
}

impl From<NegotiationError> for FrameworkError {
    fn from(value: NegotiationError) -> Self {
        Self::Negotiation(value)
    }
}

impl From<ProtocolError> for FrameworkError {
    fn from(value: ProtocolError) -> Self {
        Self::Protocol(value)
    }
}

impl From<AuthError> for FrameworkError {
    fn from(value: AuthError) -> Self {
        Self::Auth(value)
    }
}

impl From<StoreError> for FrameworkError {
    fn from(value: StoreError) -> Self {
        Self::Store(value)
    }
}

impl From<StateError> for FrameworkError {
    fn from(value: StateError) -> Self {
        Self::State(value)
    }
}

impl From<ExtensionError> for FrameworkError {
    fn from(value: ExtensionError) -> Self {
        Self::Extension(value)
    }
}
