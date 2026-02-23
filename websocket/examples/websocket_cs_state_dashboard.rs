use axum::Router;
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::time::Duration;
use tokio::time::timeout;
use websocket::{ServerMessage, WebSocketClient, WebSocketHub, websocket_router};

const GROUP_ALPHA: &str = "room.alpha";
const GROUP_BETA: &str = "room.beta";

type DemoResult<T> = Result<T, Box<dyn Error>>;

#[derive(Debug, Default)]
struct ClientState {
    name: &'static str,
    connection_id: Option<String>,
    groups: BTreeSet<String>,
    sent_events: usize,
    received_events: usize,
    received_events_by_group: BTreeMap<String, usize>,
    last_pong_nonce: Option<String>,
    last_error: Option<String>,
}

impl ClientState {
    fn new(name: &'static str) -> Self {
        Self {
            name,
            ..Self::default()
        }
    }

    fn mark_sent_event(&mut self) {
        self.sent_events += 1;
    }

    fn apply_inbound(&mut self, message: &ServerMessage) {
        match message {
            ServerMessage::Connected { connection_id, .. } => {
                self.connection_id = Some(connection_id.clone());
            }
            ServerMessage::Joined { group } => {
                self.groups.insert(group.clone());
            }
            ServerMessage::Left { group } => {
                self.groups.remove(group);
            }
            ServerMessage::Event { group, .. } => {
                self.received_events += 1;
                *self
                    .received_events_by_group
                    .entry(group.clone())
                    .or_insert(0) += 1;
            }
            ServerMessage::Direct { .. } => {
                self.received_events += 1;
                *self
                    .received_events_by_group
                    .entry("direct".to_string())
                    .or_insert(0) += 1;
            }
            ServerMessage::Pong { nonce } => {
                self.last_pong_nonce = nonce.clone();
            }
            ServerMessage::Error { code, message } => {
                self.last_error = Some(format!("{code}: {message}"));
            }
        }
    }

    fn groups_display(&self) -> String {
        if self.groups.is_empty() {
            return "-".to_string();
        }
        self.groups.iter().cloned().collect::<Vec<_>>().join(",")
    }

    fn event_counter_display(&self) -> String {
        if self.received_events_by_group.is_empty() {
            return "-".to_string();
        }

        self.received_events_by_group
            .iter()
            .map(|(group, count)| format!("{group}:{count}"))
            .collect::<Vec<_>>()
            .join(",")
    }
}

#[derive(Debug, Default)]
struct DemoDashboard {
    timeline: Vec<String>,
}

impl DemoDashboard {
    fn push(&mut self, item: impl Into<String>) {
        self.timeline.push(item.into());
    }

    async fn render(
        &self,
        stage: &str,
        hub: &WebSocketHub,
        states: [&ClientState; 3],
    ) -> DemoResult<()> {
        println!("\n==================== {stage} ====================");
        println!(
            "server group members => {GROUP_ALPHA}: {}, {GROUP_BETA}: {}",
            hub.group_member_count(GROUP_ALPHA).await,
            hub.group_member_count(GROUP_BETA).await
        );
        println!(
            "{:<8} {:<13} {:<24} {:<8} {:<9} {:<22} {:<12}",
            "client", "connection", "groups", "sent", "recv", "recv_by_group", "last_pong"
        );

        for state in states {
            let connection = state.connection_id.as_deref().unwrap_or("-");
            let last_pong = state.last_pong_nonce.as_deref().unwrap_or("-");
            println!(
                "{:<8} {:<13} {:<24} {:<8} {:<9} {:<22} {:<12}",
                state.name,
                short_id(connection),
                state.groups_display(),
                state.sent_events,
                state.received_events,
                state.event_counter_display(),
                last_pong
            );
            if let Some(err) = &state.last_error {
                println!("  -> last_error: {err}");
            }
        }

        let recent = self
            .timeline
            .iter()
            .rev()
            .take(8)
            .cloned()
            .collect::<Vec<_>>();
        println!("recent timeline:");
        for item in recent.iter().rev() {
            println!("  - {item}");
        }
        Ok(())
    }
}

fn short_id(value: &str) -> &str {
    if value.len() <= 12 {
        value
    } else {
        &value[..12]
    }
}

