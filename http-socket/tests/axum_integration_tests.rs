#![cfg(feature = "axum")]

use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::extract::Extension;
use axum::http::{Request, StatusCode};
use axum::routing::get;
use http_socket::error::ExtensionError;
use http_socket::extension::middleware::MiddlewareHook;
use http_socket::integration::axum::{
    AxumMiddlewareError, AxumRequestContext, HttpSocketAxumLayer, RouterHttpSocketExt,
};
use tower::ServiceExt;

#[derive(Debug)]
struct RejectSocketHook;

impl MiddlewareHook<AxumRequestContext> for RejectSocketHook {
    fn before(&self, request: &AxumRequestContext) -> Result<(), ExtensionError> {
        if request.path.starts_with("/socket") {
            return Err(ExtensionError::MiddlewareRejected(
                "socket blocked".to_string(),
            ));
        }

        Ok(())
    }
}

async fn socket_handler(error: Option<Extension<AxumMiddlewareError>>) -> StatusCode {
    if error.is_some() {
        StatusCode::UNAUTHORIZED
    } else {
        StatusCode::OK
    }
}

#[tokio::test]
async fn axum_layer_allows_requests_without_hook() {
    let app = Router::new()
        .route("/socket/connect", get(socket_handler))
        .with_http_socket(HttpSocketAxumLayer::new());

    let response = app
        .oneshot(
            Request::builder()
                .uri("/socket/connect")
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("request should be served");

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn axum_layer_enforces_rejection_by_default() {
    let layer = HttpSocketAxumLayer::new().with_hook(Arc::new(RejectSocketHook));
    let app = Router::new()
        .route("/socket/connect", get(socket_handler))
        .with_http_socket(layer);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/socket/connect")
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("request should be served");

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn axum_layer_can_inject_error_without_short_circuit() {
    let layer = HttpSocketAxumLayer::new()
        .with_hook(Arc::new(RejectSocketHook))
        .inject_only();
    let app = Router::new()
        .route("/socket/connect", get(socket_handler))
        .with_http_socket(layer);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/socket/connect")
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("request should be served");

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}
