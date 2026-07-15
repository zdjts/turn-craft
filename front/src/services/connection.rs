//! ConnectionManager — 全局 WebSocket 连接管理器
//!
//! 生命周期独立于路由：App 根组件初始化，所有页面复用同一条连接。
//! 路由切换只改变"当前消费的房间"，不断开 WS。

use dioxus::prelude::*;
use futures_util::{select, SinkExt, StreamExt};
use gloo_net::websocket::{futures::WebSocket, Message};
use serde_json::Value;
use std::collections::HashMap;
use tracing::{debug, error, info, warn};

const MAX_RETRY_DELAY_MS: u64 = 10_000;
const INITIAL_RETRY_DELAY_MS: u64 = 500;

// ═══════════════════════════════════════════════════════
//  Types
// ═══════════════════════════════════════════════════════

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ConnState {
    Disconnected,
    Connecting,
    Connected,
    Reconnecting { attempts: u32 },
}

impl ConnState {
    pub fn as_str(&self) -> &str {
        match self {
            ConnState::Disconnected => "未连接",
            ConnState::Connecting => "正在连接...",
            ConnState::Connected => "已连接",
            ConnState::Reconnecting { .. } => "重连中...",
        }
    }
}

/// Commands for the ConnectionManager coroutine.
enum ManagerCmd {
    Connect { room_id: String, actor_id: String },
    Disconnect,
    Send(Value),
}

/// Public handle, injected as Dioxus context.
#[derive(Clone, Copy)]
pub struct ConnectionManager {
    pub state: Signal<ConnState>,
    pub opaque_state: Signal<Value>,
    pub error_message: Signal<Option<String>>,
    pub streaming_text: Signal<HashMap<String, String>>,
    pub action_error: Signal<Option<String>>,
    pub can_retry: Signal<bool>,
    tx: Coroutine<ManagerCmd>,
}

impl ConnectionManager {
    /// Request connection to a room. If already connected to the same room, no-op.
    /// If a different room, disconnects first then reconnects.
    pub fn connect(&self, room_id: &str, actor_id: &str) {
        self.tx.send(ManagerCmd::Connect {
            room_id: room_id.to_owned(),
            actor_id: actor_id.to_owned(),
        });
    }

    /// Explicitly disconnect.
    pub fn disconnect(&self) {
        self.tx.send(ManagerCmd::Disconnect);
    }

    /// Send a payload upstream.
    pub fn send(&self, payload: &impl serde::Serialize) {
        match serde_json::to_value(payload) {
            Ok(v) => self.tx.send(ManagerCmd::Send(v)),
            Err(e) => error!(target: "conn", error = %e, "序列化失败"),
        }
    }
}

// ═══════════════════════════════════════════════════════
//  Initializer — call once from AppLayout
// ═══════════════════════════════════════════════════════

