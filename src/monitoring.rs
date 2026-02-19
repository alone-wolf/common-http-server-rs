//! Monitoring primitives:
//! - Prometheus metrics collector
//! - request performance middleware
//! - health-check utilities

use axum::body::HttpBody;
use axum::{
    Json,
    extract::{MatchedPath, Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use prometheus::{
    CounterVec, Encoder, Gauge, HistogramOpts, HistogramVec, Opts, Registry, TextEncoder,
    core::Collector,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use sysinfo::System;
use tokio::sync::RwLock;
use tracing::{debug, error};

const UNMATCHED_PATH_LABEL: &str = "__unmatched__";
const MAX_EXTERNAL_SERVICE_CHECKS: usize = 8;
#[cfg(feature = "external-health")]
const EXTERNAL_HEALTH_CHECK_TIMEOUT: Duration = Duration::from_secs(3);
const EXTERNAL_HEALTH_DNS_TIMEOUT: Duration = Duration::from_secs(2);
#[cfg(feature = "database-health")]
const HEALTH_DB_QUERY_TIMEOUT: Duration = Duration::from_secs(3);
const ALLOW_RUNTIME_HEALTH_TARGETS_ENV: &str = "COMMON_HTTP_SERVER_ALLOW_RUNTIME_HEALTH_TARGETS";

#[derive(Debug, Clone)]
pub struct MetricsCollector {
    registry: Registry,
    http_requests_total: Option<CounterVec>,
    http_request_duration_seconds: Option<HistogramVec>,
    http_response_size_bytes: Option<HistogramVec>,
    active_connections: Option<Gauge>,
    system_cpu_usage: Option<Gauge>,
    system_memory_usage: Option<Gauge>,
    request_rate: Option<Gauge>,
    error_rate: Option<Gauge>,
}

impl MetricsCollector {
    pub fn new() -> Self {
        let registry = Registry::new();

        let http_requests_total = register_metric(
            &registry,
            "http_requests_total",
            CounterVec::new(
                Opts::new("http_requests_total", "Total number of HTTP requests"),
                &["method", "path", "status_code"],
            ),
        );

        let http_request_duration_seconds = register_metric(
            &registry,
            "http_request_duration_seconds",
            HistogramVec::new(
                HistogramOpts::new(
                    "http_request_duration_seconds",
                    "HTTP request duration in seconds",
                )
                .buckets(vec![
                    0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
                ]),
                &["method", "path", "status_code"],
            ),
        );

        let http_response_size_bytes = register_metric(
            &registry,
            "http_response_size_bytes",
            HistogramVec::new(
                HistogramOpts::new("http_response_size_bytes", "HTTP response size in bytes")
                    .buckets(vec![
                        100.0, 500.0, 1000.0, 5000.0, 10000.0, 50000.0, 100000.0, 500000.0,
                        1000000.0,
                    ]),
                &["method", "path", "status_code"],
            ),
        );

        let active_connections = register_metric(
            &registry,
            "active_connections",
            Gauge::new("active_connections", "Number of active connections"),
        );
        let system_cpu_usage = register_metric(
            &registry,
            "system_cpu_usage_percent",
            Gauge::new("system_cpu_usage_percent", "System CPU usage percentage"),
        );
        let system_memory_usage = register_metric(
            &registry,
            "system_memory_usage_percent",
            Gauge::new(
                "system_memory_usage_percent",
                "System memory usage percentage",
            ),
        );
        let request_rate = register_metric(
            &registry,
            "requests_per_second",
            Gauge::new("requests_per_second", "Requests per second"),
        );
        let error_rate = register_metric(
            &registry,
            "error_rate_percent",
            Gauge::new("error_rate_percent", "Error rate percentage"),
        );

        Self {
            registry,
            http_requests_total,
            http_request_duration_seconds,
            http_response_size_bytes,
            active_connections,
            system_cpu_usage,
            system_memory_usage,
            request_rate,
            error_rate,
        }
    }

    pub fn registry(&self) -> &Registry {
        &self.registry
    }

    pub fn increment_requests(&self, method: &str, path: &str, status_code: u16) {
        if let Some(metric) = &self.http_requests_total {
            let labels = &[method, path, &status_code.to_string()];
            metric.with_label_values(labels).inc();
        }
    }

    pub fn record_request_duration(
        &self,
        method: &str,
        path: &str,
        status_code: u16,
        duration: Duration,
    ) {
        if let Some(metric) = &self.http_request_duration_seconds {
            let labels = &[method, path, &status_code.to_string()];
            metric
                .with_label_values(labels)
                .observe(duration.as_secs_f64());
        }
    }

    pub fn record_response_size(&self, method: &str, path: &str, status_code: u16, size: usize) {
        if let Some(metric) = &self.http_response_size_bytes {
            let labels = &[method, path, &status_code.to_string()];
            metric.with_label_values(labels).observe(size as f64);
        }
    }

    pub fn increment_active_connections(&self) {
        if let Some(metric) = &self.active_connections {
            metric.inc();
        }
    }

    pub fn decrement_active_connections(&self) {
        if let Some(metric) = &self.active_connections {
            metric.dec();
        }
    }

    pub fn update_system_metrics(&self, system: &System) {
        if let Some(metric) = &self.system_cpu_usage {
            metric.set(system.global_cpu_usage() as f64);
        }

        if let Some(metric) = &self.system_memory_usage {
            let total_memory = system.total_memory();
            let used_memory = system.used_memory();
            let memory_usage_percent = if total_memory == 0 {
                0.0
            } else {
                (used_memory as f64 / total_memory as f64) * 100.0
            };
            metric.set(memory_usage_percent);
        }
    }

    pub fn update_request_rate(&self, rate: f64) {
        if let Some(metric) = &self.request_rate {
            metric.set(rate);
        }
    }

    pub fn update_error_rate(&self, rate: f64) {
        if let Some(metric) = &self.error_rate {
            metric.set(rate);
        }
    }

    pub fn active_connections_value(&self) -> f64 {
        self.active_connections.as_ref().map_or(0.0, Gauge::get)
    }

    pub fn system_cpu_usage_value(&self) -> f64 {
        self.system_cpu_usage.as_ref().map_or(0.0, Gauge::get)
    }

    pub fn system_memory_usage_value(&self) -> f64 {
        self.system_memory_usage.as_ref().map_or(0.0, Gauge::get)
    }

    pub fn export_metrics(&self) -> Result<String, Box<dyn std::error::Error>> {
        let encoder = TextEncoder::new();
        let metric_families = self.registry.gather();
        let mut buffer = Vec::new();
        encoder.encode(&metric_families, &mut buffer)?;
        Ok(String::from_utf8(buffer)?)
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

fn register_metric<T>(
    registry: &Registry,
    metric_name: &str,
    metric_result: Result<T, prometheus::Error>,
) -> Option<T>
where
    T: Collector + Clone + 'static,
{
    // Fail open: if metric creation/registration fails, keep server running
    // and log details for observability.
    let metric = match metric_result {
        Ok(metric) => metric,
        Err(err) => {
            error!(
                metric = metric_name,
                error = %err,
                "Failed to create Prometheus metric"
            );
            return None;
        }
    };

    if let Err(err) = registry.register(Box::new(metric.clone())) {
        error!(
            metric = metric_name,
            error = %err,
            "Failed to register Prometheus metric"
        );
        return None;
    }

    Some(metric)
}

#[derive(Debug, Clone)]
pub struct RequestStats {
    start_time: Instant,
    total_requests: u64,
    error_requests: u64,
    last_request_time: Instant,
}

impl RequestStats {
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
            total_requests: 0,
            error_requests: 0,
            last_request_time: Instant::now(),
        }
    }

    pub fn record_request(&mut self, is_error: bool) {
        self.total_requests += 1;
        if is_error {
            self.error_requests += 1;
        }
        self.last_request_time = Instant::now();
    }

    pub fn request_rate(&self) -> f64 {
        let elapsed = self.start_time.elapsed().as_secs_f64();
        if elapsed > 0.0 {
            self.total_requests as f64 / elapsed
        } else {
            0.0
        }
    }

    pub fn error_rate(&self) -> f64 {
        if self.total_requests > 0 {
            (self.error_requests as f64 / self.total_requests as f64) * 100.0
        } else {
            0.0
        }
    }

    pub fn total_requests(&self) -> u64 {
        self.total_requests
    }

    pub fn error_requests(&self) -> u64 {
        self.error_requests
    }

    pub fn success_requests(&self) -> u64 {
        self.total_requests.saturating_sub(self.error_requests)
    }

    pub fn uptime(&self) -> Duration {
        self.start_time.elapsed()
    }
}

impl Default for RequestStats {
    fn default() -> Self {
        Self::new()
    }
}

pub type SharedMetrics = Arc<RwLock<MetricsCollector>>;
pub type SharedStats = Arc<RwLock<RequestStats>>;

#[derive(Debug, Clone)]
pub struct MonitoringState {
    pub metrics: SharedMetrics,
    pub stats: SharedStats,
    pub system: Arc<RwLock<System>>,
}

impl MonitoringState {
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(RwLock::new(MetricsCollector::new())),
            stats: Arc::new(RwLock::new(RequestStats::new())),
            system: Arc::new(RwLock::new(System::new_all())),
        }
    }

    pub async fn update_system_metrics(&self) {
        let mut system = self.system.write().await;
        system.refresh_all();

        let metrics = self.metrics.read().await;
        metrics.update_system_metrics(&system);
    }
}

impl Default for MonitoringState {
    fn default() -> Self {
        Self::new()
    }
}

pub async fn metrics_endpoint(State(state): State<MonitoringState>) -> impl IntoResponse {
    match state.metrics.read().await.export_metrics() {
        Ok(metrics) => (
            StatusCode::OK,
            [("content-type", "text/plain; version=0.0.4; charset=utf-8")],
            metrics,
        )
            .into_response(),
        Err(e) => {
            error!("Failed to export metrics: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to export metrics",
            )
                .into_response()
        }
    }
}

#[derive(Debug, Serialize)]
pub struct MonitoringInfo {
    pub uptime_seconds: f64,
    pub total_requests: u64,
    pub error_requests: u64,
    pub request_rate: f64,
    pub error_rate: f64,
    pub active_connections: f64,
    pub system_cpu_usage: f64,
    pub system_memory_usage: f64,
}

pub async fn monitoring_info_endpoint(
    State(state): State<MonitoringState>,
) -> Json<MonitoringInfo> {
    let stats = state.stats.read().await;
    let metrics = state.metrics.read().await;

    let info = MonitoringInfo {
        uptime_seconds: stats.start_time.elapsed().as_secs_f64(),
        total_requests: stats.total_requests,
        error_requests: stats.error_requests,
        request_rate: stats.request_rate(),
        error_rate: stats.error_rate(),
        active_connections: metrics.active_connections_value(),
        system_cpu_usage: metrics.system_cpu_usage_value(),
        system_memory_usage: metrics.system_memory_usage_value(),
    };

    Json(info)
}

pub async fn performance_monitoring_middleware(
    State(state): State<MonitoringState>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let start_time = Instant::now();
    let method = request.method().to_string();
    let raw_path = request.uri().path().to_string();
    let path = metric_path_label(&request);

    state.metrics.read().await.increment_active_connections();

    let response = next.run(request).await;

    let status_code = response.status().as_u16();
    let duration = start_time.elapsed();
    let response_size = response.body().size_hint().lower() as usize;

    let is_error = status_code >= 400;

    {
        let mut stats = state.stats.write().await;
        stats.record_request(is_error);

        let metrics = state.metrics.read().await;
        metrics.increment_requests(&method, &path, status_code);
        metrics.record_request_duration(&method, &path, status_code, duration);
        metrics.record_response_size(&method, &path, status_code, response_size);
        metrics.update_request_rate(stats.request_rate());
        metrics.update_error_rate(stats.error_rate());
    }

    state.metrics.read().await.decrement_active_connections();

    debug!(
        method = %method,
        path = %raw_path,
        path_label = %path,
        status_code = %status_code,
        duration_ms = duration.as_millis(),
        "Request processed"
    );

    Ok(response)
}

fn metric_path_label(request: &Request) -> String {
    // Use matched route template to prevent high-cardinality metric labels
    // (e.g. collapse `/users/123` into `/users/{id}`).
    request
        .extensions()
        .get::<MatchedPath>()
        .map(|matched_path| matched_path.as_str().to_string())
        .unwrap_or_else(|| UNMATCHED_PATH_LABEL.to_string())
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HealthCheckConfig {
    pub database_url: Option<String>,
    pub redis_url: Option<String>,
    pub external_services: Vec<String>,
}

impl HealthCheckConfig {
    fn has_runtime_targets(&self) -> bool {
        self.database_url.is_some()
            || self.redis_url.is_some()
            || !self.external_services.is_empty()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    pub status: String,
    pub timestamp: String,
    pub uptime_seconds: f64,
    pub checks: HashMap<String, HealthCheckResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckResult {
    pub status: String,
    pub message: Option<String>,
    pub response_time_ms: Option<u64>,
}

pub async fn enhanced_health_check(
    State(state): State<MonitoringState>,
    config: Option<Json<HealthCheckConfig>>,
) -> impl IntoResponse {
    let mut checks = HashMap::new();

    checks.insert(
        "server".to_string(),
        HealthCheckResult {
            status: "healthy".to_string(),
            message: None,
            response_time_ms: None,
        },
    );

    if let Some(config) = config {
        if config.has_runtime_targets() && !runtime_health_targets_allowed() {
            checks.insert(
                "runtime_targets".to_string(),
                HealthCheckResult {
                    status: "unhealthy".to_string(),
                    message: Some(format!(
                        "Runtime health targets are disabled. Set {}=true to allow.",
                        ALLOW_RUNTIME_HEALTH_TARGETS_ENV
                    )),
                    response_time_ms: None,
                },
            );
        } else {
            if let Some(database_url) = &config.database_url {
                let db_check = check_database_connection(database_url).await;
                checks.insert("database".to_string(), db_check);
            }

            if let Some(redis_url) = &config.redis_url {
                let redis_check = check_redis_connection(redis_url).await;
                checks.insert("redis".to_string(), redis_check);
            }

            for (index, service) in config
                .external_services
                .iter()
                .take(MAX_EXTERNAL_SERVICE_CHECKS)
                .enumerate()
            {
                let service_check = match validate_external_service_target(service).await {
                    Ok(_) => check_external_service(service).await,
                    Err(message) => HealthCheckResult {
                        status: "unhealthy".to_string(),
                        message: Some(format!(
                            "External service target '{}' blocked: {}",
                            service, message
                        )),
                        response_time_ms: None,
                    },
                };
                checks.insert(format!("service_{}", index + 1), service_check);
            }

            if config.external_services.len() > MAX_EXTERNAL_SERVICE_CHECKS {
                checks.insert(
                    "external_services_truncated".to_string(),
                    HealthCheckResult {
                        status: "disabled".to_string(),
                        message: Some(format!(
                            "Skipped {} external checks; maximum is {} per request",
                            config.external_services.len() - MAX_EXTERNAL_SERVICE_CHECKS,
                            MAX_EXTERNAL_SERVICE_CHECKS
                        )),
                        response_time_ms: None,
                    },
                );
            }
        }
    }

    let overall_status = if overall_checks_healthy(&checks) {
        "healthy"
    } else {
        "unhealthy"
    };

    let stats = state.stats.read().await;

    let health_status = HealthStatus {
        status: overall_status.to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        uptime_seconds: stats.start_time.elapsed().as_secs_f64(),
        checks,
    };

    let status = if overall_status == "healthy" {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    (status, Json(health_status))
}

fn overall_checks_healthy(checks: &HashMap<String, HealthCheckResult>) -> bool {
    checks
        .values()
        .all(|check| matches!(check.status.as_str(), "healthy" | "disabled"))
}

fn runtime_health_targets_allowed() -> bool {
    std::env::var(ALLOW_RUNTIME_HEALTH_TARGETS_ENV)
        .map(|value| matches!(value.to_ascii_lowercase().as_str(), "1" | "true" | "yes"))
        .unwrap_or(false)
}

async fn validate_external_service_target(service_url: &str) -> Result<(), String> {
    let uri: http::Uri = service_url
        .parse()
        .map_err(|_| "invalid URL format".to_string())?;

    match uri.scheme_str() {
        Some("http") | Some("https") => {}
        Some(scheme) => return Err(format!("unsupported URL scheme '{}'", scheme)),
        None => return Err("missing URL scheme".to_string()),
    }

    let authority = uri
        .authority()
        .ok_or_else(|| "missing URL authority".to_string())?;
    if authority.as_str().contains('@') {
        return Err("user-info in URL authority is not allowed".to_string());
    }

    let host = uri
        .host()
        .ok_or_else(|| "missing URL host".to_string())?
        .trim_end_matches('.')
        .to_ascii_lowercase();
    if host.is_empty() {
        return Err("missing URL host".to_string());
    }

    if host == "localhost" || host.ends_with(".localhost") {
        return Err("localhost targets are not allowed".to_string());
    }

    if let Ok(ip) = host.parse::<IpAddr>()
        && is_private_or_local_ip(ip)
    {
        return Err(format!("private or local IP '{}' is not allowed", ip));
    }

    if host.parse::<IpAddr>().is_err() {
        let port = uri.port_u16().unwrap_or_else(|| {
            if uri.scheme_str() == Some("https") {
                443
            } else {
                80
            }
        });
        validate_resolved_host_ips(&host, port).await?;
    }

    Ok(())
}

async fn validate_resolved_host_ips(host: &str, port: u16) -> Result<(), String> {
    let lookup_result = tokio::time::timeout(
        EXTERNAL_HEALTH_DNS_TIMEOUT,
        tokio::net::lookup_host((host, port)),
    )
    .await
    .map_err(|_| "DNS lookup timed out".to_string())?
    .map_err(|err| format!("DNS lookup failed: {}", err))?;

    let mut resolved_any = false;
    for socket_addr in lookup_result {
        resolved_any = true;
        let resolved_ip = socket_addr.ip();
        if is_private_or_local_ip(resolved_ip) {
            return Err(format!(
                "host resolves to private or local IP '{}' which is not allowed",
                resolved_ip
            ));
        }
    }

    if resolved_any {
        Ok(())
    } else {
        Err("DNS lookup returned no IP addresses".to_string())
    }
}

fn is_private_or_local_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ipv4) => {
            ipv4.is_private()
                || ipv4.is_loopback()
                || ipv4.is_link_local()
                || ipv4.is_unspecified()
                || ipv4.is_broadcast()
                || ipv4.is_multicast()
        }
        IpAddr::V6(ipv6) => {
            ipv6.is_loopback()
                || ipv6.is_unspecified()
                || ipv6.is_unique_local()
                || ipv6.is_unicast_link_local()
                || ipv6.is_multicast()
        }
    }
}

#[cfg_attr(not(feature = "database-health"), allow(unused_variables))]
async fn check_database_connection(database_url: &str) -> HealthCheckResult {
    let start_time = Instant::now();

    #[cfg(feature = "database-health")]
    {
        use sqlx::postgres::PgPoolOptions;

        match PgPoolOptions::new()
            .max_connections(1)
            .acquire_timeout(Duration::from_secs(3))
            .connect(database_url)
            .await
        {
            Ok(pool) => {
                let query_result = sqlx::query_scalar::<_, i32>("SELECT 1").fetch_one(&pool);
                let query_result =
                    tokio::time::timeout(HEALTH_DB_QUERY_TIMEOUT, query_result).await;
                pool.close().await;

                match query_result {
                    Ok(Ok(_)) => HealthCheckResult {
                        status: "healthy".to_string(),
                        message: None,
                        response_time_ms: Some(start_time.elapsed().as_millis() as u64),
                    },
                    Ok(Err(e)) => HealthCheckResult {
                        status: "unhealthy".to_string(),
                        message: Some(format!("Database query failed: {}", e)),
                        response_time_ms: Some(start_time.elapsed().as_millis() as u64),
                    },
                    Err(_) => HealthCheckResult {
                        status: "unhealthy".to_string(),
                        message: Some("Database query timed out".to_string()),
                        response_time_ms: Some(start_time.elapsed().as_millis() as u64),
                    },
                }
            }
            Err(e) => HealthCheckResult {
                status: "unhealthy".to_string(),
                message: Some(format!("Database connection failed: {}", e)),
                response_time_ms: Some(start_time.elapsed().as_millis() as u64),
            },
        }
    }

    #[cfg(not(feature = "database-health"))]
    {
        HealthCheckResult {
            status: "disabled".to_string(),
            message: Some(
                "Database health checks not enabled. Enable with 'database-health' feature."
                    .to_string(),
            ),
            response_time_ms: Some(start_time.elapsed().as_millis() as u64),
        }
    }
}

#[cfg_attr(not(feature = "redis-health"), allow(unused_variables))]
async fn check_redis_connection(redis_url: &str) -> HealthCheckResult {
    let start_time = Instant::now();

    #[cfg(feature = "redis-health")]
    {
        match tokio::time::timeout(EXTERNAL_HEALTH_CHECK_TIMEOUT, async {
            match redis::Client::open(redis_url) {
                Ok(client) => match client.get_multiplexed_async_connection().await {
                    Ok(mut conn) => match redis::cmd("PING").query_async::<String>(&mut conn).await
                    {
                        Ok(_) => HealthCheckResult {
                            status: "healthy".to_string(),
                            message: None,
                            response_time_ms: Some(start_time.elapsed().as_millis() as u64),
                        },
                        Err(e) => HealthCheckResult {
                            status: "unhealthy".to_string(),
                            message: Some(format!("Redis PING failed: {}", e)),
                            response_time_ms: Some(start_time.elapsed().as_millis() as u64),
                        },
                    },
                    Err(e) => HealthCheckResult {
                        status: "unhealthy".to_string(),
                        message: Some(format!("Redis connection failed: {}", e)),
                        response_time_ms: Some(start_time.elapsed().as_millis() as u64),
                    },
                },
                Err(e) => HealthCheckResult {
                    status: "unhealthy".to_string(),
                    message: Some(format!("Redis client creation failed: {}", e)),
                    response_time_ms: Some(start_time.elapsed().as_millis() as u64),
                },
            }
        })
        .await
        {
            Ok(result) => result,
            Err(_) => HealthCheckResult {
                status: "unhealthy".to_string(),
                message: Some("Redis health check timed out".to_string()),
                response_time_ms: Some(start_time.elapsed().as_millis() as u64),
            },
        }
    }

    #[cfg(not(feature = "redis-health"))]
    {
        HealthCheckResult {
            status: "disabled".to_string(),
            message: Some(
                "Redis health checks not enabled. Enable with 'redis-health' feature.".to_string(),
            ),
            response_time_ms: Some(start_time.elapsed().as_millis() as u64),
        }
    }
}

#[cfg_attr(not(feature = "external-health"), allow(unused_variables))]
async fn check_external_service(service_url: &str) -> HealthCheckResult {
    let start_time = Instant::now();

    #[cfg(feature = "external-health")]
    {
        let client = match reqwest::Client::builder()
            .timeout(EXTERNAL_HEALTH_CHECK_TIMEOUT)
            .redirect(reqwest::redirect::Policy::none())
            .build()
        {
            Ok(client) => client,
            Err(e) => {
                return HealthCheckResult {
                    status: "unhealthy".to_string(),
                    message: Some(format!("HTTP client initialization failed: {}", e)),
                    response_time_ms: Some(start_time.elapsed().as_millis() as u64),
                };
            }
        };

        match client.get(service_url).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    HealthCheckResult {
                        status: "healthy".to_string(),
                        message: None,
                        response_time_ms: Some(start_time.elapsed().as_millis() as u64),
                    }
                } else {
                    HealthCheckResult {
                        status: "unhealthy".to_string(),
                        message: Some(format!("Service returned status: {}", response.status())),
                        response_time_ms: Some(start_time.elapsed().as_millis() as u64),
                    }
                }
            }
            Err(e) => HealthCheckResult {
                status: "unhealthy".to_string(),
                message: Some(format!("Service request failed: {}", e)),
                response_time_ms: Some(start_time.elapsed().as_millis() as u64),
            },
        }
    }

    #[cfg(not(feature = "external-health"))]
    {
        HealthCheckResult {
            status: "disabled".to_string(),
            message: Some("External service health checks not enabled. Enable with 'external-health' feature.".to_string()),
            response_time_ms: Some(start_time.elapsed().as_millis() as u64),
        }
    }
}

