use serde::{Deserialize, Serialize};
use serde_json::Value;

#[cfg(any(feature = "server", feature = "client"))]
pub const WS_SUBPROTOCOL_MSGPACK_V1: &str = "chs.v1.msgpack";
#[cfg(any(feature = "server", feature = "client"))]
pub const WS_SUBPROTOCOL_JSON_V1: &str = "chs.v1.json";
#[cfg(any(feature = "server", feature = "client"))]
pub const WS_SUBPROTOCOL_MSGPACK_LEGACY: &str = "msgpack";
#[cfg(any(feature = "server", feature = "client"))]
pub const WS_SUBPROTOCOL_JSON_LEGACY: &str = "json";

#[cfg(any(feature = "server", feature = "client"))]
pub fn is_msgpack_subprotocol(value: &str) -> bool {
    value.eq_ignore_ascii_case(WS_SUBPROTOCOL_MSGPACK_V1)
        || value.eq_ignore_ascii_case(WS_SUBPROTOCOL_MSGPACK_LEGACY)
}

#[cfg(any(feature = "server", feature = "client"))]
pub fn is_json_subprotocol(value: &str) -> bool {
    value.eq_ignore_ascii_case(WS_SUBPROTOCOL_JSON_V1)
        || value.eq_ignore_ascii_case(WS_SUBPROTOCOL_JSON_LEGACY)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventActor {
    pub user_id: String,
    pub username: String,
    pub auth_type: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    Join {
        group: String,
    },
    Leave {
        group: String,
    },
    Event {
        group: String,
        event: String,
        payload: Value,
    },
    Direct {
        to_connection_id: String,
        event: String,
        payload: Value,
    },
    Ping {
        nonce: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    Connected {
        connection_id: String,
        actor: EventActor,
    },
    Joined {
        group: String,
    },
    Left {
        group: String,
    },
    Event {
        group: String,
        event: String,
        payload: Value,
        from: EventActor,
        timestamp: String,
    },
    Direct {
        from_connection_id: String,
        event: String,
        payload: Value,
        from: EventActor,
        timestamp: String,
    },
    Pong {
        nonce: Option<String>,
    },
    Error {
        code: String,
        message: String,
    },
}