pub fn use_connection_manager() -> ConnectionManager {
    let mut state = use_signal(|| ConnState::Disconnected);
    let mut opaque_state = use_signal(|| Value::Null);
    let mut streaming_text = use_signal(|| HashMap::<String, String>::new());
    let mut error_message = use_signal(|| Option::<String>::None);
    let mut action_error = use_signal(|| Option::<String>::None);
    let mut can_retry = use_signal(|| false);

    let tx = use_coroutine(move |mut rx: UnboundedReceiver<ManagerCmd>| {
        async move {
            // --- Persistent connection variables (survive across reconnect) ---
            let mut current_room: Option<(String, String)> = None;
            let mut retry_delay = INITIAL_RETRY_DELAY_MS;
            let mut retry_count: u32 = 0;

            loop {
                let current_state = *state.read();
                match current_state {
                    ConnState::Disconnected => {
                        // Wait for a Connect command
                        retry_count = 0;
                        retry_delay = INITIAL_RETRY_DELAY_MS;
                        while let Some(cmd) = rx.next().await {
                            if let ManagerCmd::Connect { room_id, actor_id } = cmd {
                                current_room = Some((room_id.clone(), actor_id.clone()));
                                state.set(ConnState::Connecting);
                                break;
                            }
                        }
                    }

                    ConnState::Connecting | ConnState::Reconnecting { .. } => {
                        if let Some((ref room_id, ref actor_id)) = current_room {
                            debug!(target: "conn", room = %room_id, actor = %actor_id,
                                state = ?state.read(), "attempting WS connection");

                            let url = build_ws_url(room_id, actor_id);
                            info!(target: "conn", url = %url, "正在建立 WebSocket 连接...");

                            match WebSocket::open(&url) {
                                Ok(ws) => {
                                    info!(target: "conn", "WebSocket 连接成功");
                                    state.set(ConnState::Connected);
                                    retry_count = 0;
                                    retry_delay = INITIAL_RETRY_DELAY_MS;

                                    // --- Connected phase: bidirectional pump ---
                                    let (mut sink, stream) = ws.split();
                                    let mut stream = stream.fuse();

                                    'connected: loop {
                                        select! {
                                            // ▼ Downstream: server → UI signals
                                            msg = stream.next() => {
                                                match msg {
                                                    Some(Ok(Message::Text(text))) => {
                                                        handle_downstream(
                                                            &text,
                                                            &mut opaque_state,
                                                            &mut streaming_text,
                                                            &mut error_message,
                                                            &mut action_error,
                                                            &mut can_retry,
                                                        );
                                                    }
                                                    Some(Ok(Message::Bytes(_))) => {
                                                        warn!(target: "conn", "收到意外二进制帧");
                                                    }
                                                    Some(Err(e)) => {
                                                        error!(target: "conn", error = ?e, "WS 流异常断开");
                                                        state.set(ConnState::Reconnecting { attempts: retry_count });
                                                        break 'connected;
                                                    }
                                                    None => {
                                                        info!(target: "conn", "服务器关闭连接");
                                                        state.set(ConnState::Reconnecting { attempts: retry_count });
                                                        break 'connected;
                                                    }
                                                }
                                            }

                                            // ▲ Upstream: component → server
                                            cmd = rx.next() => {
                                                match cmd {
                                                    Some(ManagerCmd::Send(value)) => {
                                                        match serde_json::to_string(&value) {
                                                            Ok(text) => {
                                                                if let Err(e) = sink.send(Message::Text(text.clone())).await {
                                                                    error!(target: "conn", error = ?e, "发送失败");
                                                                    state.set(ConnState::Disconnected);
                                                                    break 'connected;
                                                                }
                                                                debug!(target: "conn", payload = %text, "已发送");
                                                            }
                                                            Err(e) => error!(target: "conn", error = %e, "序列化失败"),
                                                        }
                                                    }
                                                    Some(ManagerCmd::Disconnect) => {
                                                        info!(target: "conn", "收到断开指令");
                                                        state.set(ConnState::Disconnected);
                                                        let _ = sink.close().await;
                                                        break 'connected;
                                                    }
                                                    Some(ManagerCmd::Connect { room_id: new_room, actor_id: new_actor }) => {
                                                        if Some((&new_room, &new_actor)) != current_room.as_ref().map(|(r,a)| (r,a)) {
                                                            // Room changed → reconnect
                                                            info!(target: "conn", old_room = ?current_room, new_room = new_room, "切换房间");
                                                            current_room = Some((new_room, new_actor));
                                                            state.set(ConnState::Connecting);
                                                            let _ = sink.close().await;
                                                            break 'connected;
                                                        }
                                                        // Same room, ignore
                                                    }
                                                    None => break,
                                                }
                                            }
                                        }
                                    }
                                    // End of connected phase — will loop back to appropriate state
                                }
                                Err(e) => {
                                    error!(target: "conn", error = ?e, "WebSocket 连接失败");
                                    retry_count += 1;
                                    let ms = retry_delay.min(MAX_RETRY_DELAY_MS);
                                    state.set(ConnState::Reconnecting { attempts: retry_count });
                                    debug!(target: "conn", delay_ms = ms, retry = retry_count, "等待重连...");
                                    gloo_timers::future::sleep(std::time::Duration::from_millis(ms)).await;
                                    retry_delay = (retry_delay * 2).min(MAX_RETRY_DELAY_MS);
                                    state.set(ConnState::Connecting);
                                }
                            }
                        } else {
                            // No room configured, go back to disconnected
                            state.set(ConnState::Disconnected);
                        }
                    }

                    ConnState::Connected => {
                        // Shouldn't reach here normally (handled inside the connected loop)
                        if let Some(cmd) = rx.next().await {
                            match cmd {
                                ManagerCmd::Disconnect => state.set(ConnState::Disconnected),
                                _ => {} // ignore other commands while in unexpected state
                            }
                        }
                    }
                }
            }
        }
    });

    let manager = ConnectionManager {
        state,
        opaque_state,
        error_message,
        streaming_text,
        action_error,
        can_retry,
        tx,
    };
    use_context_provider(|| manager);
    manager
}

