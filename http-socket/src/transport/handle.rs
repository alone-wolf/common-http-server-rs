use std::fmt;

use crate::transport::types::TransportKind;

pub trait TransportHandle: Send + Sync + fmt::Debug {
    fn id(&self) -> u64;
    fn kind(&self) -> TransportKind;
    fn is_open(&self) -> bool;
}

#[derive(Debug)]
pub struct MockTransportHandle {
    id: u64,
    kind: TransportKind,
    open: bool,
}

impl MockTransportHandle {
    pub fn new(id: u64, kind: TransportKind) -> Self {
        Self {
            id,
            kind,
            open: true,
        }
    }

    pub fn close(&mut self) {
        self.open = false;
    }
}

impl TransportHandle for MockTransportHandle {
    fn id(&self) -> u64 {
        self.id
    }

    fn kind(&self) -> TransportKind {
        self.kind
    }

    fn is_open(&self) -> bool {
        self.open
    }
}
