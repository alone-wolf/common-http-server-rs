use axum::{Json, Router, middleware, routing::get};
use common_http_server::{
    AppBuilder, AppConfig, AuthUser, HttpsPolicy, MonitoringState, ProtectionStackBuilder, Server,
    ServerConfig, auth_presets, basic_auth_middleware, ddos_presets, metrics_endpoint,
    monitoring_info_endpoint, performance_monitoring_middleware, rate_limit_presets, require_roles,
    setup_metrics_recorder, size_limit_presets,
};
use serde::Serialize;

#[derive(Debug, Serialize)]
struct ProfileResponse {
    username: String,
    roles: Vec<String>,
}

async fn profile(
    axum::extract::Extension(auth_user): axum::extract::Extension<AuthUser>,
) -> Json<ProfileResponse> {
    Json(ProfileResponse {
        username: auth_user.user.username,
        roles: auth_user.user.roles,
    })
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;

    runtime.block_on(async {
        let monitoring = MonitoringState::new();
        setup_metrics_recorder(monitoring.clone());

        let mut auth = auth_presets::development();
        auth.https_policy = HttpsPolicy::Disabled;
        let shared_auth = auth.shared();

        let ddos_config = ddos_presets::moderate();
        let rate_limit_config = rate_limit_presets::api();
        let size_limit_config = size_limit_presets::api();
        let protection_stack = ProtectionStackBuilder::new()
            .with_ddos(ddos_config.clone())
            .with_rate_limit(rate_limit_config.clone())
            .with_size_limit_content_length_only(size_limit_config.clone())
            .build()?;

        let secure_router = Router::new()
            .route("/profile", get(profile))
            .layer(middleware::from_fn(require_roles(vec!["admin", "user"])))
            .layer(middleware::from_fn_with_state(
                shared_auth.clone(),
                basic_auth_middleware,
            ));
        let secure_router = protection_stack.apply_to_router(secure_router);

        let monitoring_router =
            Router::new()
                .route(
                    "/metrics",
                    get({
                        let monitoring = monitoring.clone();
                        move || {
                            let monitoring = monitoring.clone();
                            async move { metrics_endpoint(axum::extract::State(monitoring)).await }
                        }
                    }),
                )
                .route(
                    "/monitoring",
                    get({
                        let monitoring = monitoring.clone();
                        move || {
                            let monitoring = monitoring.clone();
                            async move {
                                monitoring_info_endpoint(axum::extract::State(monitoring)).await
                            }
                        }
                    }),
                )
                .layer(middleware::from_fn_with_state(
                    monitoring.clone(),
                    performance_monitoring_middleware,
                ));

        let app_config = AppConfig::new().with_logging(true).with_tracing(true);
        let app_builder = AppBuilder::new(app_config)
            .validate_auth_config(shared_auth.clone())
            .validate_ddos_config(ddos_config)
            .validate_rate_limit_config(rate_limit_config)
            .validate_size_limit_config(size_limit_config)
            .nest("/secure", secure_router)
            .nest("/monitor", monitoring_router);

        let server_config = ServerConfig::new(3002).with_host("0.0.0.0");
        let server = Server::new(server_config, app_builder);
        server.start().await
    })
}