pub fn setup_metrics_recorder(state: MonitoringState) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(5));

        loop {
            interval.tick().await;
            state.update_system_metrics().await;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        Router,
        body::Body,
        http::{Request as HttpRequest, StatusCode},
        middleware,
        response::IntoResponse,
        routing::get,
    };
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    async fn ok_handler() -> &'static str {
        "ok"
    }

    #[tokio::test]
    async fn metrics_use_matched_path_label() {
        let state = MonitoringState::new();
        let app = Router::new().route("/users/{id}", get(ok_handler)).layer(
            middleware::from_fn_with_state(state.clone(), performance_monitoring_middleware),
        );

        let response = app
            .oneshot(
                HttpRequest::builder()
                    .uri("/users/123")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let metrics_text = state.metrics.read().await.export_metrics().unwrap();
        assert!(metrics_text.contains("path=\"/users/{id}\""));
        assert!(!metrics_text.contains("path=\"/users/123\""));
    }

    #[tokio::test]
    async fn metrics_collapse_unmatched_path_label() {
        let state = MonitoringState::new();
        let app = Router::new()
            .route("/", get(ok_handler))
            .layer(middleware::from_fn_with_state(
                state.clone(),
                performance_monitoring_middleware,
            ));

        let response = app
            .oneshot(
                HttpRequest::builder()
                    .uri("/totally/random/path/123")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);

        let metrics_text = state.metrics.read().await.export_metrics().unwrap();
        assert!(metrics_text.contains("path=\"__unmatched__\""));
    }

    #[tokio::test]
    async fn enhanced_health_check_returns_503_when_unhealthy() {
        let state = MonitoringState::new();
        let response = enhanced_health_check(
            State(state),
            Some(Json(HealthCheckConfig {
                external_services: vec!["http://127.0.0.1:8080".to_string()],
                ..HealthCheckConfig::default()
            })),
        )
        .await
        .into_response();

        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[test]
    fn disabled_checks_do_not_make_overall_status_unhealthy() {
        let checks = HashMap::from([
            (
                "server".to_string(),
                HealthCheckResult {
                    status: "healthy".to_string(),
                    message: None,
                    response_time_ms: None,
                },
            ),
            (
                "database".to_string(),
                HealthCheckResult {
                    status: "disabled".to_string(),
                    message: Some("feature disabled".to_string()),
                    response_time_ms: None,
                },
            ),
        ]);

        assert!(overall_checks_healthy(&checks));
    }

    #[tokio::test]
    async fn runtime_targets_include_explicit_security_error() {
        let state = MonitoringState::new();
        let services = (0..(MAX_EXTERNAL_SERVICE_CHECKS + 3))
            .map(|i| format!("http://127.0.0.1:{}", 8000 + i))
            .collect::<Vec<_>>();

        let response = enhanced_health_check(
            State(state),
            Some(Json(HealthCheckConfig {
                external_services: services,
                ..HealthCheckConfig::default()
            })),
        )
        .await
        .into_response();

        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);

        let body = response
            .into_body()
            .collect()
            .await
            .expect("body should be readable")
            .to_bytes();
        let payload: HealthStatus =
            serde_json::from_slice(&body).expect("health status payload should be valid JSON");

        assert_eq!(payload.checks.len(), 2);
        assert!(payload.checks.contains_key("runtime_targets"));
    }
}
