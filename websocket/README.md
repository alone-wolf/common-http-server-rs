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

## Minimal Usage

```rust
use websocket::WebSocketClient;

let mut client = WebSocketClient::builder("ws://127.0.0.1:3006/realtime/ws")
    .with_api_key_auth("dev-api-key-1")
    .prefer_msgpack()
    .connect()
    .await?;

client
    .emit_direct("<target-connection-id>", "direct.notice", serde_json::json!({"text":"hello"}))
    .await?;
```

服务端可通过 `hub.inspect()` 获取 inspection 快照：

```rust
let snapshot = hub.inspect().await;
println!("connections={}, groups={}", snapshot.total_connections, snapshot.total_groups);
```

## 详细文档

- 详细接入与协议说明：`doc/WEBSOCKET_GUIDE.md`
- 运行示例命令：`doc/SAMPLES.md`
