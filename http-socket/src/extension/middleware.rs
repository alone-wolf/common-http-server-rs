use crate::error::ExtensionError;

pub trait MiddlewareHook<Request>: Send + Sync {
    fn before(&self, _request: &Request) -> Result<(), ExtensionError> {
        Ok(())
    }
}

#[cfg(feature = "tower")]
pub trait TowerLayerAdapter<Service>: Send + Sync {
    type LayeredService;

    fn layer(&self, service: Service) -> Self::LayeredService;
}

#[cfg(all(feature = "tower", feature = "axum"))]
impl<Service> TowerLayerAdapter<Service> for crate::integration::axum::HttpSocketAxumLayer
where
    Service: Send + Sync,
{
    type LayeredService = crate::integration::axum::HttpSocketAxumService<Service>;

    fn layer(&self, service: Service) -> Self::LayeredService {
        tower::Layer::layer(self, service)
    }
}