// ═══════════════════════════════════════════════════════
//  Helpers
// ═══════════════════════════════════════════════════════

fn build_ws_url(room_id: &str, actor_id: &str) -> String {
    let origin = crate::BACKEND_ORIGIN;
    let ws_origin = origin
        .replace("http://", "ws://")
        .replace("https://", "wss://");
    let token = crate::api::get_token().unwrap_or_default();
    format!("{ws_origin}/ws/{room_id}/{actor_id}?token={token}")
}

fn handle_downstream(
    text: &str,
    opaque_state: &mut Signal<Value>,
    streaming_text: &mut Signal<HashMap<String, String>>,
    error_message: &mut Signal<Option<String>>,
    action_error: &mut Signal<Option<String>>,
    can_retry: &mut Signal<bool>,
) {
    match serde_json::from_str::<Value>(text) {
        Ok(v) => {
            let msg_type = v.get("type").and_then(|t| t.as_str()).unwrap_or("");

            if msg_type == "your_hand" {
                if let Some(hand) = v.get("hand") {
                    let mut current = opaque_state.read().clone();
                    if current.is_object() {
                        current["your_hand"] = hand.clone();
                        opaque_state.set(current);
                        info!(target: "conn", "收到手牌私密消息");
                    }
                }
                return;
            }

            if msg_type == "stream_chunk" {
                if let Some(aid) = v.get("actor_id").and_then(|a| a.as_str()) {
                    if let Some(content) = v.get("content").and_then(|c| c.as_str()) {
                        streaming_text.write()
                            .entry(aid.to_string())
                            .or_default()
                            .push_str(content);
                    }
                }
                return;
            }
            if msg_type == "stream_done" {
                return;
            }

            if msg_type == "action_error" {
                let err = v.get("error").and_then(|e| e.as_str()).unwrap_or("操作失败");
                let retry = v.get("can_retry").and_then(|r| r.as_bool()).unwrap_or(false);
                warn!(target: "conn", error = err, "收到动作执行错误");
                error_message.set(Some(err.to_string()));
                action_error.set(Some(err.to_string()));
                can_retry.set(retry);
                return;
            }

            if msg_type == "ai_exhausted" {
                let err = v.get("error").and_then(|e| e.as_str()).unwrap_or("AI 重试耗尽");
                warn!(target: "conn", error = err, "AI 重试耗尽");
                action_error.set(Some(err.to_string()));
                can_retry.set(true);
                return;
            }

            // 显式 state 类型 — 标准状态快照
            if msg_type == "state" {
                let game_type = v.get("game_type").and_then(|g| g.as_str()).unwrap_or("?");
                let active = v.get("active_player")
                    .or_else(|| v.get("active_actor"))
                    .and_then(|a| a.as_str())
                    .unwrap_or("?");
                info!(target: "conn", game_type, active, "收到状态快照");
                error_message.set(None);
                action_error.set(None);
                can_retry.set(false);
                opaque_state.set(v);
                streaming_text.write().clear();
                return;
            }

            // 对无 type 的消息，兼容旧版错误格式 { "error": "..." }
            if msg_type.is_empty() {
                if let Some(err_msg) = v.get("error").and_then(|e| e.as_str()) {
                    warn!(target: "conn", error = err_msg, "收到服务器错误");
                    error_message.set(Some(err_msg.to_string()));
                    return;
                }
                // 旧版无 type 的状态快照（回退检测 game_type）
                let game_type = v.get("game_type").and_then(|g| g.as_str());
                if game_type.is_some() {
                    info!(target: "conn", game_type = ?game_type, "收到旧版状态快照（无 type）");
                    opaque_state.set(v);
                    streaming_text.write().clear();
                    return;
                }
            }
        }
        Err(e) => {
            if text.contains("room_closed") {
                warn!(target: "conn", "收到房间关闭通知");
            }
            warn!(target: "conn", error = %e, raw = %text, "JSON 解析失败");
        }
    }
}
