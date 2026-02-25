//! Global middleware orchestration helpers.
//!
//! This module centralizes middleware assembly so applications can avoid
//! per-router duplication and accidental coverage gaps.

use crate::auth::{
    AuthError, SharedAuthConfig, api_key_auth_middleware, basic_auth_middleware,
    jwt_auth_middleware,
};
use crate::core::logging::structured_logging_middleware;
use crate::core::server::{AppConfig, ConfigError};
use crate::monitoring::{
    MonitoringState, PerformanceMonitoringConfig, performance_monitoring_middleware_with_config,
};
use crate::protection::ProtectionStack;
use axum::{
    Router,
    extract::{Request, State},
    middleware::{self, Next},
    response::Response,
};
use std::collections::HashSet;
use tracing::debug;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GlobalAuthMode {
    Basic,
    ApiKey,
    Jwt,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GlobalAuthFallback {
    Allow,
    DenyUnauthorized,
}

#[derive(Debug, Clone, Default)]
pub struct PathScope {
    include_exact: Vec<String>,
    include_prefixes: Vec<String>,
    exclude_exact: Vec<String>,
    exclude_prefixes: Vec<String>,
}

impl PathScope {
    pub fn all() -> Self {
        Self::default()
    }

    pub fn include_exact(mut self, path: impl Into<String>) -> Self {
        self.include_exact.push(normalize_scope_path(path.into()));
        self
    }

    pub fn include_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.include_prefixes
            .push(normalize_scope_path(prefix.into()));
        self
    }

    pub fn exclude_exact(mut self, path: impl Into<String>) -> Self {
        self.exclude_exact.push(normalize_scope_path(path.into()));
        self
    }

    pub fn exclude_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.exclude_prefixes
            .push(normalize_scope_path(prefix.into()));
        self
    }

    pub fn matches(&self, path: &str) -> bool {
        let path = normalize_scope_path(path.to_string());
        let included = if self.include_exact.is_empty() && self.include_prefixes.is_empty() {
            true
        } else {
            self.include_exact.iter().any(|item| item == &path)
                || self
                    .include_prefixes
                    .iter()
                    .any(|prefix| path_has_prefix_segment(&path, prefix))
        };

        let excluded = self.exclude_exact.iter().any(|item| item == &path)
            || self
                .exclude_prefixes
                .iter()
                .any(|prefix| path_has_prefix_segment(&path, prefix));

        included && !excluded
    }
}

#[derive(Debug, Clone)]
pub struct GlobalAuthConfig {
    pub auth_config: SharedAuthConfig,
    pub mode: GlobalAuthMode,
    pub scope: PathScope,
    pub realm: String,
    pub priority: i32,
}

impl GlobalAuthConfig {
    pub fn new(auth_config: SharedAuthConfig, mode: GlobalAuthMode) -> Self {
        Self {
            auth_config,
            mode,
            scope: PathScope::all(),
            realm: "default".to_string(),
            priority: 0,
        }
    }

    pub fn with_scope(mut self, scope: PathScope) -> Self {
        self.scope = scope;
        self
    }

    pub fn with_realm(mut self, realm: impl Into<String>) -> Self {
        self.realm = realm.into();
        self
    }

    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }
}

pub type AuthRule = GlobalAuthConfig;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthRealm(pub String);

#[derive(Debug, Clone, Default)]
pub struct GlobalMonitoringConfig {
    pub performance: PerformanceMonitoringConfig,
}

impl GlobalMonitoringConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_performance_config(mut self, performance: PerformanceMonitoringConfig) -> Self {
        self.performance = performance;
        self
    }
}

#[derive(Debug, Clone)]
pub struct MiddlewareOrchestrator {
    apply_app_runtime_layers: bool,
    monitoring: Option<(MonitoringState, GlobalMonitoringConfig)>,
    protection_stack: Option<ProtectionStack>,
    auth_rules: Vec<AuthRule>,
    auth_fallback: GlobalAuthFallback,
}

impl Default for MiddlewareOrchestrator {
    fn default() -> Self {
        Self {
            apply_app_runtime_layers: true,
            monitoring: None,
            protection_stack: None,
            auth_rules: Vec::new(),
            auth_fallback: GlobalAuthFallback::Allow,
        }
    }
}

