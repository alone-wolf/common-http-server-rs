//! `http-panel` provides ready-to-mount Axum routes for web-based runtime inspection.
//!
//! It exposes:
//! - `/`              : HTML dashboard page
//! - `/api/snapshot`  : combined HTTP + WebSocket snapshot
//! - `/api/http`      : HTTP monitoring data
//! - `/api/websocket` : WebSocket hub inspection snapshot

use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse},
    routing::get,
};
use common_http_server_rs::{MonitoringInfo, MonitoringState};
use serde::Serialize;
use websocket::{WebSocketHub, WebSocketHubInspection};

#[derive(Debug, Clone)]
pub struct HttpPanelConfig {
    pub title: String,
    pub refresh_interval_ms: u64,
    pub show_raw_snapshot: bool,
}

impl Default for HttpPanelConfig {
    fn default() -> Self {
        Self {
            title: "HTTP Panel".to_string(),
            refresh_interval_ms: 2000,
            show_raw_snapshot: true,
        }
    }
}

impl HttpPanelConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    pub fn refresh_interval_ms(mut self, interval_ms: u64) -> Self {
        self.refresh_interval_ms = interval_ms.max(250);
        self
    }

    pub fn show_raw_snapshot(mut self, enabled: bool) -> Self {
        self.show_raw_snapshot = enabled;
        self
    }
}

#[derive(Debug, Clone)]
pub struct HttpPanelState {
    monitoring: MonitoringState,
    websocket_hub: Option<WebSocketHub>,
    config: HttpPanelConfig,
}

impl HttpPanelState {
    pub fn new(monitoring: MonitoringState) -> Self {
        Self {
            monitoring,
            websocket_hub: None,
            config: HttpPanelConfig::default(),
        }
    }

    pub fn with_websocket_hub(mut self, websocket_hub: WebSocketHub) -> Self {
        self.websocket_hub = Some(websocket_hub);
        self
    }

    pub fn with_config(mut self, config: HttpPanelConfig) -> Self {
        self.config = config;
        self
    }
}

#[derive(Debug, Serialize)]
pub struct HttpPanelSnapshot {
    pub generated_at: String,
    pub http: MonitoringInfo,
    pub websocket: Option<WebSocketHubInspection>,
}

#[derive(Debug, Clone, Serialize)]
struct WebSocketInspectionPayload {
    enabled: bool,
    snapshot: Option<WebSocketHubInspection>,
    message: Option<String>,
}

pub fn panel_routes(state: HttpPanelState) -> Router {
    Router::new()
        .route("/", get(panel_page))
        .route("/api/snapshot", get(snapshot_endpoint))
        .route("/api/http", get(http_info_endpoint))
        .route("/api/websocket", get(websocket_info_endpoint))
        .with_state(state)
}

async fn panel_page(State(state): State<HttpPanelState>) -> Html<String> {
    Html(render_dashboard_html(&state.config))
}

async fn snapshot_endpoint(State(state): State<HttpPanelState>) -> Json<HttpPanelSnapshot> {
    Json(collect_snapshot(&state).await)
}

async fn http_info_endpoint(State(state): State<HttpPanelState>) -> Json<MonitoringInfo> {
    Json(collect_monitoring_info(&state.monitoring).await)
}

async fn websocket_info_endpoint(State(state): State<HttpPanelState>) -> impl IntoResponse {
    match &state.websocket_hub {
        Some(hub) => Json(WebSocketInspectionPayload {
            enabled: true,
            snapshot: Some(hub.inspect().await),
            message: None,
        })
        .into_response(),
        None => (
            StatusCode::NOT_FOUND,
            Json(WebSocketInspectionPayload {
                enabled: false,
                snapshot: None,
                message: Some("websocket hub is not configured".to_string()),
            }),
        )
            .into_response(),
    }
}

async fn collect_snapshot(state: &HttpPanelState) -> HttpPanelSnapshot {
    let websocket = match &state.websocket_hub {
        Some(hub) => Some(hub.inspect().await),
        None => None,
    };

    HttpPanelSnapshot {
        generated_at: chrono::Utc::now().to_rfc3339(),
        http: collect_monitoring_info(&state.monitoring).await,
        websocket,
    }
}

