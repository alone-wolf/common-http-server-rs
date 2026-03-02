//! Lightweight Server-Sent Events transport handle used by the session runtime.

use std::sync::atomic::{AtomicU64, Ordering};

use crate::transport::lightweight::{LightweightTransport, TransportMarker};
use crate::transport::types::TransportKind;

static SSE_TRANSPORT_ID_ALLOC: AtomicU64 = AtomicU64::new(1);

#[derive(Debug)]
pub struct SseMarker;

impl TransportMarker for SseMarker {
    fn allocate_id() -> u64 {
        SSE_TRANSPORT_ID_ALLOC.fetch_add(1, Ordering::Relaxed)
    }

    fn kind() -> TransportKind {
        TransportKind::Sse
    }
}

/// Feature-gated transport handle tagged as `sse`.
pub type SseTransport = LightweightTransport<SseMarker>;
