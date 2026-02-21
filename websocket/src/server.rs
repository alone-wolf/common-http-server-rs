use crate::protocol::{ClientMessage, EventActor, ServerMessage};
use axum::{
    Extension, Router,
    extract::{
        State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    http::{HeaderMap, header::SEC_WEBSOCKET_PROTOCOL},
    middleware,
    response::IntoResponse,
    routing::get,
};
use chrono::Utc;
use common_http_server_rs::{
    AuthError, AuthType, AuthUser, SharedAuthConfig, api_key_auth_middleware,
    basic_auth_middleware, jwt_auth_middleware,
};
use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};
use tokio::sync::mpsc::error::TrySendError;
use tokio::sync::{RwLock, mpsc};
use tracing::{debug, info, warn};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WebSocketFrameFormat {
    TextJson = 0,
    BinaryMessagePack = 1,
}

impl WebSocketFrameFormat {
    fn from_u8(value: u8) -> Self {
        match value {
            1 => WebSocketFrameFormat::BinaryMessagePack,
            _ => WebSocketFrameFormat::TextJson,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum WebSocketError {
    #[error("connection not found")]
    ConnectionNotFound,
    #[error("invalid group name")]
    InvalidGroup,
    #[error("invalid event name")]
    InvalidEvent,
    #[error("group '{group}' not found")]
    GroupNotFound { group: String },
    #[error("connection is not in group '{group}'")]
    NotInGroup { group: String },
    #[error("outbound queue is full for connection '{connection_id}'")]
    OutboundQueueFull { connection_id: String },
}

impl WebSocketError {
    fn code(&self) -> &'static str {
        match self {
            WebSocketError::ConnectionNotFound => "connection_not_found",
            WebSocketError::InvalidGroup => "invalid_group",
            WebSocketError::InvalidEvent => "invalid_event",
            WebSocketError::GroupNotFound { .. } => "group_not_found",
            WebSocketError::NotInGroup { .. } => "not_in_group",
            WebSocketError::OutboundQueueFull { .. } => "outbound_queue_full",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WebSocketAuthMode {
    None,
    Basic,
    #[default]
    ApiKey,
    Jwt,
}

#[derive(Debug, Clone, Copy)]
pub struct WebSocketHubConfig {
    outbound_queue_capacity: usize,
}

impl Default for WebSocketHubConfig {
    fn default() -> Self {
        Self {
            outbound_queue_capacity: 256,
        }
    }
}

impl WebSocketHubConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn outbound_queue_capacity(mut self, capacity: usize) -> Self {
        self.outbound_queue_capacity = capacity.max(1);
        self
    }
}

#[derive(Debug, Clone)]
pub struct WebSocketHub {
    inner: Arc<RwLock<HubState>>,
    config: WebSocketHubConfig,
}

impl Default for WebSocketHub {
    fn default() -> Self {
        Self::with_config(WebSocketHubConfig::default())
    }
}

#[derive(Debug, Default)]
struct HubState {
    groups: HashMap<String, HashSet<String>>,
    peers: HashMap<String, PeerEntry>,
}

#[derive(Debug)]
struct PeerEntry {
    sender: mpsc::Sender<ServerMessage>,
    auth_user: Option<AuthUser>,
    groups: HashSet<String>,
}

impl WebSocketHub {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_config(config: WebSocketHubConfig) -> Self {
        Self {
            inner: Arc::new(RwLock::new(HubState::default())),
            config,
        }
    }

    pub async fn register(
        &self,
        auth_user: Option<AuthUser>,
    ) -> (String, mpsc::Receiver<ServerMessage>) {
        let connection_id = Uuid::new_v4().to_string();
        let (tx, rx) = mpsc::channel(self.config.outbound_queue_capacity.max(1));

        let actor = auth_user
            .as_ref()
            .map(event_actor_from_auth_user)
            .unwrap_or_else(|| anonymous_actor(&connection_id));

        {
            let mut state = self.inner.write().await;
            state.peers.insert(
                connection_id.clone(),
                PeerEntry {
                    sender: tx.clone(),
                    auth_user,
                    groups: HashSet::new(),
                },
            );
        }

        let _ = tx.try_send(ServerMessage::Connected {
            connection_id: connection_id.clone(),
            actor,
        });

        (connection_id, rx)
    }

    pub async fn unregister(&self, connection_id: &str) {
        let mut state = self.inner.write().await;

        let Some(peer) = state.peers.remove(connection_id) else {
            return;
        };

        for group in peer.groups {
            if let Some(members) = state.groups.get_mut(&group) {
                members.remove(connection_id);
                if members.is_empty() {
                    state.groups.remove(&group);
                }
            }
        }
    }

    pub async fn join_group(&self, connection_id: &str, group: &str) -> Result<(), WebSocketError> {
        validate_group_name(group)?;

        let mut state = self.inner.write().await;
        let peer = state
            .peers
            .get_mut(connection_id)
            .ok_or(WebSocketError::ConnectionNotFound)?;

        peer.groups.insert(group.to_string());
        state
            .groups
            .entry(group.to_string())
            .or_default()
            .insert(connection_id.to_string());

        Ok(())
    }

    pub async fn leave_group(
        &self,
        connection_id: &str,
        group: &str,
    ) -> Result<(), WebSocketError> {
        validate_group_name(group)?;

        let mut state = self.inner.write().await;

        {
            let peer = state
                .peers
                .get_mut(connection_id)
                .ok_or(WebSocketError::ConnectionNotFound)?;

            peer.groups.remove(group);
        }

        if let Some(members) = state.groups.get_mut(group) {
            members.remove(connection_id);
            if members.is_empty() {
                state.groups.remove(group);
            }
        }

        Ok(())
    }

    pub async fn emit_to_group(
        &self,
        connection_id: &str,
        group: &str,
        event: &str,
        payload: Value,
    ) -> Result<usize, WebSocketError> {
        validate_group_name(group)?;
        validate_event_name(event)?;

        let (senders, actor) = {
            let state = self.inner.read().await;

            let members = state
                .groups
                .get(group)
                .ok_or_else(|| WebSocketError::GroupNotFound {
                    group: group.to_string(),
                })?;

            if !members.contains(connection_id) {
                return Err(WebSocketError::NotInGroup {
                    group: group.to_string(),
                });
            }

            let peer = state
                .peers
                .get(connection_id)
                .ok_or(WebSocketError::ConnectionNotFound)?;

            let actor = peer
                .auth_user
                .as_ref()
                .map(event_actor_from_auth_user)
                .unwrap_or_else(|| anonymous_actor(connection_id));

            let senders = members
                .iter()
                .filter_map(|member_id| {
                    state
                        .peers
                        .get(member_id)
                        .map(|peer| (member_id.clone(), peer.sender.clone()))
                })
                .collect::<Vec<_>>();

            (senders, actor)
        };

        let mut permits = Vec::with_capacity(senders.len());
        for (member_id, sender) in &senders {
            match sender.try_reserve() {
                Ok(permit) => permits.push(permit),
                Err(TrySendError::Full(_)) => {
                    return Err(WebSocketError::OutboundQueueFull {
                        connection_id: member_id.clone(),
                    });
                }
                Err(TrySendError::Closed(_)) => return Err(WebSocketError::ConnectionNotFound),
            }
        }

        let message = ServerMessage::Event {
            group: group.to_string(),
            event: event.to_string(),
            payload,
            from: actor,
            timestamp: Utc::now().to_rfc3339(),
        };

        let mut delivered = 0;
        for permit in permits {
            permit.send(message.clone());
            delivered += 1;
        }

        Ok(delivered)
    }

    pub async fn send_to_connection(
        &self,
        connection_id: &str,
        message: ServerMessage,
    ) -> Result<(), WebSocketError> {
        let state = self.inner.read().await;
        let peer = state
            .peers
            .get(connection_id)
            .ok_or(WebSocketError::ConnectionNotFound)?;

        match peer.sender.try_send(message) {
            Ok(_) => Ok(()),
            Err(TrySendError::Full(_)) => Err(WebSocketError::OutboundQueueFull {
                connection_id: connection_id.to_string(),
            }),
            Err(TrySendError::Closed(_)) => Err(WebSocketError::ConnectionNotFound),
        }
    }

    pub async fn group_member_count(&self, group: &str) -> usize {
        let state = self.inner.read().await;
        state.groups.get(group).map_or(0, HashSet::len)
    }
}

fn event_actor_from_auth_user(auth_user: &AuthUser) -> EventActor {
    let auth_type = match auth_user.auth_type {
        AuthType::Basic => "basic",
        AuthType::ApiKey => "api_key",
        AuthType::Jwt => "jwt",
    };

    EventActor {
        user_id: auth_user.user.id.clone(),
        username: auth_user.user.username.clone(),
        auth_type: auth_type.to_string(),
    }
}

fn anonymous_actor(connection_id: &str) -> EventActor {
    EventActor {
        user_id: format!("anonymous:{}", connection_id),
        username: "anonymous".to_string(),
        auth_type: "none".to_string(),
    }
}

fn validate_group_name(group: &str) -> Result<(), WebSocketError> {
    if group.is_empty() || group.len() > 64 {
        return Err(WebSocketError::InvalidGroup);
    }

    if !group
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.' | ':'))
    {
        return Err(WebSocketError::InvalidGroup);
    }

    Ok(())
}

fn validate_event_name(event: &str) -> Result<(), WebSocketError> {
    if event.is_empty() || event.len() > 64 {
        return Err(WebSocketError::InvalidEvent);
    }

    if !event
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.' | ':'))
    {
        return Err(WebSocketError::InvalidEvent);
    }

    Ok(())
}

pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    State((hub, auth_mode)): State<(WebSocketHub, WebSocketAuthMode)>,
    headers: HeaderMap,
    auth_user: Option<Extension<AuthUser>>,
) -> axum::response::Response {
    let initial_frame_format = detect_initial_frame_format(&headers);
    let ws = ws.protocols(["msgpack", "json"]);

    let auth_user = match (auth_mode, auth_user) {
        (WebSocketAuthMode::None, Some(Extension(auth_user))) => Some(auth_user),
        (WebSocketAuthMode::None, None) => None,
        (_, Some(Extension(auth_user))) => Some(auth_user),
        (_, None) => return AuthError::MissingAuthHeader.into_response(),
    };

    ws.on_upgrade(move |socket| websocket_session(socket, hub, auth_user, initial_frame_format))
        .into_response()
}

pub fn websocket_router(path: &'static str, hub: WebSocketHub) -> Router {
    Router::new()
        .route(path, get(websocket_handler))
        .with_state((hub, WebSocketAuthMode::None))
}

pub fn websocket_router_with_auth(
    path: &'static str,
    hub: WebSocketHub,
    auth_config: SharedAuthConfig,
    auth_mode: WebSocketAuthMode,
) -> Router {
    let router = Router::new()
        .route(path, get(websocket_handler))
        .with_state((hub, auth_mode));

    match auth_mode {
        WebSocketAuthMode::None => router,
        WebSocketAuthMode::Basic => router.layer(middleware::from_fn_with_state(
            auth_config,
            basic_auth_middleware,
        )),
        WebSocketAuthMode::ApiKey => router.layer(middleware::from_fn_with_state(
            auth_config,
            api_key_auth_middleware,
        )),
        WebSocketAuthMode::Jwt => router.layer(middleware::from_fn_with_state(
            auth_config,
            jwt_auth_middleware,
        )),
    }
}

async fn websocket_session(
    socket: WebSocket,
    hub: WebSocketHub,
    auth_user: Option<AuthUser>,
    initial_frame_format: WebSocketFrameFormat,
) {
    let (connection_id, mut rx) = hub.register(auth_user).await;
    info!(connection_id = %connection_id, "websocket connected");

    let (mut sender, mut receiver) = socket.split();
    let frame_format = Arc::new(AtomicU8::new(initial_frame_format as u8));
    let forward_frame_format = frame_format.clone();

    let forward_task = tokio::spawn(async move {
        while let Some(message) = rx.recv().await {
            let format =
                WebSocketFrameFormat::from_u8(forward_frame_format.load(Ordering::Relaxed));

            let frame = match format {
                WebSocketFrameFormat::TextJson => match serde_json::to_string(&message) {
                    Ok(payload) => Message::Text(payload.into()),
                    Err(err) => {
                        warn!(error = %err, "failed to serialize websocket JSON message");
                        continue;
                    }
                },
                WebSocketFrameFormat::BinaryMessagePack => {
                    match rmp_serde::to_vec_named(&message) {
                        Ok(payload) => Message::Binary(payload.into()),
                        Err(err) => {
                            warn!(error = %err, "failed to serialize websocket MessagePack message");
                            continue;
                        }
                    }
                }
            };

            if sender.send(frame).await.is_err() {
                break;
            }
        }
    });

    while let Some(frame_result) = receiver.next().await {
        match frame_result {
            Ok(Message::Text(text)) => {
                frame_format.store(WebSocketFrameFormat::TextJson as u8, Ordering::Relaxed);
                process_client_text_frame(&hub, &connection_id, &text).await;
            }
            Ok(Message::Binary(payload)) => {
                frame_format.store(
                    WebSocketFrameFormat::BinaryMessagePack as u8,
                    Ordering::Relaxed,
                );
                process_client_binary_frame(&hub, &connection_id, &payload).await;
            }
            Ok(Message::Ping(_)) | Ok(Message::Pong(_)) => {}
            Ok(Message::Close(_)) => {
                debug!(connection_id = %connection_id, "websocket close frame received");
                break;
            }
            Err(err) => {
                warn!(connection_id = %connection_id, error = %err, "websocket frame read failed");
                break;
            }
        }
    }

    hub.unregister(&connection_id).await;
    forward_task.abort();
    info!(connection_id = %connection_id, "websocket disconnected");
}

fn detect_initial_frame_format(headers: &HeaderMap) -> WebSocketFrameFormat {
    let Some(raw) = headers
        .get(SEC_WEBSOCKET_PROTOCOL)
        .and_then(|value| value.to_str().ok())
    else {
        return WebSocketFrameFormat::TextJson;
    };

    let supports_msgpack = raw.split(',').map(str::trim).any(|token| {
        token.eq_ignore_ascii_case("msgpack") || token.eq_ignore_ascii_case("messagepack")
    });

    if supports_msgpack {
        WebSocketFrameFormat::BinaryMessagePack
    } else {
        WebSocketFrameFormat::TextJson
    }
}

async fn process_client_text_frame(hub: &WebSocketHub, connection_id: &str, text: &str) {
    let message = match serde_json::from_str::<ClientMessage>(text) {
        Ok(message) => message,
        Err(err) => {
            send_ws_error(
                hub,
                connection_id,
                "invalid_json",
                &format!("Failed to parse JSON message: {}", err),
            )
            .await;
            return;
        }
    };

    process_client_message(hub, connection_id, message).await;
}

async fn process_client_binary_frame(hub: &WebSocketHub, connection_id: &str, payload: &[u8]) {
    let message = match rmp_serde::from_slice::<ClientMessage>(payload) {
        Ok(message) => message,
        Err(err) => {
            send_ws_error(
                hub,
                connection_id,
                "invalid_binary",
                &format!("Failed to parse MessagePack frame: {}", err),
            )
            .await;
            return;
        }
    };

    process_client_message(hub, connection_id, message).await;
}

async fn process_client_message(hub: &WebSocketHub, connection_id: &str, message: ClientMessage) {
    match message {
        ClientMessage::Join { group } => match hub.join_group(connection_id, &group).await {
            Ok(_) => {
                if let Err(err) = hub
                    .send_to_connection(connection_id, ServerMessage::Joined { group })
                    .await
                {
                    warn!(connection_id = %connection_id, error = %err, "failed to send joined ack");
                }
            }
            Err(err) => send_ws_error(hub, connection_id, err.code(), &err.to_string()).await,
        },
        ClientMessage::Leave { group } => match hub.leave_group(connection_id, &group).await {
            Ok(_) => {
                if let Err(err) = hub
                    .send_to_connection(connection_id, ServerMessage::Left { group })
                    .await
                {
                    warn!(connection_id = %connection_id, error = %err, "failed to send left ack");
                }
            }
            Err(err) => send_ws_error(hub, connection_id, err.code(), &err.to_string()).await,
        },
        ClientMessage::Event {
            group,
            event,
            payload,
        } => match hub
            .emit_to_group(connection_id, &group, &event, payload)
            .await
        {
            Ok(delivered) => {
                debug!(
                    connection_id = %connection_id,
                    group = %group,
                    event = %event,
                    delivered,
                    "group event broadcasted"
                );
            }
            Err(err) => send_ws_error(hub, connection_id, err.code(), &err.to_string()).await,
        },
        ClientMessage::Ping { nonce } => {
            if let Err(err) = hub
                .send_to_connection(connection_id, ServerMessage::Pong { nonce })
                .await
            {
                warn!(connection_id = %connection_id, error = %err, "failed to send pong");
            }
        }
    }
}

async fn send_ws_error(hub: &WebSocketHub, connection_id: &str, code: &str, message: &str) {
    if let Err(err) = hub
        .send_to_connection(
            connection_id,
            ServerMessage::Error {
                code: code.to_string(),
                message: message.to_string(),
            },
        )
        .await
    {
        warn!(connection_id = %connection_id, error = %err, "failed to send websocket error");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::{HeaderMap, HeaderValue, header::SEC_WEBSOCKET_PROTOCOL};
    use common_http_server_rs::AuthType;
    use common_http_server_rs::auth::types::User;

    fn auth_user(name: &str) -> AuthUser {
        AuthUser {
            user: User {
                id: format!("{}_id", name),
                username: name.to_string(),
                roles: vec!["user".to_string()],
                permissions: vec![],
            },
            auth_type: AuthType::ApiKey,
        }
    }

    #[tokio::test]
    async fn group_join_and_leave_updates_member_count() {
        let hub = WebSocketHub::new();
        let (connection_id, mut rx) = hub.register(Some(auth_user("alice"))).await;

        let _ = rx.recv().await;

        hub.join_group(&connection_id, "chat.room").await.unwrap();
        assert_eq!(hub.group_member_count("chat.room").await, 1);

        hub.leave_group(&connection_id, "chat.room").await.unwrap();
        assert_eq!(hub.group_member_count("chat.room").await, 0);
    }

    #[tokio::test]
    async fn emit_event_broadcasts_json_message() {
        let hub = WebSocketHub::new();

        let (alice_id, mut alice_rx) = hub.register(Some(auth_user("alice"))).await;
        let _ = alice_rx.recv().await;

        let (bob_id, mut bob_rx) = hub.register(Some(auth_user("bob"))).await;
        let _ = bob_rx.recv().await;

        hub.join_group(&alice_id, "team.dev").await.unwrap();
        hub.join_group(&bob_id, "team.dev").await.unwrap();

        let delivered = hub
            .emit_to_group(
                &alice_id,
                "team.dev",
                "message.new",
                serde_json::json!({"text": "hello"}),
            )
            .await
            .unwrap();

        assert_eq!(delivered, 2);

        let alice_msg = alice_rx.recv().await.unwrap();
        let bob_msg = bob_rx.recv().await.unwrap();

        match (alice_msg, bob_msg) {
            (
                ServerMessage::Event {
                    group,
                    event,
                    payload,
                    ..
                },
                ServerMessage::Event {
                    group: group2,
                    event: event2,
                    payload: payload2,
                    ..
                },
            ) => {
                assert_eq!(group, "team.dev");
                assert_eq!(group2, "team.dev");
                assert_eq!(event, "message.new");
                assert_eq!(event2, "message.new");
                assert_eq!(payload, serde_json::json!({"text": "hello"}));
                assert_eq!(payload2, serde_json::json!({"text": "hello"}));
            }
            _ => panic!("expected event messages"),
        }
    }

    #[tokio::test]
    async fn emit_requires_membership() {
        let hub = WebSocketHub::new();
        let (connection_id, mut rx) = hub.register(Some(auth_user("alice"))).await;
        let _ = rx.recv().await;

        let error = hub
            .emit_to_group(
                &connection_id,
                "team.dev",
                "message.new",
                serde_json::json!({"text": "hello"}),
            )
            .await
            .unwrap_err();

        assert!(matches!(error, WebSocketError::GroupNotFound { .. }));
    }

    #[tokio::test]
    async fn emit_rejects_when_any_recipient_queue_is_full() {
        let hub = WebSocketHub::with_config(WebSocketHubConfig::new().outbound_queue_capacity(1));

        let (alice_id, mut alice_rx) = hub.register(Some(auth_user("alice"))).await;
        let _ = alice_rx.recv().await;

        let (bob_id, _bob_rx) = hub.register(Some(auth_user("bob"))).await;

        hub.join_group(&alice_id, "team.dev").await.unwrap();
        hub.join_group(&bob_id, "team.dev").await.unwrap();

        let error = hub
            .emit_to_group(
                &alice_id,
                "team.dev",
                "message.new",
                serde_json::json!({"text": "hello"}),
            )
            .await
            .unwrap_err();

        assert!(matches!(error, WebSocketError::OutboundQueueFull { .. }));
    }

    #[tokio::test]
    async fn group_name_with_surrounding_whitespace_is_rejected() {
        let hub = WebSocketHub::new();
        let (connection_id, mut rx) = hub.register(Some(auth_user("alice"))).await;
        let _ = rx.recv().await;

        let error = hub
            .join_group(&connection_id, " team.dev ")
            .await
            .unwrap_err();
        assert!(matches!(error, WebSocketError::InvalidGroup));
    }

    #[tokio::test]
    async fn event_name_with_surrounding_whitespace_is_rejected() {
        let hub = WebSocketHub::new();
        let (connection_id, mut rx) = hub.register(Some(auth_user("alice"))).await;
        let _ = rx.recv().await;
        hub.join_group(&connection_id, "team.dev").await.unwrap();

        let error = hub
            .emit_to_group(
                &connection_id,
                "team.dev",
                " message.new ",
                serde_json::json!({"text": "hello"}),
            )
            .await
            .unwrap_err();
        assert!(matches!(error, WebSocketError::InvalidEvent));
    }

    #[test]
    fn detect_msgpack_subprotocol_as_binary_format() {
        let mut headers = HeaderMap::new();
        headers.insert(
            SEC_WEBSOCKET_PROTOCOL,
            HeaderValue::from_static("json,msgpack"),
        );

        assert_eq!(
            detect_initial_frame_format(&headers),
            WebSocketFrameFormat::BinaryMessagePack
        );
    }

    #[tokio::test]
    async fn process_client_binary_frame_supports_messagepack_messages() {
        let hub = WebSocketHub::new();
        let (connection_id, mut rx) = hub.register(Some(auth_user("alice"))).await;
        let _ = rx.recv().await;

        let join_payload = rmp_serde::to_vec_named(&ClientMessage::Join {
            group: "binary.room".to_string(),
        })
        .expect("messagepack payload should encode");
        process_client_binary_frame(&hub, &connection_id, &join_payload).await;

        let ack = rx.recv().await.expect("joined ack should arrive");
        assert!(matches!(ack, ServerMessage::Joined { group } if group == "binary.room"));
    }

    #[tokio::test]
    async fn anonymous_registration_sets_none_auth_actor() {
        let hub = WebSocketHub::new();
        let (_connection_id, mut rx) = hub.register(None).await;

        match rx.recv().await.unwrap() {
            ServerMessage::Connected { actor, .. } => {
                assert_eq!(actor.auth_type, "none");
                assert_eq!(actor.username, "anonymous");
            }
            _ => panic!("expected connected message"),
        }
    }
}
