# HTTP Panel Guide

`http-panel` 是 workspace 子 crate，用于给 HTTP 服务挂载一个轻量 Web 界面：

- 查看 HTTP 统计信息（请求总量、错误率、速率、活动连接）
- 输出原始 JSON snapshot 便于调试
- 可选接入 `websocket` crate 的 server inspection

## 依赖

```toml
[dependencies]
common-http-server-rs = { git = "https://github.com/alone-wolf/common-http-server-rs.git", branch = "main" }
websocket = { git = "https://github.com/alone-wolf/common-http-server-rs.git", package = "websocket", branch = "main", default-features = false, features = ["server"] }
http_panel = { git = "https://github.com/alone-wolf/common-http-server-rs.git", package = "http-panel", branch = "main" }
```

## 快速接入

```rust
use axum::Router;
use common_http_server_rs::MonitoringState;
use http_panel::{HttpPanelConfig, HttpPanelState, panel_routes};
use websocket::{WebSocketHub, websocket_router};

let monitoring = MonitoringState::new();
let ws_hub = WebSocketHub::new();

let panel_state = HttpPanelState::new(monitoring)
    .with_websocket_hub(ws_hub.clone())
    .with_config(
        HttpPanelConfig::new()
            .title("Ops Panel")
            .refresh_interval_ms(1500)
            .show_raw_snapshot(true),
    );

let app = Router::new()
    .merge(websocket_router("/ws", ws_hub))
    .nest("/panel", panel_routes(panel_state));
```

## 面板路由

挂载到 `/panel` 后，会得到：

- `/panel`：HTML 页面（统一无尾 `/`）
- `/panel/api/snapshot`：HTTP + WebSocket inspection 组合快照
- `/panel/api/http`：HTTP 监控信息
- `/panel/api/websocket`：WebSocket inspection（未配置 hub 时返回 404）

## 面板交互能力

- 自动刷新 + 手动刷新
- 暂停/恢复刷新
- 在线调整刷新间隔（最小 250ms）
- WebSocket `groups` / `connections` 结构化表格展示

## 统计口径说明

- 默认会从请求计数中排除 `/panel` 与 `/monitor` 及其子路径，避免面板/监控自轮询放大业务请求统计。

## WebSocket Inspection 字段

`websocket` inspection 返回：

- `total_connections`：当前连接数
- `total_groups`：当前分组数
- `groups[]`：每个分组的成员统计
- `connections[]`：每个连接的身份与已加入分组
