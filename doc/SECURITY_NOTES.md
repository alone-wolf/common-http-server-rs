# Security Notes

This document summarizes the current security baseline of `common-http-server` and recommended production settings.

## 1) Current Security Baseline

### Authentication
- JWT secret is validated (non-empty, not placeholder, minimum length).
- Basic auth now returns unified invalid-credential errors to reduce username-enumeration leakage.
- Transport security can be enforced with `HttpsPolicy::RequireSecureTransport`.

### Reverse Proxy Trust
- Client-IP extraction only trusts forwarding headers when peer IP is inside configured trusted proxies.
- Always configure `trusted_proxies` explicitly in production.

### Protection Modules
- `RateLimit`:
  - IP mode does not fall back to a shared `"unknown"` bucket anymore.
  - Added key-store lifecycle controls:
    - `cleanup_interval` (default 60s)
    - `max_tracked_keys` (default 50_000)
  - On key-pressure, limiter compacts stale entries.
- `IpFilter`:
  - Added cache bound: `max_cache_entries` (default 10_000).
  - If client IP cannot be determined:
    - `Allow` policy => allow
    - `Deny` policy => deny

### Monitoring / Health Checks
- External health targets are validated to reduce SSRF risk:
  - only `http/https`
  - block localhost/local/private IP targets
  - block URL userinfo (`user@host`)
- External checks are capped per request (max 8).
- External/Redis health checks now have timeout controls (default 3s).
- External HTTP checks disable redirects.
- Runtime-supplied health targets are disabled by default (opt-in via
  `COMMON_HTTP_SERVER_ALLOW_RUNTIME_HEALTH_TARGETS=true`).
- `disabled` checks no longer force overall unhealthy status.

### Logging
- Request logs now record `path` only (not full URI with query string) to reduce accidental sensitive-data exposure.

## 2) Recommended Production Hardening

1. Enforce secure transport:
   - set `https_policy = HttpsPolicy::RequireSecureTransport`
   - configure `trusted_proxies` to your ingress/LB CIDRs
2. Keep auth secrets out of code and rotate regularly.
3. Prefer `ProtectionStackBuilder` and keep startup fail-fast validation enabled.
4. Put `/metrics` and enhanced `/health` behind internal network/auth.
5. Treat health-check target lists as trusted config, not public user input.
6. Tune limits from real traffic:
   - rate-limit (`max_requests`, `burst_size`, `max_tracked_keys`)
   - ip-filter (`max_cache_entries`)
   - ddos thresholds and concurrency caps

## 3) Minimal Secure Setup Example

```rust
use common_http_server::{
    AppBuilder, AppConfig, HttpsPolicy, ProtectionStackBuilder,
    Server, ServerConfig, auth_presets, ddos_presets, rate_limit_presets, size_limit_presets,
};

let mut auth = auth_presets::production(std::env::var("JWT_SECRET")?);
auth.https_policy = HttpsPolicy::RequireSecureTransport;
// auth.trusted_proxies = ...;
let auth = auth.shared();

let rate_limit = rate_limit_presets::api()
    .max_tracked_keys(100_000)
    .cleanup_interval(std::time::Duration::from_secs(30));

let ddos = ddos_presets::moderate();
let size = size_limit_presets::api();

let protection = ProtectionStackBuilder::new()
    .with_ddos(ddos.clone())
    .with_rate_limit(rate_limit.clone())
    .with_size_limit_content_length_only(size.clone())
    .build()?;

let app_builder = AppBuilder::new(AppConfig::default())
    .validate_auth_config(auth.clone())
    .validate_rate_limit_config(rate_limit)
    .validate_ddos_config(ddos)
    .validate_size_limit_config(size)
    .with_protection(protection);

let server = Server::new(ServerConfig::new(3000), app_builder);
```

## 4) Residual Risk / Out of Scope

- No built-in WAF/rule-engine.
- No distributed rate-limit state backend (single-process memory model).
- Health checks can still consume outbound resources if misconfigured; protect endpoint access.
