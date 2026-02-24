use axum::{Json, Router, routing::get};
use common_http_server_rs::{
    AppBuilder, AppConfig, GlobalAuthConfig, GlobalAuthFallback, GlobalAuthMode,
    GlobalMonitoringConfig, HttpsPolicy, MiddlewareOrchestrator, MonitoringState, PathScope,
    PerformanceMonitoringConfig, ProtectionStackBuilder, Server, ServerConfig, auth_presets,
    ddos_presets, ip_filter_presets, metrics_endpoint, monitoring_info_endpoint,
    rate_limit_presets, setup_metrics_recorder, size_limit_presets,
};
use http_panel::{HttpPanelConfig, HttpPanelState, panel_routes};
use serde::Serialize;
use websocket::{WebSocketHub, websocket_router};

#[derive(Debug, Serialize)]
struct PingResponse {
    message: &'static str,
}

async fn ping() -> Json<PingResponse> {
    Json(PingResponse { message: "pong" })
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;

    runtime.block_on(async {
        let monitoring = MonitoringState::new();
        setup_metrics_recorder(monitoring.clone());

        // API and realtime use the same auth mode (ApiKey) but different backends.
        let mut api_auth = auth_presets::development();
        api_auth.https_policy = HttpsPolicy::Disabled;
        let api_auth = api_auth.shared();

        let mut realtime_auth = auth_presets::development();
        realtime_auth.https_policy = HttpsPolicy::Disabled;
        realtime_auth.api_keys = vec!["realtime-api-key-1".to_string()];
        let realtime_auth = realtime_auth.shared();

        let websocket_hub = WebSocketHub::new();

        let ws_router = websocket_router("/ws", websocket_hub.clone());

        let panel_state = HttpPanelState::new(monitoring.clone())
            .with_websocket_hub(websocket_hub.clone())
            .with_config(
                HttpPanelConfig::new()
                    .title("Common HTTP Server Panel")
                    .refresh_interval_ms(1500),
            );
        let panel_router = panel_routes(panel_state);

        let api_router = Router::new().route("/ping", get(ping));

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
                );

        let protection_stack = ProtectionStackBuilder::new()
            .with_ddos(ddos_presets::lenient())
            .with_ip_filter(ip_filter_presets::block_known_malicious())
            .with_rate_limit(rate_limit_presets::lenient())
            .with_size_limit(size_limit_presets::api())
            .build()?;

        let auth_rules = vec![
            GlobalAuthConfig::new(api_auth.clone(), GlobalAuthMode::ApiKey)
                .with_realm("api-users")
                .with_priority(20)
                .with_scope(PathScope::all().include_prefix("/api")),
            GlobalAuthConfig::new(realtime_auth.clone(), GlobalAuthMode::ApiKey)
                .with_realm("realtime-users")
                .with_priority(20)
                .with_scope(PathScope::all().include_prefix("/realtime")),
        ];

        let app_builder = AppBuilder::new(AppConfig::new().with_logging(true).with_tracing(true))
            .validate_auth_config(api_auth)
            .validate_auth_config(realtime_auth)
            .route("/", get(|| async { "ping" }))
            .nest("/api", api_router)
            .nest("/monitor", monitoring_router)
            .nest("/panel", panel_router)
            .nest("/realtime", ws_router)
            .with_orchestrator(
                MiddlewareOrchestrator::new()
                    .with_app_runtime_layers(true)
                    .with_monitoring_config(
                        monitoring.clone(),
                        GlobalMonitoringConfig::new().with_performance_config(
                            PerformanceMonitoringConfig::new()
                                .exclude_request_count_path_prefix("/panel")
                                .exclude_request_count_path_prefix("/monitor"),
                        ),
                    )
                    .with_protection_stack(protection_stack)
                    .with_auth_fallback(GlobalAuthFallback::Allow)
                    .with_auth_rules(auth_rules),
            );

        let server_config = ServerConfig::new(3006).with_host("0.0.0.0");
        println!("[demo] server: http://127.0.0.1:3006");
        println!("[demo] panel: http://127.0.0.1:3006/panel");
        println!("[demo] api auth: Authorization: Bearer dev-api-key-1");
        println!("[demo] ws auth: Authorization: Bearer realtime-api-key-1");
        println!("[demo] ws: ws://127.0.0.1:3006/realtime/ws");

        let server = Server::new(server_config, app_builder);
        server.start().await
    })
}
