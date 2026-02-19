# Samples

`examples/` 下提供 6 个渐进式示例：

- `level1_basic.rs`  
  最小可运行启动链路（`ServerConfig + AppBuilder + Server`）。
- `level2_app_config.rs`  
  展示 `AppConfig` 细节（CORS / logging / tracing）与基础路由。
- `level3_security_and_monitoring.rs`  
  展示认证、角色控制、防护链路与监控端点组合。
- `level4_graceful_shutdown.rs`  
  展示优雅停机与在途请求处理。
- `level5_terminal_ui.rs`  
  展示可选 Terminal UI（ratatui + crossterm）实时状态/日志/动作事件通道。
- `jwt_with_client.rs`  
  端到端 JWT 登录 + 受保护 API + Rust 客户端调用流程。

## Run

```bash
cargo run -p common-http-server --example level1_basic
cargo run -p common-http-server --example level2_app_config
cargo run -p common-http-server --example level3_security_and_monitoring
cargo run -p common-http-server --example level4_graceful_shutdown
cargo run -p common-http-server --example level5_terminal_ui

# jwt_with_client 依赖 reqwest（通过 external-health feature 启用）
cargo run -p common-http-server --example jwt_with_client --features external-health
```

更多文档入口见 `doc/README.md`。
