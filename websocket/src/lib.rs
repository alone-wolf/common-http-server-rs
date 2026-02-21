//! WebSocket group/event building blocks for `common-http-server-rs`.

mod protocol;

pub use protocol::{ClientMessage, EventActor, ServerMessage};

#[cfg(feature = "server")]
mod server;
#[cfg(feature = "server")]
pub use server::{
    WebSocketAuthMode, WebSocketError, WebSocketHub, WebSocketHubConfig, websocket_handler,
    websocket_router, websocket_router_with_auth,
};

#[cfg(feature = "client")]
mod client;
#[cfg(feature = "client")]
pub use client::{WebSocketClient, WebSocketClientBuilder, WebSocketClientError};