impl MiddlewareOrchestrator {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_app_runtime_layers(mut self, enabled: bool) -> Self {
        self.apply_app_runtime_layers = enabled;
        self
    }

    pub fn with_monitoring(mut self, state: MonitoringState) -> Self {
        self.monitoring = Some((state, GlobalMonitoringConfig::default()));
        self
    }

    pub fn with_monitoring_config(
        mut self,
        state: MonitoringState,
        config: GlobalMonitoringConfig,
    ) -> Self {
        self.monitoring = Some((state, config));
        self
    }

    pub fn with_protection_stack(mut self, protection_stack: ProtectionStack) -> Self {
        self.protection_stack = Some(protection_stack);
        self
    }

    pub fn with_global_auth(mut self, auth_config: SharedAuthConfig, mode: GlobalAuthMode) -> Self {
        self.auth_rules = vec![GlobalAuthConfig::new(auth_config, mode)];
        self
    }

    pub fn with_global_auth_config(mut self, global_auth: GlobalAuthConfig) -> Self {
        self.auth_rules = vec![global_auth];
        self
    }

    pub fn with_auth_rule(mut self, auth_rule: AuthRule) -> Self {
        self.auth_rules.push(auth_rule);
        self
    }

    pub fn with_auth_rules(mut self, auth_rules: Vec<AuthRule>) -> Self {
        self.auth_rules.extend(auth_rules);
        self
    }

    pub fn with_auth_fallback(mut self, fallback: GlobalAuthFallback) -> Self {
        self.auth_fallback = fallback;
        self
    }

    pub(crate) fn validate(&self) -> Result<(), ConfigError> {
        let mut validated_configs = HashSet::new();

        for auth_rule in &self.auth_rules {
            let auth_config_ptr = std::sync::Arc::as_ptr(&auth_rule.auth_config);
            if validated_configs.insert(auth_config_ptr) {
                auth_rule
                    .auth_config
                    .validate()
                    .map_err(|error| ConfigError::InvalidAuth(error.to_string()))?;
            }
        }

        Ok(())
    }

    pub(crate) fn apply(self, mut router: Router, app_config: &AppConfig) -> Router {
        if !self.auth_rules.is_empty() || self.auth_fallback == GlobalAuthFallback::DenyUnauthorized
        {
            let mut auth_rules = self.auth_rules;
            // Higher priority first; for equal priority, preserve insertion order.
            auth_rules.sort_by(|left, right| right.priority.cmp(&left.priority));
            let auth_state = ScopedAuthState {
                rules: auth_rules,
                fallback: self.auth_fallback,
            };
            router = router.layer(middleware::from_fn_with_state(
                auth_state,
                scoped_auth_middleware,
            ));
        }

        if let Some(protection_stack) = self.protection_stack {
            router = protection_stack.apply_to_router(router);
        }

        if let Some((monitoring, config)) = self.monitoring {
            router = router.layer(middleware::from_fn(
                performance_monitoring_middleware_with_config(monitoring, config.performance),
            ));
        }

        if self.apply_app_runtime_layers {
            router = apply_runtime_layers(router, app_config);
        }

        router
    }
}

#[derive(Debug, Clone)]
struct ScopedAuthState {
    rules: Vec<AuthRule>,
    fallback: GlobalAuthFallback,
}

async fn scoped_auth_middleware(
    State(state): State<ScopedAuthState>,
    mut request: Request,
    next: Next,
) -> Result<Response, AuthError> {
    let path = request.uri().path().to_string();
    let matched_rule = state.rules.iter().find(|rule| rule.scope.matches(&path));

    if let Some(rule) = matched_rule {
        let realm = rule.realm.clone();
        request.extensions_mut().insert(AuthRealm(realm.clone()));
        debug!(
            path = %path,
            realm = %realm,
            mode = ?rule.mode,
            "Scoped auth rule matched"
        );

        let mut response = match rule.mode {
            GlobalAuthMode::Basic => {
                basic_auth_middleware(State(rule.auth_config.clone()), request, next).await
            }
            GlobalAuthMode::ApiKey => {
                api_key_auth_middleware(State(rule.auth_config.clone()), request, next).await
            }
            GlobalAuthMode::Jwt => {
                jwt_auth_middleware(State(rule.auth_config.clone()), request, next).await
            }
        }?;
        response.extensions_mut().insert(AuthRealm(realm));
        return Ok(response);
    }

    match state.fallback {
        GlobalAuthFallback::Allow => Ok(next.run(request).await),
        GlobalAuthFallback::DenyUnauthorized => Err(AuthError::MissingAuthHeader),
    }
}

