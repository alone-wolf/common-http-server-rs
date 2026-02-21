# common-http-server-rs

一个基于 Axum 的通用 HTTP 服务骨架，提供可复用的：
- 服务启动与配置（`ServerConfig` / `AppConfig` / `AppBuilder` / `Server`）
- 认证与授权（Basic / API Key / JWT）
- 防护中间件（DDoS / IP Filter / Rate Limit / Size Limit）
- 监控与健康检查（Prometheus 指标、性能中间件、健康探针）
- 可选运行时终端 UI（TUI）

> WebSocket 能力已拆分到 workspace 子 crate：`websocket`。

## 快速开始

### 1) 运行默认应用

```bash
cargo run
```

### 2) 运行示例

```bash
cargo run --example level1_basic
cargo run --example level2_app_config
cargo run --example level3_security_and_monitoring
cargo run --example level4_graceful_shutdown
cargo run --example level5_terminal_ui
cargo run --example jwt_with_client --features external-health

# WebSocket 示例（workspace 子 crate）
cargo run -p websocket --example websocket_group_events
```

## 通过 Git 引入依赖（完整写法）

本仓库可被其他项目以两种方式引入：
- 主 crate：`common-http-server-rs`
- workspace 子 crate：`websocket`

### Cargo.toml 可配置项说明

- `git`：Git 仓库地址（必填）
- `branch` / `tag` / `rev`：版本定位（3 选 1）
- `package`：当仓库中有多个 package 时指定目标包名（引入 `websocket` 时必填）
- `features`：启用功能开关
- `default-features`：是否启用默认 feature
- `optional`：作为可选依赖引入

> 推荐优先使用 `tag` 或 `rev`，避免长期跟踪 `branch = "main"` 带来的不确定变更。  
> 若仓库尚未创建发布 tag，请先使用 `branch` 或 `rev`，发布后再切换到 `tag`。

### 1) 仅引入主 crate

```toml
[dependencies]
common-http-server-rs = { git = "https://github.com/alone-wolf/common-http-server-rs.git", branch = "main" }
```

启用健康检查相关 feature：

```toml
[dependencies]
common-http-server-rs = { git = "https://github.com/alone-wolf/common-http-server-rs.git", branch = "main", features = ["full-health"] }
```

### 2) 仅引入 websocket 子 crate

`websocket` 默认 feature 为 `full`（同时启用 `server` + `client`）。

只启用 client（常见于调用方）：

```toml
[dependencies]
websocket = { git = "https://github.com/alone-wolf/common-http-server-rs.git", package = "websocket", branch = "main", default-features = false, features = ["client"] }
```

只启用 server：

```toml
[dependencies]
websocket = { git = "https://github.com/alone-wolf/common-http-server-rs.git", package = "websocket", branch = "main", default-features = false, features = ["server"] }
```

### 3) 同时引入主 crate + websocket（并重命名依赖）

```toml
[dependencies]
common_http = { package = "common-http-server-rs", git = "https://github.com/alone-wolf/common-http-server-rs.git", branch = "main", features = ["full-health"] }
common_ws = { package = "websocket", git = "https://github.com/alone-wolf/common-http-server-rs.git", branch = "main", default-features = false, features = ["client"] }
```

### 4) 使用 commit 锁定（rev）

```toml
[dependencies]
common-http-server-rs = { git = "https://github.com/alone-wolf/common-http-server-rs.git", rev = "COMMIT_SHA" }
websocket = { git = "https://github.com/alone-wolf/common-http-server-rs.git", package = "websocket", rev = "COMMIT_SHA", default-features = false, features = ["server"] }
```

### 5) 跟踪开发分支（branch）

```toml
[dependencies]
common-http-server-rs = { git = "https://github.com/alone-wolf/common-http-server-rs.git", branch = "main" }
```

### 6) 作为可选依赖（optional）

```toml
[dependencies]
common-http-server-rs = { git = "https://github.com/alone-wolf/common-http-server-rs.git", branch = "main", optional = true }
websocket = { git = "https://github.com/alone-wolf/common-http-server-rs.git", package = "websocket", branch = "main", default-features = false, features = ["client"], optional = true }

[features]
with-http = ["dep:common-http-server-rs"]
with-ws-client = ["dep:websocket"]
```

### 代码导入方式

crate 名中的 `-` 在代码里会变为 `_`：

```rust
use common_http_server_rs::{AppBuilder, AppConfig, Server, ServerConfig};
use websocket::WebSocketClient;
```

## 项目结构

- `src/`：库与默认二进制入口
- `examples/`：分层示例（从基础到安全/监控/UI）
- `websocket/`：WebSocket 子 crate（feature: server/client/full）
- `doc/`：详细指南（认证、防护、监控、CORS、安全）
- `Prompts.md`：面向 AI 的提示词模板与任务脚手架

## 文档入口

建议从这里开始阅读：
- `doc/README.md`
- `doc/SAMPLES.md`
- `doc/AUTH_GUIDE.md`
- `doc/PROTECTION_GUIDE.md`
- `doc/MONITORING_GUIDE.md`
- `doc/WEBSOCKET_GUIDE.md`
- `doc/SECURITY_NOTES.md`

## 给 AI/Agent 的使用说明（重点）

如果你使用 AI 协助改这个仓库，请优先引用：
- [`Prompts.md`](./Prompts.md)

`Prompts.md` 中包含：
- 推荐系统提示词（约束 AI 只用现有 API）
- 常见任务模板（新增接口、接认证、接防护、接监控、改文档）
- 事实清单（入口文件、示例、默认健康路由）
- 标准验证命令

## 常用验证命令

```bash
cargo fmt --all -- --check
cargo check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
cargo check --examples --all-features
```

## Feature Flags

`common-http-server-rs`：
- `database-health`：启用数据库健康检查能力（`sqlx`）
- `redis-health`：启用 Redis 健康检查能力（`redis`）
- `external-health`：启用外部服务健康检查能力（`reqwest`）
- `full-health`：一次性启用全部健康检查能力

`websocket`：
- `server`：WebSocket server/group/event 能力（依赖主 crate 的认证中间件）
- `client`：异步 WebSocket JSON 客户端
- `full`：默认开启（`server` + `client`）