async fn collect_monitoring_info(monitoring: &MonitoringState) -> MonitoringInfo {
    let stats = monitoring.stats.read().await;
    let metrics = monitoring.metrics.read().await;

    MonitoringInfo {
        uptime_seconds: stats.uptime().as_secs_f64(),
        total_requests: stats.total_requests(),
        error_requests: stats.error_requests(),
        request_rate: stats.request_rate(),
        error_rate: stats.error_rate(),
        active_connections: metrics.active_connections_value(),
        system_cpu_usage: metrics.system_cpu_usage_value(),
        system_memory_usage: metrics.system_memory_usage_value(),
    }
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn render_dashboard_html(config: &HttpPanelConfig) -> String {
    let title = escape_html(&config.title);
    let raw_snapshot_section = if config.show_raw_snapshot {
        r#"
  <details open>
    <summary>Raw Snapshot JSON</summary>
    <pre id="snapshot">-</pre>
  </details>
"#
    } else {
        ""
    };

    format!(
        r#"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8" />
<meta name="viewport" content="width=device-width, initial-scale=1" />
<title>{title}</title>
<style>
  body {{ font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Arial, sans-serif; margin: 16px; background: #f7f8fa; color: #1f2937; }}
  h1 {{ margin: 0 0 4px 0; }}
  h2 {{ margin: 18px 0 10px 0; }}
  .muted {{ color: #6b7280; font-size: 12px; }}
  .toolbar {{ display: flex; gap: 8px; align-items: center; flex-wrap: wrap; margin: 10px 0 6px 0; }}
  .toolbar button {{ border: 1px solid #d1d5db; background: #ffffff; border-radius: 6px; padding: 6px 10px; cursor: pointer; }}
  .toolbar input {{ border: 1px solid #d1d5db; border-radius: 6px; padding: 6px 8px; width: 100px; }}
  .row {{ display: grid; grid-template-columns: repeat(auto-fit, minmax(180px, 1fr)); gap: 10px; }}
  .card {{ background: #fff; border: 1px solid #e5e7eb; border-radius: 8px; padding: 10px; }}
  .label {{ font-size: 12px; color: #4b5563; }}
  .value {{ font-size: 22px; font-weight: 700; margin-top: 2px; }}
  .table-wrap {{ background: #fff; border: 1px solid #e5e7eb; border-radius: 8px; overflow: auto; }}
  table {{ width: 100%; border-collapse: collapse; }}
  th, td {{ border-bottom: 1px solid #f1f5f9; text-align: left; padding: 8px; font-size: 13px; vertical-align: top; }}
  th {{ color: #374151; background: #f9fafb; position: sticky; top: 0; }}
  pre {{ background: #111827; color: #e5e7eb; padding: 12px; border-radius: 8px; overflow: auto; }}
  .ok {{ color: #16a34a; }}
  .warn {{ color: #b45309; }}
  .err {{ color: #dc2626; }}
</style>
</head>
<body>
  <h1>{title}</h1>
  <div class="muted">Operational dashboard for HTTP + WebSocket inspection</div>
  <div class="toolbar">
    <button id="refresh_now" type="button">Refresh Now</button>
    <button id="toggle_pause" type="button">Pause</button>
    <label for="interval_input" class="muted">Interval(ms)</label>
    <input id="interval_input" type="number" min="250" step="250" value="{refresh_interval_ms}" />
    <button id="apply_interval" type="button">Apply</button>
  </div>
  <div id="status" class="muted">loading...</div>

  <h2>HTTP Metrics</h2>
  <div class="row">
    <div class="card"><div class="label">Uptime</div><div id="uptime_seconds" class="value">-</div></div>
    <div class="card"><div class="label">Total Requests</div><div id="total_requests" class="value">-</div></div>
    <div class="card"><div class="label">Error Requests</div><div id="error_requests" class="value">-</div></div>
    <div class="card"><div class="label">Request Rate</div><div id="request_rate" class="value">-</div></div>
    <div class="card"><div class="label">Error Rate</div><div id="error_rate" class="value">-</div></div>
    <div class="card"><div class="label">Active Connections</div><div id="active_connections" class="value">-</div></div>
    <div class="card"><div class="label">CPU Usage</div><div id="system_cpu_usage" class="value">-</div></div>
    <div class="card"><div class="label">Memory Usage</div><div id="system_memory_usage" class="value">-</div></div>
  </div>

  <h2>WebSocket Inspection</h2>
  <div class="row">
    <div class="card"><div class="label">Inspection Enabled</div><div id="ws_enabled" class="value">-</div></div>
    <div class="card"><div class="label">Total Connections</div><div id="ws_total_connections" class="value">-</div></div>
    <div class="card"><div class="label">Total Groups</div><div id="ws_total_groups" class="value">-</div></div>
  </div>

  <h2>WebSocket Groups</h2>
  <div class="table-wrap">
    <table>
      <thead>
        <tr><th>Group</th><th>Members</th><th>Connection IDs</th></tr>
      </thead>
      <tbody id="groups_rows">
        <tr><td colspan="3" class="muted">No data</td></tr>
      </tbody>
    </table>
  </div>

  <h2>WebSocket Connections</h2>
  <div class="table-wrap">
    <table>
      <thead>
        <tr><th>Connection ID</th><th>User</th><th>Auth</th><th>Groups</th></tr>
      </thead>
      <tbody id="connections_rows">
        <tr><td colspan="4" class="muted">No data</td></tr>
      </tbody>
    </table>
  </div>
{raw_snapshot_section}
  <script>
    const panelBasePath = window.location.pathname.replace(/\/+$/, "") || "/";
    const apiBasePath = panelBasePath === "/" ? "" : panelBasePath;
    const state = {{
      intervalMs: {refresh_interval_ms},
      paused: false,
      timer: null,
      showRawSnapshot: {show_raw_snapshot},
    }};

    function setStatus(text, level) {{
      const el = document.getElementById("status");
      el.textContent = text;
      el.className = level ? "muted " + level : "muted";
    }}

    function fmt(value, digits = 2) {{
      if (typeof value !== "number" || !Number.isFinite(value)) return "-";
      return value.toFixed(digits);
    }}

    function setText(id, value) {{
      const el = document.getElementById(id);
      if (el) el.textContent = value;
    }}

    function escapeHtml(value) {{
      return String(value)
        .replaceAll("&", "&amp;")
        .replaceAll("<", "&lt;")
        .replaceAll(">", "&gt;")
        .replaceAll('"', "&quot;")
        .replaceAll("'", "&#39;");
    }}

    function renderWsRows(websocket) {{
      const groupsRows = document.getElementById("groups_rows");
      const connectionsRows = document.getElementById("connections_rows");

      if (!websocket) {{
        groupsRows.innerHTML = '<tr><td colspan="3" class="muted">WebSocket inspection not enabled</td></tr>';
        connectionsRows.innerHTML = '<tr><td colspan="4" class="muted">WebSocket inspection not enabled</td></tr>';
        return;
      }}

      if (!websocket.groups || websocket.groups.length === 0) {{
        groupsRows.innerHTML = '<tr><td colspan="3" class="muted">No groups</td></tr>';
      }} else {{
        groupsRows.innerHTML = websocket.groups.map((item) => {{
          const members = (item.members || []).map((id) => escapeHtml(id)).join("<br/>");
          return `<tr><td>${{escapeHtml(item.group)}}</td><td>${{item.member_count ?? 0}}</td><td>${{members || "-"}}</td></tr>`;
        }}).join("");
      }}

      if (!websocket.connections || websocket.connections.length === 0) {{
        connectionsRows.innerHTML = '<tr><td colspan="4" class="muted">No connections</td></tr>';
      }} else {{
        connectionsRows.innerHTML = websocket.connections.map((item) => {{
          const groups = (item.groups || []).map((g) => escapeHtml(g)).join(", ");
          const user = `${{escapeHtml(item.username || "-")}} (${{escapeHtml(item.user_id || "-")}})`;
          return `<tr><td>${{escapeHtml(item.connection_id || "-")}}</td><td>${{user}}</td><td>${{escapeHtml(item.auth_type || "-")}}</td><td>${{groups || "-"}}</td></tr>`;
        }}).join("");
      }}
    }}

    function renderSnapshot(data) {{
      const http = data.http || {{}};
      setText("uptime_seconds", fmt(http.uptime_seconds, 1) + "s");
      setText("total_requests", http.total_requests ?? "-");
      setText("error_requests", http.error_requests ?? "-");
      setText("request_rate", fmt(http.request_rate, 2) + "/s");
      setText("error_rate", fmt(http.error_rate, 2) + "%");
      setText("active_connections", fmt(http.active_connections, 0));
      setText("system_cpu_usage", fmt(http.system_cpu_usage, 2) + "%");
      setText("system_memory_usage", fmt(http.system_memory_usage, 2) + "%");

      const websocket = data.websocket;
      setText("ws_enabled", websocket ? "yes" : "no");
      setText("ws_total_connections", websocket ? websocket.total_connections : "-");
      setText("ws_total_groups", websocket ? websocket.total_groups : "-");
      renderWsRows(websocket);

      if (state.showRawSnapshot) {{
        const snapshotEl = document.getElementById("snapshot");
        if (snapshotEl) snapshotEl.textContent = JSON.stringify(data, null, 2);
      }}
      setStatus("updated: " + (data.generated_at || "-"), "ok");
    }}

    async function refreshPanel() {{
      if (state.paused) return;
      try {{
        const resp = await fetch(apiBasePath + "/api/snapshot", {{ cache: "no-store" }});
        if (!resp.ok) throw new Error("HTTP " + resp.status);
        const data = await resp.json();
        renderSnapshot(data);
      }} catch (err) {{
        setStatus("refresh failed: " + err, "err");
      }}
    }}

    function reschedule() {{
      if (state.timer) clearInterval(state.timer);
      state.timer = setInterval(refreshPanel, state.intervalMs);
    }}

    document.getElementById("refresh_now").addEventListener("click", () => {{
      refreshPanel();
    }});
    document.getElementById("toggle_pause").addEventListener("click", (event) => {{
      state.paused = !state.paused;
      event.target.textContent = state.paused ? "Resume" : "Pause";
      setStatus(state.paused ? "paused" : "running", state.paused ? "warn" : "ok");
      if (!state.paused) refreshPanel();
    }});
    document.getElementById("apply_interval").addEventListener("click", () => {{
      const value = Number(document.getElementById("interval_input").value);
      state.intervalMs = Number.isFinite(value) ? Math.max(250, Math.floor(value)) : state.intervalMs;
      document.getElementById("interval_input").value = String(state.intervalMs);
      reschedule();
      setStatus("interval updated: " + state.intervalMs + "ms", "ok");
    }});

    refreshPanel();
    reschedule();
  </script>
</body>
</html>
"#,
        title = title,
        refresh_interval_ms = config.refresh_interval_ms,
        raw_snapshot_section = raw_snapshot_section,
        show_raw_snapshot = config.show_raw_snapshot,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn snapshot_without_websocket_hub_sets_none() {
        let state = HttpPanelState::new(MonitoringState::new());
        let snapshot = collect_snapshot(&state).await;

        assert!(snapshot.websocket.is_none());
        assert_eq!(snapshot.http.total_requests, 0);
    }

    #[tokio::test]
    async fn snapshot_with_websocket_hub_includes_connection_counts() {
        let monitoring = MonitoringState::new();
        let websocket_hub = WebSocketHub::new();
        let (_connection_id, mut rx) = websocket_hub.register(None).await;
        let _ = rx.recv().await;

        let state = HttpPanelState::new(monitoring).with_websocket_hub(websocket_hub);
        let snapshot = collect_snapshot(&state).await;
        let websocket = snapshot.websocket.expect("websocket snapshot should exist");

        assert_eq!(websocket.total_connections, 1);
        assert_eq!(websocket.total_groups, 0);
    }

    #[test]
    fn config_interval_has_lower_bound() {
        let config = HttpPanelConfig::new().refresh_interval_ms(10);
        assert_eq!(config.refresh_interval_ms, 250);
    }

    #[test]
    fn html_render_respects_escape_and_raw_section_switch() {
        let config = HttpPanelConfig::new()
            .title("Ops <Panel>")
            .show_raw_snapshot(false);

        let html = render_dashboard_html(&config);
        assert!(html.contains("<title>Ops &lt;Panel&gt;</title>"));
        assert!(!html.contains("id=\"snapshot\""));
        assert!(
            html.contains("const apiBasePath = panelBasePath === \"/\" ? \"\" : panelBasePath;")
        );
        assert!(html.contains("fetch(apiBasePath + \"/api/snapshot\""));
    }
}
