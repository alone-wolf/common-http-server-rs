# http-panel

Web UI panel routes for `common-http-server-rs` monitoring stats and `websocket` hub inspection.

## Features

- Web dashboard page (auto refresh + manual refresh/pause + interval control)
- HTTP stats snapshot API
- Optional WebSocket server inspection API
- WebSocket groups/connections table view

## Quick Start

```rust
use axum::Router;
use common_http_server_rs::MonitoringState;
use http_panel::{HttpPanelConfig, HttpPanelState, panel_routes};
use websocket::{WebSocketHub, websocket_router};

let monitoring = MonitoringState::new();
let websocket_hub = WebSocketHub::new();

let panel_state = HttpPanelState::new(monitoring)
    .with_websocket_hub(websocket_hub.clone())
    .with_config(
        HttpPanelConfig::new()
            .title("Server Panel")
            .refresh_interval_ms(1500)
            .show_raw_snapshot(true),
    );

let app = Router::new()
    .merge(websocket_router("/ws", websocket_hub))
    .nest("/panel", panel_routes(panel_state));
```

## Endpoints

- `/panel` : HTML dashboard
- `/panel/api/snapshot` : combined JSON snapshot
- `/panel/api/http` : HTTP monitoring info
- `/panel/api/websocket` : WebSocket hub inspection (404 if websocket hub not configured)

Note: requests under `/panel` and `/monitor` are excluded from request counting stats by default.

## 详细文档

- 详细接入说明：`doc/HTTP_PANEL_GUIDE.md`
- 运行示例命令：`doc/SAMPLES.md`
