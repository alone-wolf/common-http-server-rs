# Protection Features Guide

This guide demonstrates how to use the protection features in the common-http-server package.
For production hardening defaults and audit notes, see `doc/SECURITY_NOTES.md`.
All docs index: `doc/README.md`.

## Features

### 1. Rate Limiting
Control the frequency of requests from clients.

```rust
use common_http_server::protection;
use std::time::Duration;

// Create rate limiting service
let rate_limit_service = protection::rate_limit_presets::api()
    .burst_size(20)
    .vary_by_ip()
    .max_tracked_keys(100_000)
    .cleanup_interval(Duration::from_secs(30))
    .build();

// Apply middleware
let app = Router::new()
    .layer(middleware::from_fn_with_state(
        rate_limit_service,
        protection::rate_limit_middleware,
    ));
```

### 2. IP Filtering
Whitelist or blacklist IP addresses and networks.

```rust
use common_http_server::protection;

// Create IP filter service
let ip_filter_service = protection::ip_filter_presets::private_networks()
    .log_blocked(true)
    .max_cache_entries(20_000)
    .build();

// Apply middleware
let app = Router::new()
    .layer(middleware::from_fn_with_state(
        ip_filter_service,
        protection::ip_filter_middleware,
    ));
```

### 3. Request Size Limiting
Limit the size of incoming requests.

```rust
use common_http_server::protection;

// Create size limit service
let size_limit_service = protection::size_limit_presets::api()
    .max_header_size(8 * 1024) // 8KB headers
    .max_url_length(2048) // 2KB URL
    .build();

// Apply middleware
let app = Router::new()
    .layer(middleware::from_fn_with_state(
        size_limit_service,
        protection::size_limit_middleware,
    ));
```

### 4. DDoS Protection
Comprehensive protection against DDoS attacks.

```rust
use common_http_server::protection;
use std::time::Duration;

// Create DDoS protection service (single-module usage)
let ddos_service = protection::ddos_presets::moderate()
    .burst_threshold(100) // Max 100 requests in 10 seconds
    .sustained_threshold(500) // Max 500 requests in 60 seconds
    .max_concurrent_connections(200) // Limit in-flight concurrent requests
    .enable_metrics(true) // Turn DDoS counters on/off
    .auto_ban(true, 1000, Duration::from_secs(3600)) // Auto-ban after 1000 suspicious requests
    .slow_down(true, Duration::from_millis(100))
    .build();

// Apply middleware
let app = Router::new()
    .layer(middleware::from_fn_with_state(
        ddos_service,
        protection::ddos_protection_middleware,
    ));
```

### 5. One-Shot Stack Assembly (Recommended)
Build and apply all protection layers in one place:

```rust
use common_http_server::{AppBuilder, AppConfig, ProtectionStackBuilder, ddos_presets, rate_limit_presets, size_limit_presets};

let ddos_config = ddos_presets::moderate();
let rate_limit_config = rate_limit_presets::api();
let size_limit_config = size_limit_presets::api();

let protection = ProtectionStackBuilder::new()
    .with_ddos(ddos_config.clone())
    .with_rate_limit(rate_limit_config.clone())
    .with_size_limit_content_length_only(size_limit_config.clone())
    .build()?;

let app_builder = AppBuilder::new(AppConfig::default())
    .validate_ddos_config(ddos_config)
    .validate_rate_limit_config(rate_limit_config)
    .validate_size_limit_config(size_limit_config)
    .with_protection(protection);
```

For most production services, prefer this one-shot stack over manually mixing
module internals.

## Presets

### Rate Limiting Presets
- `strict()`: 10 requests per minute
- `moderate()`: 100 requests per minute  
- `lenient()`: 1000 requests per minute
- `api()`: 60 requests per minute with burst
- `web()`: 200 requests per minute

### IP Filter Presets
- `localhost_only()`: Only allow localhost
- `private_networks()`: Allow private network ranges
- `block_known_malicious()`: Block known malicious ranges
- `corporate_network()`: Corporate network configuration

### Size Limit Presets
- `minimal()`: 1MB body, 4KB headers
- `moderate()`: 10MB body, 8KB headers
- `generous()`: 100MB body, 16KB headers
- `api()`: 5MB body, optimized for APIs
- `file_upload()`: 500MB body, for file uploads

### DDoS Protection Presets
- `strict()`: Strict limits with 2-hour bans
- `moderate()`: Moderate limits with 1-hour bans
- `lenient()`: Lenient limits, no auto-ban
- `api_protection()`: API-specific protection with challenges
  - When challenges are enabled, suspicious traffic gets `429 Too Many Requests`.
  - Challenge/concurrency throttling responses include `Retry-After`.

## Integration Example

```rust
use common_http_server::{
    AppBuilder, AppConfig, ProtectionStackBuilder, ddos_presets, ip_filter_presets,
    rate_limit_presets, size_limit_presets,
};

let ddos_config = ddos_presets::moderate();
let ip_filter_config = ip_filter_presets::private_networks();
let rate_limit_config = rate_limit_presets::api();
let size_limit_config = size_limit_presets::api();

let protection = ProtectionStackBuilder::new()
    .with_ddos(ddos_config.clone())
    .with_ip_filter(ip_filter_config.clone())
    .with_rate_limit(rate_limit_config.clone())
    .with_size_limit_content_length_only(size_limit_config.clone())
    .build()?;

let app_builder = AppBuilder::new(AppConfig::default())
    .validate_ddos_config(ddos_config)
    .validate_ip_filter_config(ip_filter_config)
    .validate_rate_limit_config(rate_limit_config)
    .validate_size_limit_config(size_limit_config)
    .with_protection(protection);
```

## Startup Fail-Fast Validation

When using `Server + AppBuilder`, you can validate auth/protection config before
the server binds its listening socket:

```rust
let app_builder = AppBuilder::new(AppConfig::default())
    .validate_auth_config(auth_config.clone())
    .validate_rate_limit_config(rate_limit_config.clone())
    .validate_ip_filter_config(ip_filter_config.clone())
    .validate_size_limit_config(size_limit_config.clone())
    .validate_ddos_config(ddos_config.clone());
```

## Configuration Tips

1. **Order matters**: Effective runtime order should be DDoS → IP Filter → Rate Limit → Size Limit
2. **Axum layering**: If you layer manually, apply in reverse (size first, DDoS last)
3. **Resource safety**: tune `max_tracked_keys` (rate-limit) and `max_cache_entries` (ip-filter)
4. **Reverse proxies**: configure `trusted_proxies` explicitly
5. **Monitoring**: use metrics to tune thresholds with real traffic
6. **Fail-fast**: keep startup validation for all protection configs

## Error Handling

All protection modules return specific error types:
- `RateLimitError`: Rate limiting violations
- `IpFilterError`: IP filtering violations  
- `SizeLimitError`: Size limit violations
- `DdosError`: DDoS protection violations

These can be handled in your error response middleware to provide appropriate HTTP status codes and messages.
