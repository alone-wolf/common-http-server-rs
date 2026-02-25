# Documentation Index

`common-http-server-rs` 文档入口与推荐阅读顺序：

1. `SAMPLES.md`  
   先看可运行示例，快速了解能力边界。
2. `AUTH_GUIDE.md`  
   认证与授权（Basic / API Key / JWT）。
3. `PROTECTION_GUIDE.md`  
   防护中间件（DDoS / IP Filter / Rate Limit / Size Limit）。
4. `MONITORING_GUIDE.md`  
   监控、指标、健康检查。
5. `WEBSOCKET_GUIDE.md`  
   WebSocket 实时通信（JSON 文本帧 + MessagePack 二进制帧、group/event、auth 鉴权）。
6. `HTTP_PANEL_GUIDE.md`  
   HTTP 面板接入（HTTP 统计与 WebSocket inspection）。
7. `CORS_GUIDE.md`  
   跨域配置与常见误区。
8. `SECURITY_NOTES.md`  
   生产安全基线与硬化建议（强烈建议上线前阅读）。
9. `PUBLISHING.md`  
   发布流程、检查清单、Git 与 crates.io 发布策略。
10. `FRAMEWORK_ROADMAP.md`  
   框架发展路径与阶段能力矩阵。
11. `ARCHITECTURE_REDESIGN.md`  
   面向未来构想的模块重构与组合设计建议。

## Quick Run

```bash
# 基础运行
cargo run -p common-http-server-rs

# 示例
cargo run -p common-http-server-rs --example level1_basic
cargo run -p common-http-server-rs --example level2_app_config
cargo run -p common-http-server-rs --example level3_security_and_monitoring
cargo run -p common-http-server-rs --example level4_graceful_shutdown
cargo run -p common-http-server-rs --example level5_terminal_ui
cargo run -p common-http-server-rs --example level6_websocket_http_panel
cargo run -p common-http-server-rs --example jwt_with_client --features external-health

# WebSocket 示例（workspace 子 crate）
cargo run -p websocket --example websocket_group_events
cargo run -p websocket --example websocket_cs_state_dashboard
```

## Notes

- 文档中的端口、路由以示例代码为准；如果示例更新，请同步更新文档。
- WebSocket 相关代码位于 workspace 子 crate `websocket`。
- HTTP 面板相关代码位于 workspace 子 crate `http-panel`。
- 全局中间件推荐使用 `MiddlewareOrchestrator`（见 `examples/level6_websocket_http_panel.rs`）。
- `internal/repair.md` 是历史修复记录，不代表当前代码状态。
- `internal/DEVELOPMENT_PLAN.md` 是内部开发计划，不作为依赖使用指南。
