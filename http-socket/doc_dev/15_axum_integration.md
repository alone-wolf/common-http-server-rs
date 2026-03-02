# Axum 集成方案

本文件定义 `http-socket` 作为插件接入已有 axum 应用的方式。

## 1. Feature 开关

- 开启 `axum` feature 后会自动启用：
  - `tower`
  - `server`
- 依赖：`axum`、`tower`、`http`

## 2. 核心类型

- `HttpSocketAxumLayer`：可挂载到 `Router::layer(...)` 的中间层。
- `AxumRequestContext`：传给 `MiddlewareHook` 的请求上下文（method/path）。
- `AxumMiddlewareError`：中间层 hook 失败后注入到请求扩展的错误对象。
- `RouterHttpSocketExt`：`Router` 扩展方法，支持 `.with_http_socket(layer)`。

## 3. 接入方式

```rust
use axum::{routing::get, Router};
use http_socket::HttpSocketAxumLayer;
use http_socket::RouterHttpSocketExt;

let app = Router::new()
    .route("/socket/connect", get(|| async { "ok" }))
    .with_http_socket(HttpSocketAxumLayer::new());
```

完整 C/S 示例可参考：
- `examples/axum_http_socket_server.rs`
- `examples/http_socket_client.rs`

两者组合可演示 `http-socket` 协商/会话与 axum server/client 请求链路。

## 4. 与扩展系统关系

- `HttpSocketAxumLayer` 使用 `MiddlewareHook<AxumRequestContext>`。
- hook 返回错误时，不直接短路请求，而是注入 `AxumMiddlewareError`。
- 上层 handler 可通过 `Option<Extension<AxumMiddlewareError>>` 读取并决定响应策略。

## 5. 设计意图

- 保持与现有 axum app 的低侵入集成（仅 layer 挂载）。
- 核心协商/session 系统保持独立，不与 Web 路由强耦合。
- 错误处理策略交由应用层决定，避免框架代替业务做策略裁决。