fn apply_runtime_layers(mut router: Router, config: &AppConfig) -> Router {
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

fn normalize_scope_path(path: String) -> String {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return "/".to_string();
    }

    let with_leading = if trimmed.starts_with('/') {
        trimmed.to_string()
    } else {
        format!("/{trimmed}")
    };

    if with_leading.len() > 1 && with_leading.ends_with('/') {
        with_leading.trim_end_matches('/').to_string()
    } else {
        with_leading
    }
}

fn path_has_prefix_segment(path: &str, prefix: &str) -> bool {
    if prefix == "/" {
        return true;
    }

    path.strip_prefix(prefix)
        .is_some_and(|rest| rest.is_empty() || rest.starts_with('/'))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::presets;
    use axum::{
        Router,
        body::{Body, to_bytes},
        extract::Extension,
        http::{Request as HttpRequest, StatusCode},
        routing::get,
    };
    use tower::ServiceExt;

    async fn ok_handler() -> &'static str {
        "ok"
    }

    async fn realm_handler(Extension(realm): Extension<AuthRealm>) -> String {
        realm.0
    }

    fn auth_with_api_key(api_key: &str) -> SharedAuthConfig {
        crate::auth::AuthConfig {
            api_keys: vec![api_key.to_string()],
            ..crate::auth::AuthConfig::default()
        }
        .shared()
    }

    #[test]
    fn path_scope_prefix_matching_is_segment_aware() {
        let scope = PathScope::all().exclude_prefix("/panel");

        assert!(!scope.matches("/panel"));
        assert!(!scope.matches("/panel/api"));
        assert!(scope.matches("/panelized"));
    }

    #[test]
    fn path_scope_include_filters_paths() {
        let scope = PathScope::all()
            .include_prefix("/api")
            .exclude_exact("/api/private");

        assert!(scope.matches("/api/users"));
        assert!(!scope.matches("/panel"));
        assert!(!scope.matches("/api/private"));
    }

    #[test]
    fn orchestrator_validate_rejects_invalid_auth_rules() {
        let invalid_auth = crate::auth::AuthConfig::default()
            .with_jwt_secret("your-secret-key")
            .shared();

        let orchestrator = MiddlewareOrchestrator::new()
            .with_auth_rule(
                AuthRule::new(invalid_auth.clone(), GlobalAuthMode::ApiKey)
                    .with_scope(PathScope::all().include_prefix("/api")),
            )
            .with_auth_rule(
                AuthRule::new(invalid_auth, GlobalAuthMode::ApiKey)
                    .with_scope(PathScope::all().include_prefix("/realtime")),
            );

        assert!(matches!(
            orchestrator.validate(),
            Err(ConfigError::InvalidAuth(_))
        ));
    }

    #[tokio::test]
    async fn global_auth_scope_can_skip_health_routes() {
        let auth = presets::development().shared();
        let scope = PathScope::all().exclude_exact("/health");

        let router = Router::new()
            .route("/health", get(ok_handler))
            .route("/secure", get(ok_handler));

        let app = MiddlewareOrchestrator::new()
            .with_app_runtime_layers(false)
            .with_global_auth_config(
                GlobalAuthConfig::new(auth, GlobalAuthMode::ApiKey).with_scope(scope),
            )
            .apply(router, &AppConfig::default());

        let health_resp = app
            .clone()
            .oneshot(
                HttpRequest::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(health_resp.status(), StatusCode::OK);

        let secure_unauth_resp = app
            .clone()
            .oneshot(
                HttpRequest::builder()
                    .uri("/secure")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(secure_unauth_resp.status(), StatusCode::UNAUTHORIZED);

        let secure_auth_resp = app
            .oneshot(
                HttpRequest::builder()
                    .uri("/secure")
                    .header("authorization", "Bearer dev-api-key-1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(secure_auth_resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn matched_auth_rule_realm_is_available_to_handlers_and_response_extensions() {
        let auth = auth_with_api_key("tenant-a-key");

        let app = MiddlewareOrchestrator::new()
            .with_app_runtime_layers(false)
            .with_auth_rule(
                AuthRule::new(auth, GlobalAuthMode::ApiKey)
                    .with_realm("tenant-a")
                    .with_scope(PathScope::all().include_prefix("/tenant-a")),
            )
            .apply(
                Router::new().route("/tenant-a/realm", get(realm_handler)),
                &AppConfig::default(),
            );

        let response = app
            .oneshot(
                HttpRequest::builder()
                    .uri("/tenant-a/realm")
                    .header("authorization", "Bearer tenant-a-key")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.extensions().get::<AuthRealm>(),
            Some(&AuthRealm("tenant-a".to_string()))
        );

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        assert_eq!(std::str::from_utf8(&body).unwrap(), "tenant-a");
    }

    #[tokio::test]
    async fn auth_rules_support_same_mode_with_different_backends() {
        let auth_a = auth_with_api_key("tenant-a-key");
        let auth_b = auth_with_api_key("tenant-b-key");

        let app = MiddlewareOrchestrator::new()
            .with_app_runtime_layers(false)
            .with_auth_rule(
                AuthRule::new(auth_a, GlobalAuthMode::ApiKey)
                    .with_realm("tenant-a")
                    .with_scope(PathScope::all().include_prefix("/tenant-a")),
            )
            .with_auth_rule(
                AuthRule::new(auth_b, GlobalAuthMode::ApiKey)
                    .with_realm("tenant-b")
                    .with_scope(PathScope::all().include_prefix("/tenant-b")),
            )
            .apply(
                Router::new()
                    .route("/tenant-a/data", get(ok_handler))
                    .route("/tenant-b/data", get(ok_handler)),
                &AppConfig::default(),
            );

        let a_resp = app
            .clone()
            .oneshot(
                HttpRequest::builder()
                    .uri("/tenant-a/data")
                    .header("authorization", "Bearer tenant-a-key")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(a_resp.status(), StatusCode::OK);

        let b_wrong_resp = app
            .clone()
            .oneshot(
                HttpRequest::builder()
                    .uri("/tenant-b/data")
                    .header("authorization", "Bearer tenant-a-key")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(b_wrong_resp.status(), StatusCode::UNAUTHORIZED);

        let b_resp = app
            .oneshot(
                HttpRequest::builder()
                    .uri("/tenant-b/data")
                    .header("authorization", "Bearer tenant-b-key")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(b_resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn auth_rule_priority_uses_highest_priority_match() {
        let general_auth = auth_with_api_key("general-key");
        let admin_auth = auth_with_api_key("admin-key");

        let app = MiddlewareOrchestrator::new()
            .with_app_runtime_layers(false)
            .with_auth_rule(
                AuthRule::new(general_auth, GlobalAuthMode::ApiKey)
                    .with_scope(PathScope::all().include_prefix("/api"))
                    .with_priority(10),
            )
            .with_auth_rule(
                AuthRule::new(admin_auth, GlobalAuthMode::ApiKey)
                    .with_scope(PathScope::all().include_prefix("/api/admin"))
                    .with_priority(20),
            )
            .apply(
                Router::new().route("/api/admin/stats", get(ok_handler)),
                &AppConfig::default(),
            );

        let wrong_resp = app
            .clone()
            .oneshot(
                HttpRequest::builder()
                    .uri("/api/admin/stats")
                    .header("authorization", "Bearer general-key")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(wrong_resp.status(), StatusCode::UNAUTHORIZED);

        let correct_resp = app
            .oneshot(
                HttpRequest::builder()
                    .uri("/api/admin/stats")
                    .header("authorization", "Bearer admin-key")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(correct_resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn auth_fallback_can_deny_unmatched_paths() {
        let app = MiddlewareOrchestrator::new()
            .with_app_runtime_layers(false)
            .with_auth_fallback(GlobalAuthFallback::DenyUnauthorized)
            .apply(
                Router::new().route("/public", get(ok_handler)),
                &AppConfig::default(),
            );

        let response = app
            .oneshot(
                HttpRequest::builder()
                    .uri("/public")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }
}
