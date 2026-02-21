use axum::Router;
use serde_json::json;
use std::error::Error;
use websocket::{ServerMessage, WebSocketClient, WebSocketHub, websocket_router};

fn main() -> Result<(), Box<dyn Error>> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;

    runtime.block_on(async {
        let hub = WebSocketHub::new();
        let app = Router::new().merge(websocket_router("/ws", hub));

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let addr = listener.local_addr()?;
        let ws_url = format!("ws://{}/ws", addr);

        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();
        let server_task = tokio::spawn(async move {
            axum::serve(listener, app)
                .with_graceful_shutdown(async {
                    let _ = shutdown_rx.await;
                })
                .await
        });

        println!("[server] listening on {ws_url}");

        let mut alice = WebSocketClient::builder(&ws_url)
            .with_binary_messagepack()
            .connect()
            .await?;
        let mut bob = WebSocketClient::builder(&ws_url)
            .with_binary_messagepack()
            .connect()
            .await?;

        expect_connected("alice", &mut alice).await?;
        expect_connected("bob", &mut bob).await?;

        alice.join_group("chat.room").await?;
        bob.join_group("chat.room").await?;

        expect_joined("alice", &mut alice, "chat.room").await?;
        expect_joined("bob", &mut bob, "chat.room").await?;

        alice
            .emit_event(
                "chat.room",
                "message.new",
                json!({"text": "hello from alice"}),
            )
            .await?;

        expect_event("alice", &mut alice).await?;
        expect_event("bob", &mut bob).await?;

        bob.ping(Some("demo-ping".to_string())).await?;
        expect_pong("bob", &mut bob).await?;

        alice.close().await?;
        bob.close().await?;

        let _ = shutdown_tx.send(());

        let server_result = server_task.await?;
        server_result?;

        println!("[demo] client/server websocket flow finished");
        Ok(())
    })
}

async fn expect_connected(name: &str, client: &mut WebSocketClient) -> Result<(), Box<dyn Error>> {
    let message = client.recv().await?;
    match message {
        ServerMessage::Connected {
            connection_id,
            actor,
        } => {
            println!(
                "[{name}] connected: connection_id={connection_id}, actor={} ({})",
                actor.username, actor.auth_type
            );
            Ok(())
        }
        other => Err(unexpected_message("connected", other)),
    }
}

async fn expect_joined(
    name: &str,
    client: &mut WebSocketClient,
    expected_group: &str,
) -> Result<(), Box<dyn Error>> {
    let message = client.recv().await?;
    match message {
        ServerMessage::Joined { group } if group == expected_group => {
            println!("[{name}] joined group: {group}");
            Ok(())
        }
        other => Err(unexpected_message("joined", other)),
    }
}

async fn expect_event(name: &str, client: &mut WebSocketClient) -> Result<(), Box<dyn Error>> {
    let message = client.recv().await?;
    match message {
        ServerMessage::Event {
            group,
            event,
            payload,
            from,
            timestamp,
        } => {
            println!(
                "[{name}] event: group={group}, event={event}, from={}, payload={}, ts={timestamp}",
                from.username, payload
            );
            Ok(())
        }
        other => Err(unexpected_message("event", other)),
    }
}

async fn expect_pong(name: &str, client: &mut WebSocketClient) -> Result<(), Box<dyn Error>> {
    let message = client.recv().await?;
    match message {
        ServerMessage::Pong { nonce } => {
            println!("[{name}] pong: nonce={:?}", nonce);
            Ok(())
        }
        other => Err(unexpected_message("pong", other)),
    }
}

fn unexpected_message(expected: &str, actual: ServerMessage) -> Box<dyn Error> {
    format!("expected {expected} message, got: {actual:?}").into()
}
