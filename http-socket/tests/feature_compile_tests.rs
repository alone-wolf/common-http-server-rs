use http_socket::{ClientBuilder, ServerBuilder, SessionBuilder};

#[test]
fn builders_are_available_in_default_feature_set() {
    let _client = ClientBuilder::new();
    let _server = ServerBuilder::new();
    let _session = SessionBuilder::new().build();
}

#[cfg(feature = "tower")]
#[test]
fn tower_extension_boundary_compiles() {
    use http_socket::extension::middleware::TowerLayerAdapter;

    struct NoopLayer;

    impl http_socket::extension::middleware::TowerLayerAdapter<u64> for NoopLayer {
        type LayeredService = u64;

        fn layer(&self, service: u64) -> Self::LayeredService {
            service
        }
    }

    let value = NoopLayer.layer(10);
    assert_eq!(value, 10);
}

#[cfg(feature = "axum")]
#[test]
fn axum_layer_type_is_exposed() {
    let _layer = http_socket::HttpSocketAxumLayer::new();
}
