//! Shared lightweight transport implementation used by feature-gated transport handles.

use std::fmt;
use std::marker::PhantomData;

use crate::transport::handle::TransportHandle;
use crate::transport::types::TransportKind;

pub trait TransportMarker: Send + Sync + fmt::Debug + 'static {
    fn allocate_id() -> u64;
    fn kind() -> TransportKind;
}

#[derive(Debug)]
pub struct LightweightTransport<M: TransportMarker> {
    id: u64,
    open: bool,
    _marker: PhantomData<M>,
}

impl<M: TransportMarker> Clone for LightweightTransport<M> {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            open: self.open,
            _marker: PhantomData,
        }
    }
}

impl<M: TransportMarker> LightweightTransport<M> {
    pub fn new() -> Self {
        Self {
            id: M::allocate_id(),
            open: true,
            _marker: PhantomData,
        }
    }

    pub fn with_id(id: u64) -> Self {
        Self {
            id,
            open: true,
            _marker: PhantomData,
        }
    }

    pub fn close(&mut self) {
        self.open = false;
    }
}

impl<M: TransportMarker> Default for LightweightTransport<M> {
    fn default() -> Self {
        Self::new()
    }
}

impl<M: TransportMarker> TransportHandle for LightweightTransport<M> {
    fn id(&self) -> u64 {
        self.id
    }

    fn kind(&self) -> TransportKind {
        M::kind()
    }

    fn is_open(&self) -> bool {
        self.open
    }
}
