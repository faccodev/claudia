use axum::{
    extract::{ws::{WebSocket, WebSocketUpgrade}, Query, State},
    response::IntoResponse,
};
use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    Query(params): Query<std::collections::HashMap<String, String>>,
    State(state): State<Arc<crate::state::AppState>>,
) -> impl IntoResponse {
    let run_id = params.get("run_id").and_then(|s| s.parse::<i64>().ok());
    let session_id = params.get("session_id").map(|s| s.clone());

    ws.on_upgrade(move |socket| handle_socket(socket, state, run_id, session_id))
}

async fn handle_socket(
    socket: WebSocket,
    state: Arc<crate::state::AppState>,
    run_id: Option<i64>,
    session_id: Option<String>,
) {
    let (sender, receiver) = socket.split();

    // Wrap sender in Arc for sharing across tasks
    let sender = Arc::new(std::sync::Mutex::new(sender));
    let run_id_clone = run_id;
    let session_id_clone = session_id.clone();

    // Send initial connection message
    {
        let sender = sender.lock().unwrap();
        let _ = sender.send(axum::extract::ws::Message::Text(
            serde_json::json!({
                "type": "connected",
                "run_id": run_id_clone,
                "session_id": session_id_clone
            })
            .to_string()
            .into(),
        )).await;
    }

    // Spawn task to handle incoming messages
    let sender_reader = sender.clone();
    let state_for_reader = state.clone();
    tokio::spawn(async move {
        let mut receiver = receiver;
        while let Some(msg) = receiver.next().await {
            if let Ok(axum::extract::ws::Message::Text(text)) = msg {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
                    if json.get("type").and_then(|t| t.as_str()) == Some("subscribe") {
                        let run_id = json.get("run_id").and_then(|r| r.as_i64());
                        let session_id = json.get("session_id").and_then(|s| s.as_str());
                        let sender = sender_reader.lock().unwrap();
                        let _ = sender.send(axum::extract::ws::Message::Text(
                            serde_json::json!({
                                "type": "subscribed",
                                "run_id": run_id,
                                "session_id": session_id
                            })
                            .to_string()
                            .into(),
                        )).await;
                    }
                }
            }
        }
    });

    // Poll for live output if run_id is specified
    if let Some(run_id) = run_id {
        let sender_poll = sender.clone();
        let state_for_poll = state.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

                // Get live output
                let output = state_for_poll.process_registry.get_live_output(run_id).await;
                if !output.is_empty() {
                    let sender = sender_poll.lock().unwrap();
                    let _ = sender.send(axum::extract::ws::Message::Text(
                        serde_json::json!({
                            "type": "output",
                            "run_id": run_id,
                            "data": output
                        })
                        .to_string()
                        .into(),
                    )).await;
                }

                // Check status
                let status = {
                    let db = state_for_poll.db.lock().unwrap();
                    db.query_row(
                        "SELECT status FROM agent_runs WHERE id = ?",
                        [run_id],
                        |row| row.get::<_, String>(0),
                    ).ok()
                };

                if let Some(status) = status {
                    if status == "completed" || status == "failed" || status == "cancelled" {
                        let sender = sender_poll.lock().unwrap();
                        let _ = sender.send(axum::extract::ws::Message::Text(
                            serde_json::json!({
                                "type": "complete",
                                "run_id": run_id,
                                "status": status
                            })
                            .to_string()
                            .into(),
                        )).await;
                        break;
                    }
                }
            }
        });
    }

    // Poll for Claude session output if session_id is specified
    if session_id.is_some() {
        let sender_poll = sender.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                let sender = sender_poll.lock().unwrap();
                let _ = sender.send(axum::extract::ws::Message::Text(
                    serde_json::json!({
                        "type": "heartbeat"
                    })
                    .to_string()
                    .into(),
                )).await;
            }
        });
    }
}
