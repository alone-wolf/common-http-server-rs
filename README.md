# common-http-server-rs

一个基于 Axum 的通用 HTTP 服务骨架，提供可复用的：
- 服务启动与配置（`ServerConfig` / `AppConfig` / `AppBuilder` / `Server`）
- 认证与授权（Basic / API Key / JWT）
- 防护中间件（DDoS / IP Filter / Rate Limit / Size Limit）
- 监控与健康检查（Prometheus 指标、性能中间件、健康探针）
- 可选运行时终端 UI（TUI）

> 扩展能力已拆分到 workspace 子 crate：`websocket`、`http-panel`。

## 快速开始

### 1) 运行默认应用

```bash
cargo run
```

### 2) 运行示例

完整示例运行命令请统一参考：`doc/SAMPLES.md`（唯一维护入口）。

## 全局中间件编排器（推荐）

`MiddlewareOrchestrator` 用于把监控、防护、全局鉴权与应用级 runtime layer（logging/tracing/cors）统一在一个地方挂载，避免子路由重复或漏挂。

```rust
use common_http_server_rs::{
    AppBuilder, AppConfig, GlobalAuthConfig, GlobalAuthMode, GlobalMonitoringConfig,
    MiddlewareOrchestrator, MonitoringState, PathScope, PerformanceMonitoringConfig, auth_presets,
};

let monitoring = MonitoringState::new();
let auth_config_a = auth_presets::development().shared();
let auth_config_b = auth_presets::development().shared();

let app_builder = AppBuilder::new(AppConfig::default())
    .route("/", axum::routing::get(|| async { "ok" }))
    .with_orchestrator(
        MiddlewareOrchestrator::new()
            .with_monitoring_config(
                monitoring.clone(),
                GlobalMonitoringConfig::new().with_performance_config(
                    PerformanceMonitoringConfig::new()
                        .exclude_request_count_path_prefix("/panel")
                        .exclude_request_count_path_prefix("/monitor"),
                ),
            )
            .with_auth_rules(vec![
                GlobalAuthConfig::new(auth_config_a.clone(), GlobalAuthMode::ApiKey)
                    .with_scope(PathScope::all().include_prefix("/api")),
                GlobalAuthConfig::new(auth_config_b.clone(), GlobalAuthMode::ApiKey)
                    .with_scope(PathScope::all().include_prefix("/realtime")),
            ]),
    );
```

## 通过 Git 引入依赖（完整写法）

本仓库可被其他项目以三种方式引入：
- 主 crate：`common-http-server-rs`
- workspace 子 crate：`websocket`
- workspace 子 crate：`http-panel`

### Cargo.toml 可配置项说明

- `git`：Git 仓库地址（必填）
- `branch` / `tag` / `rev`：版本定位（3 选 1）
- `package`：当仓库中有多个 package 时指定目标包名（引入 `websocket` / `http-panel` 时必填）
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

### 3) 引入 http-panel（并显式声明其上游类型依赖）

`http-panel` 的 `HttpPanelState` 通常会组合 `MonitoringState`（来自主 crate）和 `WebSocketHub`（来自 websocket crate），
因此实际接入时通常会同时声明这三个依赖：

```toml
[dependencies]
common-http-server-rs = { git = "https://github.com/alone-wolf/common-http-server-rs.git", branch = "main" }
websocket = { git = "https://github.com/alone-wolf/common-http-server-rs.git", package = "websocket", branch = "main", default-features = false, features = ["server"] }
http_panel = { git = "https://github.com/alone-wolf/common-http-server-rs.git", package = "http-panel", branch = "main" }
```

### 4) 同时引入主 crate + websocket（并重命名依赖）

```toml
[dependencies]
common_http = { package = "common-http-server-rs", git = "https://github.com/alone-wolf/common-http-server-rs.git", branch = "main", features = ["full-health"] }
common_ws = { package = "websocket", git = "https://github.com/alone-wolf/common-http-server-rs.git", branch = "main", default-features = false, features = ["client"] }
```

### 5) 使用 commit 锁定（rev）

```toml
[dependencies]
common-http-server-rs = { git = "https://github.com/alone-wolf/common-http-server-rs.git", rev = "COMMIT_SHA" }
websocket = { git = "https://github.com/alone-wolf/common-http-server-rs.git", package = "websocket", rev = "COMMIT_SHA", default-features = false, features = ["server"] }
http_panel = { git = "https://github.com/alone-wolf/common-http-server-rs.git", package = "http-panel", rev = "COMMIT_SHA" }
```

### 6) 跟踪开发分支（branch）

```toml
[dependencies]
common-http-server-rs = { git = "https://github.com/alone-wolf/common-http-server-rs.git", branch = "main" }
```

### 7) 作为可选依赖（optional）

```toml
[dependencies]
common-http-server-rs = { git = "https://github.com/alone-wolf/common-http-server-rs.git", branch = "main", optional = true }
websocket = { git = "https://github.com/alone-wolf/common-http-server-rs.git", package = "websocket", branch = "main", default-features = false, features = ["client"], optional = true }
http_panel = { git = "https://github.com/alone-wolf/common-http-server-rs.git", package = "http-panel", branch = "main", optional = true }

[features]
with-http = ["dep:common-http-server-rs"]
with-ws-client = ["dep:websocket"]
with-http-panel = ["dep:http_panel"]
```

### 代码导入方式

crate 名中的 `-` 在代码里会变为 `_`：

```rust
use common_http_server_rs::{AppBuilder, AppConfig, Server, ServerConfig};
use websocket::WebSocketClient;
use http_panel::panel_routes;
```

## 项目结构

- `src/`：库与默认二进制入口
- `examples/`：分层示例（从基础到安全/监控/UI）
- `websocket/`：WebSocket 子 crate（feature: server/client/full）
- `http-panel/`：HTTP 面板子 crate（监控面板 + WebSocket inspection）
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
- `doc/HTTP_PANEL_GUIDE.md`
- `doc/SECURITY_NOTES.md`
- `doc/PUBLISHING.md`
- `doc/FRAMEWORK_ROADMAP.md`
- `doc/ARCHITECTURE_REDESIGN.md`

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
- `client`：异步 WebSocket 客户端（JSON + MessagePack，含子协议协商）
- `full`：默认开启（`server` + `client`）

`http-panel`：
- 无 feature 开关；用于挂载 Web 面板路由（HTTP 统计 + 可选 WebSocket inspection）
