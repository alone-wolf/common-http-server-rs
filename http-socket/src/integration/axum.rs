use std::convert::Infallible;
use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use axum::Router;
use axum::response::{IntoResponse, Response};
use http::{Method, Request, StatusCode};
use tower::{Layer, Service};

use crate::error::ExtensionError;
use crate::extension::middleware::MiddlewareHook;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AxumRequestContext {
    pub method: Method,
    pub path: String,
}

impl AxumRequestContext {
    fn from_request<B>(request: &Request<B>) -> Self {
        Self {
            method: request.method().clone(),
            path: request.uri().path().to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AxumMiddlewareError {
    pub reason: String,
    pub status: StatusCode,
}

impl AxumMiddlewareError {
    pub fn status(&self) -> StatusCode {
        self.status
    }
}

fn status_code_for_error(error: &ExtensionError) -> StatusCode {
    match error {
        ExtensionError::MiddlewareRejected(reason) => {
            let normalized = reason.to_ascii_lowercase();
            if normalized.contains("only accepts") {
                StatusCode::METHOD_NOT_ALLOWED
            } else if normalized.contains("forbidden")
                || normalized.contains("denied")
                || normalized.contains("blocked")
            {
                StatusCode::FORBIDDEN
            } else {
                StatusCode::UNAUTHORIZED
            }
        }
        ExtensionError::HookFailed(_) => StatusCode::UNAUTHORIZED,
    }
}

impl From<ExtensionError> for AxumMiddlewareError {
    fn from(value: ExtensionError) -> Self {
        let status = status_code_for_error(&value);
        Self {
            reason: value.to_string(),
            status,
        }
    }
}

pub type AxumMiddlewareHook = Arc<dyn MiddlewareHook<AxumRequestContext>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MiddlewareRejectionMode {
    Enforce,
    InjectOnly,
}

#[derive(Clone)]
pub struct HttpSocketAxumLayer {
    hook: Option<AxumMiddlewareHook>,
    protected_prefixes: Vec<String>,
    rejection_mode: MiddlewareRejectionMode,
}

impl fmt::Debug for HttpSocketAxumLayer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("HttpSocketAxumLayer")
            .field("has_hook", &self.hook.is_some())
            .field("protected_prefixes", &self.protected_prefixes)
            .field("rejection_mode", &self.rejection_mode)
            .finish()
    }
}

impl Default for HttpSocketAxumLayer {
    fn default() -> Self {
        Self {
            hook: None,
            protected_prefixes: vec!["/socket".to_string(), "/ws".to_string()],
            rejection_mode: MiddlewareRejectionMode::Enforce,
        }
    }
}

impl HttpSocketAxumLayer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_hook(mut self, hook: AxumMiddlewareHook) -> Self {
        self.hook = Some(hook);
        self
    }

    pub fn protect_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.protected_prefixes.push(prefix.into());
        self
    }

    pub fn enforce_rejection(mut self) -> Self {
        self.rejection_mode = MiddlewareRejectionMode::Enforce;
        self
    }

    pub fn inject_only(mut self) -> Self {
        self.rejection_mode = MiddlewareRejectionMode::InjectOnly;
        self
    }

    fn should_check(&self, path: &str) -> bool {
        self.protected_prefixes
            .iter()
            .any(|prefix| path.starts_with(prefix))
    }
}

impl<S> Layer<S> for HttpSocketAxumLayer {
    type Service = HttpSocketAxumService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        HttpSocketAxumService {
            inner,
            layer: self.clone(),
        }
    }
}

#[derive(Clone)]
pub struct HttpSocketAxumService<S> {
    inner: S,
    layer: HttpSocketAxumLayer,
}

impl<S, B> Service<Request<B>> for HttpSocketAxumService<S>
where
    S: Service<Request<B>, Response = Response, Error = Infallible> + Send + 'static,
    S::Future: Send + 'static,
    B: Send + 'static,
{
    type Response = Response;
    type Error = Infallible;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut request: Request<B>) -> Self::Future {
        if self.layer.should_check(request.uri().path())
            && let Some(hook) = &self.layer.hook
        {
            let context = AxumRequestContext::from_request(&request);
            if let Err(error) = hook.before(&context) {
                let middleware_error = AxumMiddlewareError::from(error);
                if self.layer.rejection_mode == MiddlewareRejectionMode::Enforce {
                    let status = middleware_error.status();
                    let response = (
                        status,
                        format!("middleware rejected: {}", middleware_error.reason),
                    )
                        .into_response();
                    return Box::pin(async move { Ok(response) });
                }
                request.extensions_mut().insert(middleware_error);
            }
        }

        let future = self.inner.call(request);
        Box::pin(future)
    }
}

pub trait RouterHttpSocketExt<S> {
    fn with_http_socket(self, layer: HttpSocketAxumLayer) -> Self;
}

impl<S> RouterHttpSocketExt<S> for Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    fn with_http_socket(self, layer: HttpSocketAxumLayer) -> Self {
        self.layer(layer)
    }
}
