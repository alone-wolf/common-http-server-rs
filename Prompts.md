# Prompts.md

这是一份给 AI 助手使用的提示词模板，目标是让 AI 在本仓库里稳定产出**可编译、风格一致、贴合现有 API** 的代码与文档。

## 1) 推荐系统提示词（System Prompt）

```text
你是本仓库的 Rust 代码助手。请严格基于当前代码实现，不要臆造 API。

仓库关键信息：
- workspace 项目：
  - 核心 crate：common-http-server-rs（导入路径 common_http_server_rs）
  - WebSocket crate：websocket（导入路径 websocket）
- 核心启动链路：AppBuilder -> Server::new(...) -> Server::start()
- 核心配置：ServerConfig, AppConfig, CorsConfig, LoggingConfig
- 认证模块：AuthConfig / basic_auth_middleware / api_key_auth_middleware / jwt_auth_middleware / require_roles / require_permissions
- 防护模块：ProtectionStackBuilder（DDoS/IPFilter/RateLimit/SizeLimit，按推荐顺序叠加）
- 监控模块：MonitoringState / setup_metrics_recorder / performance_monitoring_middleware / metrics_endpoint / monitoring_info_endpoint

编码规则：
1. 只使用仓库内已存在的公开 API（优先从 common_http_server_rs 顶层 re-export 导入）。
2. 代码必须可直接编译；补齐必要 use、错误处理与 trait 约束。
3. 优先沿用现有风格：tokio 多线程 runtime、axum Router、builder 链式配置。
4. 不引入无关依赖，不改动无关文件。
5. 涉及功能改动时，给出最小可运行示例或测试建议。

输出要求：
- 先给变更思路，再给代码。
- 明确指出修改文件路径。
- 最后给本地验证命令（cargo fmt/check/test/clippy）。
```

## 2) 推荐任务提示词模板（User Prompt Template）

```text
请在本仓库中完成以下任务，并严格使用对应 crate 的现有 API：

任务：<在这里写需求>

约束：
- 仅修改必要文件。
- 保持现有代码风格。
- 输出包含：
  1) 变更说明
  2) 具体改动文件
  3) 关键代码片段
  4) 验证命令
```

## 3) 场景化提示词

### A. 生成最小可运行服务

```text
请基于 common_http_server_rs 生成一个最小可运行 HTTP 服务：
- 使用 ServerConfig + AppConfig + AppBuilder + Server
- 新增 GET /hello 返回 JSON
- 保留默认健康检查路由
- 给出完整 main.rs 可编译代码
- 最后给运行命令
```

### B. 接入认证（Basic/JWT）

```text
请在现有 axum 路由中接入 common_http_server_rs 认证：
- 使用 AuthConfig 构建共享配置（shared）
- 演示 basic_auth_middleware 或 jwt_auth_middleware
- 在受保护路由上增加 require_roles(["admin", "user"])
- 返回 AuthUser 中的 username/roles
- 保证代码可编译并说明安全注意事项（JWT secret 长度、HTTPS policy）
```

### C. 接入防护栈

```text
请使用 ProtectionStackBuilder 为现有 Router 添加防护：
- 启用 ddos_presets::moderate()
- 启用 rate_limit_presets::api()
- 启用 size_limit_presets::api()（content-length only 模式）
- 展示如何用 AppBuilder::with_protection(...) 或 apply_to_router(...)
- 给出校验配置的写法（validate_*_config）
```

### D. 接入监控与指标

```text
请为服务接入 common_http_server_rs 监控能力：
- 创建 MonitoringState 并 setup_metrics_recorder
- 对业务路由添加 performance_monitoring_middleware
- 暴露 /monitor/metrics 与 /monitor/monitoring
- 说明如何在本地查看 Prometheus 文本指标
```

### E. 文档更新任务

```text
请检查并更新仓库文档，要求：
- 命令按所属 crate 书写（例如 `cargo run -p <package> --example ...`）
- 避免写入本机绝对路径
- 若发现历史快照内容，标注“可能过时”并给出最新参考文档
- 修改后给出受影响文件列表
```

### F. 接入 WebSocket（group/event + auth）

```text
请为服务接入 websocket 的 WebSocket 能力：
- 使用 WebSocketHub + websocket_router_with_auth（来自 websocket）
- 认证方式使用 WebSocketAuthMode::ApiKey（并复用 common_http_server_rs 的 auth）
- 实现/演示 group join/leave 与 event 广播
- 通信格式使用 JSON 文本帧
- 给出最小可运行示例及测试/验证步骤
```

## 4) AI 生成代码时的事实清单（可附加到任何提示词后）

```text
事实清单：
- 库入口：src/lib.rs（大量 API 已 re-export，可直接从 common_http_server_rs 导入）
- 核心示例目录：examples/
  - level1_basic.rs
  - level2_app_config.rs
  - level3_security_and_monitoring.rs
  - level4_graceful_shutdown.rs
  - level5_terminal_ui.rs
  - jwt_with_client.rs
- WebSocket 示例目录：websocket/examples/
  - websocket_group_events.rs
- 文档目录：doc/
- 默认健康路由由 AppBuilder::new(...) 自动提供：/health、/health/detailed、/api/v1/status
```

## 5) 建议验证命令（让 AI 在回答末尾附上）

```bash
cargo fmt --all -- --check
cargo check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
cargo check --examples --all-features
```
