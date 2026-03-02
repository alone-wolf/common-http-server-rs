//! Lightweight WebSocket transport handle used by the session runtime.

use std::sync::atomic::{AtomicU64, Ordering};

use crate::transport::lightweight::{LightweightTransport, TransportMarker};
use crate::transport::types::TransportKind;

static WS_TRANSPORT_ID_ALLOC: AtomicU64 = AtomicU64::new(1);

#[derive(Debug)]
pub struct WsMarker;

impl TransportMarker for WsMarker {
    fn allocate_id() -> u64 {
        WS_TRANSPORT_ID_ALLOC.fetch_add(1, Ordering::Relaxed)
    }

    fn kind() -> TransportKind {
        TransportKind::Ws
    }
}

/// Feature-gated transport handle tagged as `ws`.
pub type WsTransport = LightweightTransport<WsMarker>;