fn describe_message(message: &ServerMessage) -> String {
    match message {
        ServerMessage::Connected {
            connection_id,
            actor,
        } => {
            format!(
                "connected(connection_id={}, actor={})",
                short_id(connection_id),
                actor.username
            )
        }
        ServerMessage::Joined { group } => format!("joined(group={group})"),
        ServerMessage::Left { group } => format!("left(group={group})"),
        ServerMessage::Event {
            group,
            event,
            from,
            payload,
            ..
        } => {
            format!(
                "event(group={group}, event={event}, from={}, payload={payload})",
                from.username
            )
        }
        ServerMessage::Direct {
            from_connection_id,
            event,
            from,
            payload,
            ..
        } => {
            format!(
                "direct(from_connection_id={}, event={event}, from={}, payload={payload})",
                short_id(from_connection_id),
                from.username
            )
        }
        ServerMessage::Pong { nonce } => format!("pong(nonce={nonce:?})"),
        ServerMessage::Error { code, message } => format!("error(code={code}, message={message})"),
    }
}

async fn recv_and_record(
    client: &mut WebSocketClient,
    state: &mut ClientState,
    dashboard: &mut DemoDashboard,
    reason: &str,
) -> DemoResult<ServerMessage> {
    let message = timeout(Duration::from_secs(3), client.recv())
        .await
        .map_err(|_| format!("timeout while waiting message: {reason}"))??;
    state.apply_inbound(&message);
    dashboard.push(format!(
        "recv/{reason}: {} <- {}",
        state.name,
        describe_message(&message)
    ));
    Ok(message)
}

async fn expect_no_message_for(
    client: &mut WebSocketClient,
    name: &str,
    wait_for: Duration,
    dashboard: &mut DemoDashboard,
    reason: &str,
) -> DemoResult<()> {
    match timeout(wait_for, client.recv()).await {
        Err(_) => {
            dashboard.push(format!("recv/{reason}: {name} no frame as expected"));
            Ok(())
        }
        Ok(Ok(message)) => Err(format!(
            "{name} unexpectedly received message during {reason}: {}",
            describe_message(&message)
        )
        .into()),
        Ok(Err(err)) => Err(format!("{name} recv failed during {reason}: {err}").into()),
    }
}

fn expect_message_type(
    message: &ServerMessage,
    expected: &str,
    checker: impl FnOnce(&ServerMessage) -> bool,
) -> DemoResult<()> {
    if checker(message) {
        Ok(())
    } else {
        Err(format!("expected {expected}, got {}", describe_message(message)).into())
    }
}

struct DemoServer {
    hub: WebSocketHub,
    ws_url: String,
    shutdown_tx: tokio::sync::oneshot::Sender<()>,
    server_task: tokio::task::JoinHandle<std::io::Result<()>>,
}

async fn start_demo_server() -> DemoResult<DemoServer> {
    let hub = WebSocketHub::new();
    let app = Router::new().merge(websocket_router("/ws", hub.clone()));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    let ws_url = format!("ws://{addr}/ws");

    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();
    let server_task = tokio::spawn(async move {
        axum::serve(listener, app)
            .with_graceful_shutdown(async {
                let _ = shutdown_rx.await;
            })
            .await
    });

    Ok(DemoServer {
        hub,
        ws_url,
        shutdown_tx,
        server_task,
    })
}

async fn stop_demo_server(server: DemoServer) -> DemoResult<()> {
    let _ = server.shutdown_tx.send(());
    server.server_task.await??;
    Ok(())
}

struct DemoClients {
    alice: WebSocketClient,
    bob: WebSocketClient,
    carol: WebSocketClient,
}

struct DemoStates {
    alice: ClientState,
    bob: ClientState,
    carol: ClientState,
}

impl DemoStates {
    fn new() -> Self {
        Self {
            alice: ClientState::new("alice"),
            bob: ClientState::new("bob"),
            carol: ClientState::new("carol"),
        }
    }

    fn as_refs(&self) -> [&ClientState; 3] {
        [&self.alice, &self.bob, &self.carol]
    }
}

