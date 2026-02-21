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
   WebSocket 实时通信（JSON 协议、group/event、auth 鉴权）。
6. `CORS_GUIDE.md`  
   跨域配置与常见误区。
7. `SECURITY_NOTES.md`  
   生产安全基线与硬化建议（强烈建议上线前阅读）。

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
cargo run -p common-http-server-rs --example jwt_with_client --features external-health

# WebSocket 示例（workspace 子 crate）
cargo run -p websocket --example websocket_group_events
```

## Notes

- 文档中的端口、路由以示例代码为准；如果示例更新，请同步更新文档。
- WebSocket 相关代码位于 workspace 子 crate `websocket`。
- `internal/repair.md` 是历史修复记录，不代表当前代码状态。
- `internal/DEVELOPMENT_PLAN.md` 是内部开发计划，不作为依赖使用指南。
