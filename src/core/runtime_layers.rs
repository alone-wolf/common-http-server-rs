use crate::core::logging::structured_logging_middleware;
use crate::core::server::AppConfig;
use axum::{Router, middleware};

pub(crate) fn apply_runtime_layers(mut router: Router, config: &AppConfig) -> Router {
    // Keep middleware assembly centralized so route composition and runtime
    // concerns stay separated.
    if config.enable_logging {
        router = router.layer(middleware::from_fn(structured_logging_middleware));
    }

    if config.enable_tracing {
        router = router.layer(tower_http::trace::TraceLayer::new_for_http());
    }

    if let Some(cors_config) = config.get_cors_config() {
        router = router.layer(cors_config.build_layer());
    }

    router
}
