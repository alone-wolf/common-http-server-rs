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

client
    .emit_direct("<target-connection-id>", "direct.notice", serde_json::json!({"text":"hello"}))
    .await?;
```

连接帧格式在握手阶段确定：
- 服务端支持子协议：`chs.v1.msgpack` / `msgpack` / `chs.v1.json` / `json`（优先 MessagePack）。
- `.with_binary_messagepack()`（等价 `.force_msgpack()`）会声明 `chs.v1.msgpack, msgpack`，连接必须协商到 MessagePack。
- `.prefer_msgpack()` 会声明 `chs.v1.msgpack, msgpack, chs.v1.json, json`，优先 MessagePack，协商失败时可回落 JSON。
- 默认/`.with_text_json()`（等价 `.force_json()`）使用 JSON 子协议。
- 若运行时帧类型与协商格式不一致，服务端会返回 `frame_format_mismatch` 并关闭连接。
