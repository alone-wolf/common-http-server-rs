# common-http-server-rs

一个基于 Axum 的通用 HTTP 服务骨架，提供可复用的：
- 服务启动与配置（`ServerConfig` / `AppConfig` / `AppBuilder` / `Server`）
- 认证与授权（Basic / API Key / JWT）
- 防护中间件（DDoS / IP Filter / Rate Limit / Size Limit）
- 监控与健康检查（Prometheus 指标、性能中间件、健康探针）
- 可选运行时终端 UI（TUI）

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
```

## 通过 Git 引入依赖

在你的项目 `Cargo.toml` 中添加：

```toml
[dependencies]
common-http-server-rs = { git = "https://github.com/alone-wolf/common-http-server-rs.git", branch = "main" }
```

如果你需要启用可选能力（例如完整健康检查），可以这样写：

```toml
[dependencies]
common-http-server-rs = { git = "https://github.com/alone-wolf/common-http-server-rs.git", branch = "main", features = ["full-health"] }
```

然后在代码里这样导入（crate 名会转为下划线）：

```rust
use common_http_server_rs::{AppBuilder, AppConfig, Server, ServerConfig};
```

如需锁定版本，建议改用 `tag` 或 `rev`（commit SHA）而不是 `branch`。

## 项目结构

- `src/`：库与默认二进制入口
- `examples/`：分层示例（从基础到安全/监控/UI）
- `doc/`：详细指南（认证、防护、监控、CORS、安全）
- `Prompts.md`：面向 AI 的提示词模板与任务脚手架

## 文档入口

建议从这里开始阅读：
- `doc/README.md`
- `doc/SAMPLES.md`
- `doc/AUTH_GUIDE.md`
- `doc/PROTECTION_GUIDE.md`
- `doc/MONITORING_GUIDE.md`
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

- `database-health`：启用数据库健康检查能力（`sqlx`）
- `redis-health`：启用 Redis 健康检查能力（`redis`）
- `external-health`：启用外部服务健康检查能力（`reqwest`）
- `full-health`：一次性启用全部健康检查能力
