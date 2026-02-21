use serde::{Deserialize, Serialize};
use serde_json::Value;

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
    Pong {
        nonce: Option<String>,
    },
    Error {
        code: String,
        message: String,
    },
}
