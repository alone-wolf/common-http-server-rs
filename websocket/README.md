# websocket

WebSocket group/event module extracted from `common-http-server-rs`.

## Features

- `server`: Axum WebSocket server, group management, broadcast, auth integration
- `client`: async websocket client wrapper (text JSON + binary MessagePack)
- `full`: enables both `server` and `client`

## Quick Add

```toml
[dependencies]
websocket = { git = "https://github.com/alone-wolf/common-http-server-rs.git", package = "websocket", branch = "main", default-features = false, features = ["server"] }
```

## Client Init (Chain Style)

```rust
use websocket::WebSocketClient;

let mut client = WebSocketClient::builder("ws://127.0.0.1:3006/realtime/ws")
    .with_api_key_auth("dev-api-key-1")
    .with_binary_messagepack()
    .connect()
    .await?;
```
