//! Lightweight long-polling transport handle used by the session runtime.

use std::sync::atomic::{AtomicU64, Ordering};

use crate::transport::lightweight::{LightweightTransport, TransportMarker};
use crate::transport::types::TransportKind;

static POLL_TRANSPORT_ID_ALLOC: AtomicU64 = AtomicU64::new(1);

#[derive(Debug)]
pub struct PollMarker;

impl TransportMarker for PollMarker {
    fn allocate_id() -> u64 {
        POLL_TRANSPORT_ID_ALLOC.fetch_add(1, Ordering::Relaxed)
    }

    fn kind() -> TransportKind {
        TransportKind::Poll
    }
}

/// Feature-gated transport handle tagged as `poll`.
pub type PollTransport = LightweightTransport<PollMarker>;