async fn connect_demo_clients(ws_url: &str) -> DemoResult<DemoClients> {
    let alice = WebSocketClient::builder(ws_url)
        .prefer_msgpack()
        .connect()
        .await?;
    let bob = WebSocketClient::builder(ws_url)
        .force_json()
        .connect()
        .await?;
    let carol = WebSocketClient::builder(ws_url)
        .force_msgpack()
        .connect()
        .await?;

    Ok(DemoClients { alice, bob, carol })
}

async fn stage_connected(
    clients: &mut DemoClients,
    states: &mut DemoStates,
    dashboard: &mut DemoDashboard,
) -> DemoResult<()> {
    let connected = recv_and_record(
        &mut clients.alice,
        &mut states.alice,
        dashboard,
        "alice-connected",
    )
    .await?;
    expect_message_type(&connected, "connected", |msg| {
        matches!(msg, ServerMessage::Connected { .. })
    })?;

    let connected = recv_and_record(
        &mut clients.bob,
        &mut states.bob,
        dashboard,
        "bob-connected",
    )
    .await?;
    expect_message_type(&connected, "connected", |msg| {
        matches!(msg, ServerMessage::Connected { .. })
    })?;

    let connected = recv_and_record(
        &mut clients.carol,
        &mut states.carol,
        dashboard,
        "carol-connected",
    )
    .await?;
    expect_message_type(&connected, "connected", |msg| {
        matches!(msg, ServerMessage::Connected { .. })
    })?;

    Ok(())
}

async fn stage_group_membership(
    clients: &mut DemoClients,
    states: &mut DemoStates,
    dashboard: &mut DemoDashboard,
) -> DemoResult<()> {
    clients.alice.join_group(GROUP_ALPHA).await?;
    recv_and_record(
        &mut clients.alice,
        &mut states.alice,
        dashboard,
        "alice-join-alpha",
    )
    .await?;
    clients.alice.join_group(GROUP_BETA).await?;
    recv_and_record(
        &mut clients.alice,
        &mut states.alice,
        dashboard,
        "alice-join-beta",
    )
    .await?;

    clients.bob.join_group(GROUP_ALPHA).await?;
    recv_and_record(
        &mut clients.bob,
        &mut states.bob,
        dashboard,
        "bob-join-alpha",
    )
    .await?;

    clients.carol.join_group(GROUP_BETA).await?;
    recv_and_record(
        &mut clients.carol,
        &mut states.carol,
        dashboard,
        "carol-join-beta",
    )
    .await?;

    Ok(())
}

async fn stage_group_broadcast(
    clients: &mut DemoClients,
    states: &mut DemoStates,
    dashboard: &mut DemoDashboard,
) -> DemoResult<()> {
    states.alice.mark_sent_event();
    dashboard.push("send/alice -> room.alpha message.new".to_string());
    clients
        .alice
        .emit_event(
            GROUP_ALPHA,
            "message.new",
            json!({"text": "hello alpha", "seq": 1}),
        )
        .await?;
    recv_and_record(
        &mut clients.alice,
        &mut states.alice,
        dashboard,
        "alice-recv-alpha-event",
    )
    .await?;
    recv_and_record(
        &mut clients.bob,
        &mut states.bob,
        dashboard,
        "bob-recv-alpha-event",
    )
    .await?;

    states.carol.mark_sent_event();
    dashboard.push("send/carol -> room.beta message.new".to_string());
    clients
        .carol
        .emit_event(
            GROUP_BETA,
            "message.new",
            json!({"text": "hello beta", "seq": 2}),
        )
        .await?;
    recv_and_record(
        &mut clients.alice,
        &mut states.alice,
        dashboard,
        "alice-recv-beta-event",
    )
    .await?;
    recv_and_record(
        &mut clients.carol,
        &mut states.carol,
        dashboard,
        "carol-recv-beta-event",
    )
    .await?;

    Ok(())
}

