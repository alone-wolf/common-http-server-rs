# WebSocket Group/Event Guide

`common-http-server-rs` 提供基于 JSON 的 WebSocket 实时能力，当前阶段支持：

- 连接鉴权（复用本包 auth 中间件：JWT / Basic / API Key）
- Group（加入/离开分组）
- Event（按 group 广播事件）

## Quick Start

```rust
use axum::Router;
use common_http_server_rs::{
    WebSocketAuthMode, WebSocketHub, auth_presets, websocket_router_with_auth,
};

let hub = WebSocketHub::new();
let auth = auth_presets::development().shared();

let ws_router = websocket_router_with_auth("/ws", hub, auth, WebSocketAuthMode::ApiKey);
let app = Router::new().merge(ws_router);
```

> 当前阶段推荐优先使用 `WebSocketAuthMode::ApiKey`。

## Client -> Server JSON 协议

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

## Server -> Client JSON 协议

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

- 仅支持文本帧（JSON），二进制帧会返回 `unsupported_frame` 错误。
- 出站消息队列为有界队列；当目标连接队列已满时，事件会被拒绝并返回 `outbound_queue_full`。
- `group` 和 `event` 名称限制：
  - 非空
  - 长度 <= 64
  - 仅允许 `[A-Za-z0-9_.:-]`

## 运行示例

```bash
cargo run --example websocket_group_events
```
