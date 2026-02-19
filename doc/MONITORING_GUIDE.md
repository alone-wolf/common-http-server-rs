# Monitoring and Metrics Guide

This guide explains how to use the monitoring and metrics functionality in the common-http-server package.
For security constraints and deployment advice, see `doc/SECURITY_NOTES.md`.
All docs index: `doc/README.md`.

## Features

- **Prometheus Metrics Collection**: Comprehensive metrics collection with Prometheus format
- **Request Statistics**: Track request counts, response times, and error rates
- **Performance Monitoring Middleware**: Automatic request tracking and performance metrics
- **Enhanced Health Checks**: Database, Redis, and external service health monitoring
- **Health Safety Guards**: External target validation, per-request check cap, request timeout

## Quick Start

```rust
use axum::{Router, middleware, routing::get};
use common_http_server::{
    monitoring::{
        MonitoringState, metrics_endpoint, monitoring_info_endpoint,
        performance_monitoring_middleware, setup_metrics_recorder,
    },
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let monitoring_state = MonitoringState::new();
    setup_metrics_recorder(monitoring_state.clone());

    let app = Router::new()
        .route("/metrics", get(metrics_endpoint))
        .route("/monitoring", get(monitoring_info_endpoint))
        .layer(middleware::from_fn_with_state(
            monitoring_state.clone(),
            performance_monitoring_middleware,
        ))
        .with_state(monitoring_state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    axum::serve(listener, app).await?;
    Ok(())
}
```

## Available Endpoints

### `/metrics`
Exports Prometheus-formatted metrics including:
- `http_requests_total`: Total HTTP requests by method, normalized route path, and status
- `http_request_duration_seconds`: Request duration histogram
- `http_response_size_bytes`: Response size histogram
- `active_connections`: Number of active connections
- `system_cpu_usage_percent`: System CPU usage
- `system_memory_usage_percent`: System memory usage
- `requests_per_second`: Current request rate
- `error_rate_percent`: Current error rate

### `/monitoring`
Returns JSON with current monitoring statistics:
```json
{
  "uptime_seconds": 3600.5,
  "total_requests": 1250,
  "error_requests": 15,
  "request_rate": 0.35,
  "error_rate": 1.2,
  "active_connections": 5.0,
  "system_cpu_usage": 25.3,
  "system_memory_usage": 67.8
}
```

### `/health`
Enhanced health check with optional database, Redis, and external service checks:
```json
{
  "status": "healthy",
  "timestamp": "2024-02-16T12:33:00Z",
  "uptime_seconds": 3600.5,
  "checks": {
    "server": {
      "status": "healthy",
      "message": null,
      "response_time_ms": null
    },
    "database": {
      "status": "healthy",
      "message": null,
      "response_time_ms": 12
    }
  }
}
```

Notes:
- `disabled` checks do not make the overall status unhealthy.
- External service checks are capped per request (current cap: 8).
- External and Redis checks have a timeout (default: 3 seconds).
- Runtime-supplied health targets are disabled by default.  
  To explicitly allow them, set `COMMON_HTTP_SERVER_ALLOW_RUNTIME_HEALTH_TARGETS=true`.

## Configuration

### Health Check Configuration
```rust
use common_http_server::monitoring::{HealthCheckConfig, enhanced_health_check};

let config = HealthCheckConfig {
    database_url: Some("postgresql://user:pass@localhost/db".to_string()),
    redis_url: Some("redis://localhost:6379".to_string()),
    external_services: vec![
        "https://api.example.com/health".to_string(),
    ],
};

// Use with health endpoint
.route("/health", post(enhanced_health_check))
```

Security recommendations:
- Do not accept arbitrary external health targets from untrusted clients.
- Restrict access to enhanced health endpoints in production.

## Feature Flags

The monitoring module supports optional features for health checks:

- `database-health`: Enable database health checks (requires SQLx)
- `redis-health`: Enable Redis health checks (requires Redis client)
- `external-health`: Enable external service health checks (requires Reqwest)
- `full-health`: Enable all health check features

Enable them in your `Cargo.toml`:
```toml
[dependencies]
common-http-server = { version = "0.1.0", features = ["full-health"] }
```

## Custom Metrics

You can access the metrics collector directly to add custom metrics:

```rust
use common_http_server::monitoring::MetricsCollector;

let metrics = MetricsCollector::new();

// Increment custom counter
metrics.increment_requests("GET", "/api/custom", 200);

// Record custom duration
let duration = start_time.elapsed();
metrics.record_request_duration("POST", "/api/custom", 201, duration);

// Update system metrics
metrics.update_request_rate(10.5);
metrics.update_error_rate(2.3);
```

## Performance Considerations

- Metrics collection is designed to be low-overhead
- System metrics are updated every 5 seconds by default
- Health checks are performed on-demand when the endpoint is called
- All metrics operations are thread-safe and use async-friendly data structures
- External health checks enforce timeout and redirect restrictions

## Example Usage

See `examples/level3_security_and_monitoring.rs` for a complete working example that combines monitoring and protection.

Run the example:
```bash
cargo run -p common-http-server --example level3_security_and_monitoring
```

Then visit:
- http://localhost:3002/monitor/metrics - Prometheus metrics
- http://localhost:3002/monitor/monitoring - JSON monitoring info
- http://localhost:3002/health - base health endpoint
- http://localhost:3002/secure/profile - protected endpoint (requires Basic auth)
