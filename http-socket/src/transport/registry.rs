use std::collections::HashMap;
use std::sync::Arc;

use crate::error::TransportError;
use crate::transport::handle::TransportHandle;
use crate::transport::types::TransportKind;

pub trait TransportFactory: Send + Sync {
    fn create(&self) -> Arc<dyn TransportHandle>;
}

#[derive(Default)]
pub struct TransportRegistry {
    factories: HashMap<TransportKind, Arc<dyn TransportFactory>>,
}

impl TransportRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, kind: TransportKind, factory: Arc<dyn TransportFactory>) {
        self.factories.insert(kind, factory);
    }

    pub fn create(&self, kind: TransportKind) -> Result<Arc<dyn TransportHandle>, TransportError> {
        let factory = self
            .factories
            .get(&kind)
            .ok_or(TransportError::UnsupportedTransport(kind.to_string()))?;

        Ok(factory.create())
    }
}