async fn stage_ping_direct_and_invalid(
    clients: &mut DemoClients,
    states: &mut DemoStates,
    dashboard: &mut DemoDashboard,
) -> DemoResult<()> {
    clients.bob.ping(Some("heartbeat-1".to_string())).await?;
    recv_and_record(&mut clients.bob, &mut states.bob, dashboard, "bob-ping").await?;

    let carol_connection_id = states
        .carol
        .connection_id
        .clone()
        .ok_or("carol connection id should be available")?;
    let alice_connection_id = states
        .alice
        .connection_id
        .clone()
        .ok_or("alice connection id should be available")?;

    states.alice.mark_sent_event();
    dashboard.push("send/alice -> carol direct.notice".to_string());
    clients
        .alice
        .emit_direct(
            carol_connection_id,
            "direct.notice",
            json!({"text":"private hello", "seq": "d1"}),
        )
        .await?;
    let direct_result = recv_and_record(
        &mut clients.carol,
        &mut states.carol,
        dashboard,
        "carol-recv-direct",
    )
    .await?;
    expect_message_type(&direct_result, "direct", |msg| {
        matches!(
            msg,
            ServerMessage::Direct {
                from_connection_id,
                event,
                ..
            } if from_connection_id == &alice_connection_id && event == "direct.notice"
        )
    })?;
    expect_no_message_for(
        &mut clients.bob,
        "bob",
        Duration::from_millis(250),
        dashboard,
        "bob-no-direct",
    )
    .await?;

    clients.bob.join_group(" room.invalid ").await?;
    let invalid_group_result = recv_and_record(
        &mut clients.bob,
        &mut states.bob,
        dashboard,
        "bob-invalid-group",
    )
    .await?;
    expect_message_type(
        &invalid_group_result,
        "error.invalid_group",
        |msg| matches!(msg, ServerMessage::Error { code, .. } if code == "invalid_group"),
    )?;

    Ok(())
}

async fn stage_leave_and_isolated_broadcast(
    clients: &mut DemoClients,
    states: &mut DemoStates,
    dashboard: &mut DemoDashboard,
) -> DemoResult<()> {
    clients.alice.leave_group(GROUP_ALPHA).await?;
    recv_and_record(
        &mut clients.alice,
        &mut states.alice,
        dashboard,
        "alice-leave-alpha",
    )
    .await?;

    states.bob.mark_sent_event();
    dashboard.push("send/bob -> room.alpha message.new(after leave)".to_string());
    clients
        .bob
        .emit_event(
            GROUP_ALPHA,
            "message.new",
            json!({"text": "only bob should receive this", "seq": 3}),
        )
        .await?;
    recv_and_record(
        &mut clients.bob,
        &mut states.bob,
        dashboard,
        "bob-recv-alpha-after-leave",
    )
    .await?;

    expect_no_message_for(
        &mut clients.alice,
        "alice",
        Duration::from_millis(350),
        dashboard,
        "alice-no-alpha-after-leave",
    )
    .await?;

    Ok(())
}

async fn close_demo_clients(clients: DemoClients) -> DemoResult<()> {
    clients.alice.close().await?;
    clients.bob.close().await?;
    clients.carol.close().await?;
    Ok(())
}

fn main() -> DemoResult<()> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;

    runtime.block_on(async {
        let server = start_demo_server().await?;
        println!("[server] listening on {}", server.ws_url);

        let mut dashboard = DemoDashboard::default();
        let mut states = DemoStates::new();

        let mut clients = connect_demo_clients(&server.ws_url).await?;
        stage_connected(&mut clients, &mut states, &mut dashboard).await?;

        dashboard
            .render("stage-1 connected", &server.hub, states.as_refs())
            .await?;

        stage_group_membership(&mut clients, &mut states, &mut dashboard).await?;

        dashboard
            .render("stage-2 group membership", &server.hub, states.as_refs())
            .await?;

        stage_group_broadcast(&mut clients, &mut states, &mut dashboard).await?;

        dashboard
            .render("stage-3 event broadcast", &server.hub, states.as_refs())
            .await?;

        stage_ping_direct_and_invalid(&mut clients, &mut states, &mut dashboard).await?;

        dashboard
            .render(
                "stage-4 ping + direct + invalid operation",
                &server.hub,
                states.as_refs(),
            )
            .await?;

        stage_leave_and_isolated_broadcast(&mut clients, &mut states, &mut dashboard).await?;

        dashboard
            .render(
                "stage-5 leave + isolated broadcast",
                &server.hub,
                states.as_refs(),
            )
            .await?;

        close_demo_clients(clients).await?;
        stop_demo_server(server).await?;

        println!("\n[done] websocket c/s dashboard demo completed");
        Ok(())
    })
}
