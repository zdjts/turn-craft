//! WebSocket Bridge — 全双工网络网关中枢
//!
//! Two pipelines live inside one `use_coroutine`:
//!
//! **Downstream (server → UI)**
//!   raw JSON text from Axum → `serde_json::Value` → written into `opaque_state`
//!   Signal → Dioxus cascade re-renders every consumer automatically.
//!
//! **Upstream (UI → server)**
//!   game components call `bridge.send(&payload)` → serialized to JSON text →
//!   fired through the WebSocket to Axum's `handle_socket` ingress loop.

use dioxus::prelude::*;
use futures_util::{select, SinkExt, StreamExt};
use gloo_net::websocket::{futures::WebSocket, Message};
use serde_json::Value;
use tracing::{debug, error, info, warn};

// ═══════════════════════════════════════════════════════
//  Public Types
// ═══════════════════════════════════════════════════════

/// Upstream commands: child components push these through the bridge to the server.
#[derive(Debug, Clone)]
pub enum WsCommand {
    /// Serialize any JSON-compatible payload and fire it down the wire.
    Send(Value),
}

/// Shared bridge state, injected into the component tree via Dioxus context.
///
/// All fields are `Copy` (lightweight handles into the Dioxus signal store),
/// so cloning this struct is virtually free.
#[derive(Clone, Copy)]
pub struct WsBridge {
    /// `true` while the WebSocket socket is open and alive.
    pub connected: Signal<bool>,

    /// Latest opaque game-state blob pushed from the server.
    /// Every write triggers automatic cascade re-renders in every consumer.
    pub opaque_state: Signal<Value>,

    /// Coroutine handle for shooting commands upstream.
    pub tx: Coroutine<WsCommand>,
}

impl WsBridge {
    /// Convenience: serialize any `Serialize`-able payload and send it upstream.
    pub fn send<T: serde::Serialize>(&self, payload: &T) {
        match serde_json::to_value(payload) {
            Ok(v) => {
                debug!(target: "ws::upstream", payload = %v, "发送动作");
                self.tx.send(WsCommand::Send(v))
            }
            Err(e) => {
                error!(target: "ws::upstream", error = %e, "序列化失败");
            }
        }
    }
}

// ═══════════════════════════════════════════════════════
//  Hook
// ═══════════════════════════════════════════════════════

/// Spin up the WebSocket bridge.
///
/// Call **once** from the Arena (game shell) component.
/// The returned `WsBridge` is also injected as context so all descendant
/// components can retrieve it via `use_context::<WsBridge>()`.
pub fn use_ws_bridge(room_id: &str, actor_id: &str) -> WsBridge {
    let mut connected = use_signal(|| false);
    let mut opaque_state = use_signal(|| Value::Null);

    let room = room_id.to_owned();
    let actor = actor_id.to_owned();

    let tx = use_coroutine(move |mut rx: UnboundedReceiver<WsCommand>| {
        let room = room.clone();
        let actor = actor.clone();
        async move {
            // ── Derive WS endpoint from backend origin ──
            let url = {
                let origin = crate::BACKEND_ORIGIN;
                let ws_origin = origin.replace("http://", "ws://").replace("https://", "wss://");
                format!("{ws_origin}/ws/{room}/{actor}")
            };

            info!(target: "ws", url = %url, room = %room, actor = %actor, "正在建立 WebSocket 连接...");

            // ── Open the socket ──
            let ws = match WebSocket::open(&url) {
                Ok(ws) => {
                    info!(target: "ws", url = %url, "WebSocket open() 成功，等待连接建立...");
                    ws
                }
                Err(e) => {
                    error!(target: "ws", error = ?e, url = %url, "WebSocket 连接失败");
                    return;
                }
            };

            let (mut sink, stream) = ws.split();
            let mut stream = stream.fuse();
            connected.set(true);

            info!(target: "ws", url = %url, "WebSocket 连接成功，双向管道已就绪");

            // ── Bidirectional pump ──
            loop {
                select! {
                    // ▼ Downstream: server → opaque_state signal
                    msg = stream.next() => {
                        match msg {
                            Some(Ok(Message::Text(text))) => {
                                match serde_json::from_str::<Value>(&text) {
                                    Ok(v) => {
                                        // 检查是否是私密消息（如手牌）
                                        if let Some(msg_type) = v.get("type").and_then(|t| t.as_str()) {
                                            if msg_type == "your_hand" {
                                                // 将手牌信息合并到当前状态
                                                if let Some(hand) = v.get("hand") {
                                                    let mut current = opaque_state.read().clone();
                                                    if current.is_object() {
                                                        current["your_hand"] = hand.clone();
                                                        let _ = opaque_state.set(current);
                                                        info!(target: "ws::downstream", "收到手牌私密消息，已合并到状态");
                                                    }
                                                }
                                                continue;
                                            }
                                        }
                                        
                                        let game_type = v.get("game_type").and_then(|g| g.as_str()).unwrap_or("?");
                                        let players_count = v.get("players").and_then(|p| p.as_array()).map(|a| a.len()).unwrap_or(0);
                                        let active = v.get("active_player")
                                            .or_else(|| v.get("active_actor"))
                                            .and_then(|a| a.as_str())
                                            .unwrap_or("?");
                                        info!(target: "ws::downstream", game_type, players_count, active, "收到状态快照");
                                        opaque_state.set(v);
                                    }
                                    Err(e) => {
                                        // 可能是 error JSON 或 room_closed 事件
                                        if text.contains("room_closed") {
                                            warn!(target: "ws", "收到房间关闭通知");
                                            connected.set(false);
                                            break;
                                        }
                                        warn!(target: "ws::downstream", error = %e, raw = %text, "JSON 解析失败");
                                    }
                                }
                            }
                            Some(Ok(Message::Bytes(_))) => {
                                warn!(target: "ws::downstream", "收到意外的二进制帧，已忽略");
                            }
                            Some(Err(e)) => {
                                error!(target: "ws", error = ?e, "WebSocket 流异常断开");
                                connected.set(false);
                                break;
                            }
                            None => {
                                info!(target: "ws", "服务器已关闭 WebSocket 连接（stream 返回 None）");
                                connected.set(false);
                                break;
                            }
                        }
                    }
                    // ▲ Upstream: component callback → server
                    cmd = rx.next() => {
                        match cmd {
                            Some(WsCommand::Send(value)) => {
                                match serde_json::to_string(&value) {
                                    Ok(text) => {
                                        if let Err(e) = sink.send(Message::Text(text.clone())).await {
                                            error!(target: "ws::upstream", error = ?e, payload = %text, "发送失败，连接可能已断开");
                                            connected.set(false);
                                            break;
                                        }
                                        debug!(target: "ws::upstream", payload = %text, "动作已发射");
                                    }
                                    Err(e) => {
                                        error!(target: "ws::upstream", error = %e, "JSON 序列化失败");
                                    }
                                }
                            }
                            None => break,
                        }
                    }
                }
            }

            // ── Graceful teardown ──
            let _ = sink.close().await;
            info!(target: "ws", "WebSocket 管道已优雅关闭");
        }
    });

    let bridge = WsBridge {
        connected,
        opaque_state,
        tx,
    };
    use_context_provider(|| bridge);
    bridge
}
