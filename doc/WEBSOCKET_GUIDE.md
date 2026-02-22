# WebSocket Group/Event Guide

`websocket` 提供基于结构化消息的 WebSocket 实时能力，当前阶段支持：

- 连接鉴权（复用本包 auth 中间件：JWT / Basic / API Key）
- Group（加入/离开分组）
- Event（按 group 广播事件）

## Feature 划分

- `server`：提供 Axum WebSocket 服务端能力（hub、group/event、auth 集成）
- `client`：提供异步 WebSocket 客户端封装（text JSON + binary MessagePack）
- `full`：同时启用 `server + client`

依赖示例：

```toml
[dependencies]
common-http-server-rs = { git = "https://github.com/alone-wolf/common-http-server-rs.git", branch = "main" }
websocket = { git = "https://github.com/alone-wolf/common-http-server-rs.git", package = "websocket", branch = "main", default-features = false, features = ["server"] }
```

## Quick Start

```rust
use axum::Router;
use common_http_server_rs::auth_presets;
use websocket::{WebSocketAuthMode, WebSocketHub, websocket_router_with_auth};

let hub = WebSocketHub::new();
let auth = auth_presets::development().shared();

let ws_router = websocket_router_with_auth("/ws", hub, auth, WebSocketAuthMode::ApiKey);
let app = Router::new().merge(ws_router);
```

> 当前阶段推荐优先使用 `WebSocketAuthMode::ApiKey`。

客户端可使用链式初始化：

```rust
use websocket::WebSocketClient;

let mut client = WebSocketClient::builder("ws://127.0.0.1:3006/realtime/ws")
    .with_api_key_auth("dev-api-key-1")
    .with_binary_messagepack()
    .connect()
    .await?;
```

## Client -> Server 消息结构

### 1) 加入分组

```json
{"type":"join","group":"chat.room"}
```

### 2) 离开分组

```json
{"type":"leave","group":"chat.room"}
```

### 3) 发送事件（广播到 group）

```json
{"type":"event","group":"chat.room","event":"message.new","payload":{"text":"hello"}}
```

### 4) 心跳

```json
{"type":"ping","nonce":"abc-123"}
```

## Server -> Client 消息结构

### connected

```json
{"type":"connected","connection_id":"...","actor":{"user_id":"...","username":"alice","auth_type":"api_key"}}
```

### joined / left

```json
{"type":"joined","group":"chat.room"}
{"type":"left","group":"chat.room"}
```

### event

```json
{
  "type":"event",
  "group":"chat.room",
  "event":"message.new",
  "payload":{"text":"hello"},
  "from":{"user_id":"...","username":"alice","auth_type":"api_key"},
  "timestamp":"2026-02-19T00:00:00Z"
}
```

### pong / error

```json
{"type":"pong","nonce":"abc-123"}
{"type":"error","code":"invalid_group","message":"invalid group name"}
```

## 认证说明

`websocket_router_with_auth` 会在 WebSocket upgrade 前执行 auth 中间件。

- JWT: `Authorization: Bearer <token>`
- API Key: `Authorization: Bearer <api_key>`（开发预设可用 `dev-api-key-1`）
- Basic: `Authorization: Basic <base64(username:password)>`
- None: `WebSocketAuthMode::None`（显式允许未认证连接）

鉴权成功后会将 `AuthUser` 注入请求扩展，WebSocket 会话直接复用它作为事件发送者身份。
若使用 `None`，连接会以匿名身份接入（`auth_type = "none"`）。

## 约束（当前阶段）

- 协议支持两种帧格式：
  - 文本帧：JSON
  - 二进制帧：MessagePack（结构化消息）
- 帧格式在握手阶段确定并在连接生命周期内保持一致：
  - 客户端声明 `Sec-WebSocket-Protocol: msgpack` 时使用二进制 MessagePack
  - 未声明时默认使用文本 JSON
  - 若连接收到与协商格式不一致的帧，会返回 `frame_format_mismatch`
- 出站消息队列为有界队列；当目标连接队列已满时，事件会被拒绝并返回 `outbound_queue_full`。
- `group` 和 `event` 名称限制：
  - 非空
  - 长度 <= 64
  - 仅允许 `[A-Za-z0-9_.:-]`

## 运行示例

```bash
cargo run -p websocket --example websocket_group_events
```
