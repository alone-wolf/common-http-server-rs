use crate::protocol::{
    ClientMessage, ServerMessage, WS_SUBPROTOCOL_JSON_V1, WS_SUBPROTOCOL_MSGPACK_V1,
    is_json_subprotocol, is_msgpack_subprotocol,
};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use futures_util::{SinkExt, StreamExt};
use http::{HeaderValue, Request, header::AUTHORIZATION, header::SEC_WEBSOCKET_PROTOCOL};
use serde_json::Value;
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async};

#[derive(Debug, thiserror::Error)]
pub enum WebSocketClientError {
    #[error("websocket transport error: {0}")]
    Transport(#[from] tokio_tungstenite::tungstenite::Error),
    #[error("json serialize/deserialize error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("messagepack encode error: {0}")]
    MsgPackEncode(#[from] rmp_serde::encode::Error),
    #[error("messagepack decode error: {0}")]
    MsgPackDecode(#[from] rmp_serde::decode::Error),
    #[error("request build error: {0}")]
    RequestBuild(#[from] http::Error),
    #[error("invalid header value: {0}")]
    InvalidHeaderValue(#[from] http::header::InvalidHeaderValue),
    #[error("unsupported negotiated websocket subprotocol: {0}")]
    UnsupportedNegotiatedSubprotocol(String),
    #[error(
        "websocket subprotocol negotiation mismatch: expected {expected}, negotiated {negotiated}"
    )]
    SubprotocolNegotiationMismatch {
        expected: &'static str,
        negotiated: String,
    },
    #[error(
        "websocket subprotocol negotiation mismatch: expected {expected}, but server did not select a subprotocol"
    )]
    MissingNegotiatedSubprotocol { expected: &'static str },
    #[error("websocket connection closed")]
    ConnectionClosed,
}

pub struct WebSocketClient {
    stream: WebSocketStream<MaybeTlsStream<TcpStream>>,
    frame_format: WebSocketFrameFormat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WebSocketFrameFormat {
    #[default]
    TextJson,
    BinaryMessagePack,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum WebSocketProtocolPreference {
    #[default]
    ForceJson,
    ForceMsgpack,
    PreferMsgpack,
}

#[derive(Debug, Clone, Default)]
enum WebSocketClientAuth {
    #[default]
    None,
    Bearer(String),
    Basic {
        username: String,
        password: String,
    },
}

#[derive(Debug, Clone)]
pub struct WebSocketClientBuilder {
    url: String,
    auth: WebSocketClientAuth,
    protocol_preference: WebSocketProtocolPreference,
}

impl WebSocketClient {
    pub fn builder(url: impl Into<String>) -> WebSocketClientBuilder {
        WebSocketClientBuilder {
            url: url.into(),
            auth: WebSocketClientAuth::None,
            protocol_preference: WebSocketProtocolPreference::ForceJson,
        }
    }

    pub async fn connect(url: &str) -> Result<Self, WebSocketClientError> {
        Self::builder(url).connect().await
    }

    pub async fn connect_with_api_key(
        url: &str,
        api_key: &str,
    ) -> Result<Self, WebSocketClientError> {
        Self::builder(url)
            .with_api_key_auth(api_key)
            .connect()
            .await
    }

    pub async fn send(&mut self, message: &ClientMessage) -> Result<(), WebSocketClientError> {
        let frame = encode_client_message(message, self.frame_format)?;
        self.stream.send(frame).await?;
        Ok(())
    }

    pub async fn recv(&mut self) -> Result<ServerMessage, WebSocketClientError> {
        while let Some(frame) = self.stream.next().await {
            match frame? {
                Message::Text(text) => return Ok(serde_json::from_str(text.as_ref())?),
                Message::Binary(data) => {
                    return rmp_serde::from_slice(&data).map_err(WebSocketClientError::from);
                }
                Message::Close(_) => return Err(WebSocketClientError::ConnectionClosed),
                Message::Ping(_) | Message::Pong(_) | Message::Frame(_) => continue,
            }
        }

        Err(WebSocketClientError::ConnectionClosed)
    }

    pub async fn join_group(
        &mut self,
        group: impl Into<String>,
    ) -> Result<(), WebSocketClientError> {
        self.send(&ClientMessage::Join {
            group: group.into(),
        })
        .await
    }

    pub async fn leave_group(
        &mut self,
        group: impl Into<String>,
    ) -> Result<(), WebSocketClientError> {
        self.send(&ClientMessage::Leave {
            group: group.into(),
        })
        .await
    }

    pub async fn emit_event(
        &mut self,
        group: impl Into<String>,
        event: impl Into<String>,
        payload: Value,
    ) -> Result<(), WebSocketClientError> {
        self.send(&ClientMessage::Event {
            group: group.into(),
            event: event.into(),
            payload,
        })
        .await
    }

    pub async fn emit_direct(
        &mut self,
        to_connection_id: impl Into<String>,
        event: impl Into<String>,
        payload: Value,
    ) -> Result<(), WebSocketClientError> {
        self.send(&ClientMessage::Direct {
            to_connection_id: to_connection_id.into(),
            event: event.into(),
            payload,
        })
        .await
    }

    pub async fn ping(&mut self, nonce: Option<String>) -> Result<(), WebSocketClientError> {
        self.send(&ClientMessage::Ping { nonce }).await
    }

    pub async fn close(mut self) -> Result<(), WebSocketClientError> {
        self.stream.close(None).await?;
        Ok(())
    }
}

impl WebSocketClientBuilder {
    fn with_auth(mut self, auth: WebSocketClientAuth) -> Self {
        self.auth = auth;
        self
    }

    pub fn with_bearer_auth(self, token: impl Into<String>) -> Self {
        self.with_auth(WebSocketClientAuth::Bearer(token.into()))
    }

    pub fn with_api_key_auth(self, api_key: impl Into<String>) -> Self {
        self.with_bearer_auth(api_key)
    }

    pub fn with_jwt_auth(self, jwt: impl Into<String>) -> Self {
        self.with_bearer_auth(jwt)
    }

    pub fn with_basic_auth(self, username: impl Into<String>, password: impl Into<String>) -> Self {
        self.with_auth(WebSocketClientAuth::Basic {
            username: username.into(),
            password: password.into(),
        })
    }

    pub fn without_auth(self) -> Self {
        self.with_auth(WebSocketClientAuth::None)
    }

    pub fn with_frame_format(mut self, format: WebSocketFrameFormat) -> Self {
        self.protocol_preference = match format {
            WebSocketFrameFormat::TextJson => WebSocketProtocolPreference::ForceJson,
            WebSocketFrameFormat::BinaryMessagePack => WebSocketProtocolPreference::ForceMsgpack,
        };
        self
    }

    pub fn with_binary_messagepack(self) -> Self {
        self.force_msgpack()
    }

    pub fn with_text_json(self) -> Self {
        self.force_json()
    }

    pub fn force_msgpack(mut self) -> Self {
        self.protocol_preference = WebSocketProtocolPreference::ForceMsgpack;
        self
    }

    pub fn force_json(mut self) -> Self {
        self.protocol_preference = WebSocketProtocolPreference::ForceJson;
        self
    }

    pub fn prefer_msgpack(mut self) -> Self {
        self.protocol_preference = WebSocketProtocolPreference::PreferMsgpack;
        self
    }

    pub async fn connect(self) -> Result<WebSocketClient, WebSocketClientError> {
        let request = self.build_request()?;
        let (stream, response) = connect_async(request).await?;
        let negotiated_subprotocol = response
            .headers()
            .get(SEC_WEBSOCKET_PROTOCOL)
            .and_then(|value| value.to_str().ok());
        let frame_format = resolve_frame_format(self.protocol_preference, negotiated_subprotocol)?;
        Ok(WebSocketClient {
            stream,
            frame_format,
        })
    }

    fn build_request(&self) -> Result<Request<()>, WebSocketClientError> {
        let mut request = self.url.as_str().into_client_request()?;
        if let Some(authorization) = self.auth.authorization_header_value() {
            let header_value = HeaderValue::from_str(&authorization)?;
            request.headers_mut().insert(AUTHORIZATION, header_value);
        }
        request.headers_mut().insert(
            SEC_WEBSOCKET_PROTOCOL,
            HeaderValue::from_static(self.protocol_preference.request_subprotocols_header()),
        );
        Ok(request)
    }
}

impl WebSocketProtocolPreference {
    fn request_subprotocols_header(self) -> &'static str {
        match self {
            WebSocketProtocolPreference::ForceJson => "chs.v1.json, json",
            WebSocketProtocolPreference::ForceMsgpack => "chs.v1.msgpack, msgpack",
            WebSocketProtocolPreference::PreferMsgpack => {
                "chs.v1.msgpack, msgpack, chs.v1.json, json"
            }
        }
    }
}

fn resolve_frame_format(
    preference: WebSocketProtocolPreference,
    negotiated_subprotocol: Option<&str>,
) -> Result<WebSocketFrameFormat, WebSocketClientError> {
    let negotiated_format = match negotiated_subprotocol {
        Some(token) if is_msgpack_subprotocol(token) => {
            Some(WebSocketFrameFormat::BinaryMessagePack)
        }
        Some(token) if is_json_subprotocol(token) => Some(WebSocketFrameFormat::TextJson),
        Some(token) => {
            return Err(WebSocketClientError::UnsupportedNegotiatedSubprotocol(
                token.to_string(),
            ));
        }
        None => None,
    };

    match preference {
        WebSocketProtocolPreference::ForceMsgpack => match negotiated_format {
            Some(WebSocketFrameFormat::BinaryMessagePack) => {
                Ok(WebSocketFrameFormat::BinaryMessagePack)
            }
            Some(WebSocketFrameFormat::TextJson) => {
                Err(WebSocketClientError::SubprotocolNegotiationMismatch {
                    expected: WS_SUBPROTOCOL_MSGPACK_V1,
                    negotiated: negotiated_subprotocol
                        .unwrap_or(WS_SUBPROTOCOL_JSON_V1)
                        .to_string(),
                })
            }
            None => Err(WebSocketClientError::MissingNegotiatedSubprotocol {
                expected: WS_SUBPROTOCOL_MSGPACK_V1,
            }),
        },
        WebSocketProtocolPreference::ForceJson => match negotiated_format {
            Some(WebSocketFrameFormat::BinaryMessagePack) => {
                Err(WebSocketClientError::SubprotocolNegotiationMismatch {
                    expected: WS_SUBPROTOCOL_JSON_V1,
                    negotiated: negotiated_subprotocol
                        .unwrap_or(WS_SUBPROTOCOL_MSGPACK_V1)
                        .to_string(),
                })
            }
            Some(WebSocketFrameFormat::TextJson) | None => Ok(WebSocketFrameFormat::TextJson),
        },
        WebSocketProtocolPreference::PreferMsgpack => {
            Ok(negotiated_format.unwrap_or(WebSocketFrameFormat::TextJson))
        }
    }
}

fn encode_client_message(
    message: &ClientMessage,
    format: WebSocketFrameFormat,
) -> Result<Message, WebSocketClientError> {
    match format {
        WebSocketFrameFormat::TextJson => {
            let payload = serde_json::to_string(message)?;
            Ok(Message::Text(payload.into()))
        }
        WebSocketFrameFormat::BinaryMessagePack => {
            let payload = rmp_serde::to_vec_named(message)?;
            Ok(Message::Binary(payload.into()))
        }
    }
}

impl WebSocketClientAuth {
    fn authorization_header_value(&self) -> Option<String> {
        match self {
            WebSocketClientAuth::None => None,
            WebSocketClientAuth::Bearer(token) => Some(format!("Bearer {}", token)),
            WebSocketClientAuth::Basic { username, password } => {
                let raw = format!("{}:{}", username, password);
                let encoded = STANDARD.encode(raw.as_bytes());
                Some(format!("Basic {}", encoded))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn api_key_auth_sets_bearer_header() {
        let request = WebSocketClient::builder("ws://localhost:3000/ws")
            .with_api_key_auth("dev-api-key")
            .build_request()
            .expect("request should be built");

        assert_eq!(
            request
                .headers()
                .get(AUTHORIZATION)
                .and_then(|v| v.to_str().ok()),
            Some("Bearer dev-api-key")
        );
    }

    #[test]
    fn basic_auth_sets_basic_header() {
        let request = WebSocketClient::builder("ws://localhost:3000/ws")
            .with_basic_auth("alice", "secret")
            .build_request()
            .expect("request should be built");

        assert_eq!(
            request
                .headers()
                .get(AUTHORIZATION)
                .and_then(|v| v.to_str().ok()),
            Some("Basic YWxpY2U6c2VjcmV0")
        );
    }

    #[test]
    fn without_auth_clears_authorization_header() {
        let request = WebSocketClient::builder("ws://localhost:3000/ws")
            .with_api_key_auth("dev-api-key")
            .without_auth()
            .build_request()
            .expect("request should be built");

        assert!(request.headers().get(AUTHORIZATION).is_none());
    }

    #[test]
    fn binary_messagepack_sets_subprotocol_header() {
        let request = WebSocketClient::builder("ws://localhost:3000/ws")
            .with_binary_messagepack()
            .build_request()
            .expect("request should be built");

        assert_eq!(
            request
                .headers()
                .get(SEC_WEBSOCKET_PROTOCOL)
                .and_then(|v| v.to_str().ok()),
            Some("chs.v1.msgpack, msgpack")
        );
    }

    #[test]
    fn prefer_msgpack_sets_fallback_subprotocol_header() {
        let request = WebSocketClient::builder("ws://localhost:3000/ws")
            .prefer_msgpack()
            .build_request()
            .expect("request should be built");

        assert_eq!(
            request
                .headers()
                .get(SEC_WEBSOCKET_PROTOCOL)
                .and_then(|v| v.to_str().ok()),
            Some("chs.v1.msgpack, msgpack, chs.v1.json, json")
        );
    }

    #[test]
    fn resolve_frame_format_for_prefer_msgpack_legacy_token() {
        let format = resolve_frame_format(
            WebSocketProtocolPreference::PreferMsgpack,
            Some(crate::protocol::WS_SUBPROTOCOL_MSGPACK_LEGACY),
        )
        .expect("subprotocol should map");

        assert_eq!(format, WebSocketFrameFormat::BinaryMessagePack);
    }

    #[test]
    fn resolve_frame_format_for_prefer_msgpack_without_negotiation_falls_back_to_json() {
        let format = resolve_frame_format(WebSocketProtocolPreference::PreferMsgpack, None)
            .expect("fallback should be json");

        assert_eq!(format, WebSocketFrameFormat::TextJson);
    }

    #[test]
    fn resolve_frame_format_force_msgpack_requires_negotiated_msgpack() {
        let error = resolve_frame_format(WebSocketProtocolPreference::ForceMsgpack, None)
            .expect_err("missing subprotocol should fail");

        assert!(matches!(
            error,
            WebSocketClientError::MissingNegotiatedSubprotocol { .. }
        ));
    }

    #[test]
    fn resolve_frame_format_rejects_unknown_negotiated_subprotocol() {
        let error = resolve_frame_format(
            WebSocketProtocolPreference::PreferMsgpack,
            Some("custom.v1"),
        )
        .expect_err("unknown subprotocol should fail");

        assert!(matches!(
            error,
            WebSocketClientError::UnsupportedNegotiatedSubprotocol(_)
        ));
    }

    #[test]
    fn encode_client_message_as_binary_frame() {
        let frame = encode_client_message(
            &ClientMessage::Ping {
                nonce: Some("n1".to_string()),
            },
            WebSocketFrameFormat::BinaryMessagePack,
        )
        .expect("frame should be built");

        match frame {
            Message::Binary(data) => {
                let decoded: ClientMessage =
                    rmp_serde::from_slice(&data).expect("binary should decode");
                match decoded {
                    ClientMessage::Ping { nonce } => assert_eq!(nonce.as_deref(), Some("n1")),
                    _ => panic!("unexpected decoded message"),
                }
            }
            _ => panic!("expected binary frame"),
        }
    }

    #[test]
    fn encode_direct_message_as_text_frame() {
        let frame = encode_client_message(
            &ClientMessage::Direct {
                to_connection_id: "conn-2".to_string(),
                event: "direct.notice".to_string(),
                payload: serde_json::json!({"text":"hello"}),
            },
            WebSocketFrameFormat::TextJson,
        )
        .expect("frame should be built");

        match frame {
            Message::Text(text) => {
                let decoded: ClientMessage =
                    serde_json::from_str(text.as_ref()).expect("text should decode");
                assert!(matches!(
                    decoded,
                    ClientMessage::Direct {
                        to_connection_id,
                        event,
                        ..
                    } if to_connection_id == "conn-2" && event == "direct.notice"
                ));
            }
            _ => panic!("expected text frame"),
        }
    }
}
