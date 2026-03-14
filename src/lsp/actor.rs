use std::collections::HashMap;
use std::process::Stdio;
use tokio::io::{AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::{mpsc, oneshot};

use super::codec;

// ─── Message types ─────────────────────────────────────────────────────

#[derive(Debug)]
pub enum ActorMessage {
    Request {
        method: String,
        params: serde_json::Value,
        response_tx: oneshot::Sender<Result<serde_json::Value, String>>,
    },
    Notification {
        method: String,
        params: serde_json::Value,
    },
    Cancel { id: i64 },
    Shutdown,
}

#[derive(Debug, Clone)]
pub struct LspServerNotification {
    pub method: String,
    pub params: serde_json::Value,
}

// ─── Actor handle ──────────────────────────────────────────────────────

pub struct ActorHandle {
    pub tx: mpsc::UnboundedSender<ActorMessage>,
    pub language_id: String,
    pub server_name: String,
}

impl ActorHandle {
    pub async fn send_request(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let (response_tx, response_rx) = oneshot::channel();
        self.tx
            .send(ActorMessage::Request {
                method: method.to_string(),
                params,
                response_tx,
            })
            .map_err(|_| "actor channel closed".to_string())?;

        response_rx
            .await
            .map_err(|_| "actor dropped response sender".to_string())?
    }

    pub fn send_notification(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<(), String> {
        self.tx
            .send(ActorMessage::Notification {
                method: method.to_string(),
                params,
            })
            .map_err(|_| "actor channel closed".to_string())
    }

    pub fn request_shutdown(&self) {
        let _ = self.tx.send(ActorMessage::Shutdown);
    }
}

// ─── Actor spawner ─────────────────────────────────────────────────────

pub struct SpawnedActor {
    pub handle: ActorHandle,
    pub join_handle: tokio::task::JoinHandle<()>,
}

/// Spawn a language server process and its dedicated actor task.
pub fn spawn_actor(
    command: &str,
    args: &[String],
    working_dir: &std::path::Path,
    language_id: &str,
    event_tx: mpsc::UnboundedSender<LspServerNotification>,
) -> Result<SpawnedActor, String> {
    log::info!("spawning LSP server: {} {:?}", command, args);

    let mut child = Command::new(command)
        .args(args)
        .current_dir(working_dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| format!("failed to spawn {}: {}", command, e))?;

    let stdin = child
        .stdin
        .take()
        .ok_or_else(|| "failed to capture server stdin".to_string())?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "failed to capture server stdout".to_string())?;

    let (tx, rx) = mpsc::unbounded_channel();

    let server_name = command.to_string();
    let lang = language_id.to_string();

    let join_handle = tokio::spawn(actor_loop(
        rx,
        stdin,
        BufReader::new(stdout),
        child,
        event_tx,
        server_name.clone(),
    ));

    Ok(SpawnedActor {
        handle: ActorHandle {
            tx,
            language_id: lang,
            server_name,
        },
        join_handle,
    })
}

// ─── Actor loop ────────────────────────────────────────────────────────

async fn actor_loop(
    mut rx: mpsc::UnboundedReceiver<ActorMessage>,
    mut stdin: tokio::process::ChildStdin,
    mut stdout: BufReader<tokio::process::ChildStdout>,
    mut child: Child,
    event_tx: mpsc::UnboundedSender<LspServerNotification>,
    server_name: String,
) {
    let mut next_id: i64 = 1;
    let mut pending: HashMap<i64, oneshot::Sender<Result<serde_json::Value, String>>> =
        HashMap::new();

    loop {
        tokio::select! {
            // Inbound from Klein
            msg = rx.recv() => {
                match msg {
                    Some(ActorMessage::Request { method, params, response_tx }) => {
                        let id = next_id;
                        next_id += 1;

                        let json_msg = serde_json::json!({
                            "jsonrpc": "2.0",
                            "id": id,
                            "method": method,
                            "params": params,
                        });

                        let bytes = codec::encode(&json_msg);
                        if let Err(e) = stdin.write_all(&bytes).await {
                            log::error!("[{}] stdin write failed: {}", server_name, e);
                            let _ = response_tx.send(Err(format!("write failed: {}", e)));
                            break;
                        }
                        let _ = stdin.flush().await;

                        pending.insert(id, response_tx);
                        log::debug!("[{}] → request #{}: {}", server_name, id, method);
                    }

                    Some(ActorMessage::Notification { method, params }) => {
                        let json_msg = serde_json::json!({
                            "jsonrpc": "2.0",
                            "method": method,
                            "params": params,
                        });

                        let bytes = codec::encode(&json_msg);
                        if let Err(e) = stdin.write_all(&bytes).await {
                            log::error!("[{}] stdin write failed: {}", server_name, e);
                            break;
                        }
                        let _ = stdin.flush().await;
                        log::debug!("[{}] → notification: {}", server_name, method);
                    }

                    Some(ActorMessage::Cancel { id }) => {
                        if pending.remove(&id).is_some() {
                            let cancel = serde_json::json!({
                                "jsonrpc": "2.0",
                                "method": "$/cancelRequest",
                                "params": { "id": id },
                            });
                            let bytes = codec::encode(&cancel);
                            let _ = stdin.write_all(&bytes).await;
                            let _ = stdin.flush().await;
                            log::debug!("[{}] cancelled request #{}", server_name, id);
                        }
                    }

                    Some(ActorMessage::Shutdown) => {
                        log::info!("[{}] shutting down", server_name);
                        do_shutdown(&mut stdin, &mut stdout, &mut next_id, &server_name).await;
                        let _ = child.kill().await;
                        break;
                    }

                    None => {
                        // All senders dropped, clean up
                        log::info!("[{}] channel closed, exiting actor", server_name);
                        let _ = child.kill().await;
                        break;
                    }
                }
            }

            // Inbound from the language server
            result = codec::decode(&mut stdout) => {
                match result {
                    Ok(msg) => {
                        handle_server_message(msg, &mut pending, &event_tx, &server_name);
                    }
                    Err(e) => {
                        log::error!("[{}] server read error (likely crashed): {}", server_name, e);
                        // Notify all pending requests that the server is gone
                        for (_, tx) in pending.drain() {
                            let _ = tx.send(Err("server crashed".to_string()));
                        }
                        let _ = event_tx.send(LspServerNotification {
                            method: "klein/serverCrashed".to_string(),
                            params: serde_json::json!({ "server": server_name }),
                        });
                        break;
                    }
                }
            }
        }
    }

    log::info!("[{}] actor loop exited", server_name);
}

fn handle_server_message(
    msg: serde_json::Value,
    pending: &mut HashMap<i64, oneshot::Sender<Result<serde_json::Value, String>>>,
    event_tx: &mpsc::UnboundedSender<LspServerNotification>,
    server_name: &str,
) {
    // Response to a request we sent
    if let Some(id) = msg.get("id").and_then(|v| v.as_i64()) {
        if msg.get("method").is_some() {
            // Server-initiated request (we don't handle these yet)
            log::debug!("[{}] ← server request #{} (ignored)", server_name, id);
            return;
        }

        if let Some(tx) = pending.remove(&id) {
            if let Some(error) = msg.get("error") {
                let err_msg = error
                    .get("message")
                    .and_then(|m| m.as_str())
                    .unwrap_or("unknown error")
                    .to_string();
                log::debug!("[{}] ← response #{}: error: {}", server_name, id, err_msg);
                let _ = tx.send(Err(err_msg));
            } else {
                let result = msg.get("result").cloned().unwrap_or(serde_json::Value::Null);
                log::debug!("[{}] ← response #{}: ok", server_name, id);
                let _ = tx.send(Ok(result));
            }
        } else {
            log::debug!("[{}] ← orphan response #{}", server_name, id);
        }
        return;
    }

    // Server-initiated notification
    if let Some(method) = msg.get("method").and_then(|m| m.as_str()) {
        let params = msg.get("params").cloned().unwrap_or(serde_json::Value::Null);
        log::debug!("[{}] ← notification: {}", server_name, method);
        let _ = event_tx.send(LspServerNotification {
            method: method.to_string(),
            params,
        });
    }
}

async fn do_shutdown(
    stdin: &mut tokio::process::ChildStdin,
    stdout: &mut BufReader<tokio::process::ChildStdout>,
    next_id: &mut i64,
    server_name: &str,
) {
    // Send shutdown request
    let id = *next_id;
    *next_id += 1;
    let shutdown = serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": "shutdown",
        "params": null,
    });
    let bytes = codec::encode(&shutdown);
    if stdin.write_all(&bytes).await.is_err() {
        return;
    }
    let _ = stdin.flush().await;

    // Wait for shutdown response (with timeout)
    match tokio::time::timeout(std::time::Duration::from_secs(5), codec::decode(stdout)).await {
        Ok(Ok(_)) => {
            log::debug!("[{}] shutdown response received", server_name);
        }
        _ => {
            log::warn!("[{}] shutdown response timed out", server_name);
        }
    }

    // Send exit notification
    let exit = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "exit",
    });
    let bytes = codec::encode(&exit);
    let _ = stdin.write_all(&bytes).await;
    let _ = stdin.flush().await;
}
