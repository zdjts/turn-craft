use dioxus::prelude::*;
use serde_json::Value;
use crate::services::websocket::use_ws_bridge;
use crate::games::{GamePluginManager, GamePluginProps};
use crate::routes::layout::use_toast;

#[component]
pub fn Game(room_id: String, actor_id: String) -> Element {
    let toast = use_toast();
    let nav = use_navigator();
    
    // Connect websocket bridge
    let bridge = use_ws_bridge(&room_id, &actor_id);
    let connected = bridge.connected;
    let state = bridge.opaque_state;

    let rid_for_copy = room_id.clone();
    let aid_for_plugin = actor_id.clone();

    // Handle copying room ID to clipboard
    let copy_room_id = move |_| {
        if let Some(win) = web_sys::window() {
            let nav = win.navigator().clipboard();
            let _ = nav.write_text(&rid_for_copy);
            toast.show("房间 ID 已复制到剪贴板".to_string(), crate::routes::layout::ToastType::Success);
        }
    };

    // Extract players and game info from opaque state
    let game_type = use_memo(move || {
        let s = state.read();
        s.get("game_type")
            .and_then(|g| g.as_str())
            .unwrap_or("unknown")
            .to_string()
    });

    let max_round = use_memo(move || {
        let s = state.read();
        s.get("max_round")
            .and_then(|r| r.as_u64())
            .unwrap_or(0)
    });

    let current_round = use_memo(move || {
        let s = state.read();
        s.get("round")
            .and_then(|r| r.as_u64())
            .unwrap_or(0)
    });

    let finished = use_memo(move || {
        let s = state.read();
        s.get("finished")
            .and_then(|f| f.as_bool())
            .unwrap_or(false)
    });

    let active_actor = use_memo(move || {
        let s = state.read();
        s.get("active_actor")
            .or_else(|| s.get("active_player"))
            .and_then(|a| a.as_str())
            .map(|s| s.to_string())
    });

    // Parse player roster slots
    let roster_slots = use_memo(move || {
        let mut list = Vec::new();
        let s = state.read();
        if let Some(actors) = s.get("actors").and_then(|a| a.as_array()) {
            // Lincoln style roster
            for act in actors {
                if let (Some(id), Some(role), Some(kind)) = (
                    act.get("id").and_then(|v| v.as_str()),
                    act.get("role").and_then(|v| v.as_str()),
                    act.get("kind").and_then(|v| v.as_str()),
                ) {
                    list.push((id.to_string(), role.to_string(), kind.to_string()));
                }
            }
        } else if let Some(players) = s.get("players").and_then(|p| p.as_array()) {
            // Texas holdem style roster
            for p in players {
                if let (Some(id), Some(pos), Some(kind)) = (
                    p.get("id").and_then(|v| v.as_str()),
                    p.get("position").or_else(|| p.get("role")).and_then(|v| v.as_str()),
                    p.get("kind").and_then(|v| v.as_str()),
                ) {
                    list.push((id.to_string(), pos.to_string(), kind.to_string()));
                }
            }
        }
        list
    });

    rsx! {
        div { class: "arena-shell",
            // ── Left Side Glass Control Panel ──
            div { class: "arena-sidebar glass-panel",
                // Room Info Header
                div { class: "arena-room-card",
                    div { class: "room-row-top",
                        span { class: "room-label", "游戏房间" }
                        button {
                            class: "copy-btn",
                            onclick: copy_room_id,
                            title: "复制房间ID",
                            "📋"
                        }
                    }
                    div { class: "room-id-mono", "{room_id}" }
                    
                    // Connection Status
                    div { class: "connection-status-row",
                        div {
                            class: if *connected.read() { "status-dot online" } else { "status-dot offline" }
                        }
                        span { class: "status-text",
                            if *connected.read() { "已连接" } else { "连接断开，正在重试..." }
                        }
                    }
                }

                // AI Configuration jumps
                div { class: "sidebar-actions-panel",
                    h4 { class: "roster-title", "⚙️ AI 助手配置" }
                    div { class: "ai-config-buttons-list",
                        {
                            let ai_slots: Vec<_> = roster_slots().iter()
                                .filter(|(_, _, kind)| kind.to_lowercase() == "ai")
                                .cloned()
                                .collect();
                            
                            if ai_slots.is_empty() {
                                rsx! {
                                    div { class: "empty-ai-configs-hint", "本局无 AI 参与者" }
                                }
                            } else {
                                rsx! {
                                    for (id, role, _) in ai_slots {
                                        {
                                            let rid = room_id.clone();
                                            let aid = id.clone();
                                            let role_display = role.clone();
                                            let nav = nav.clone();
                                            rsx! {
                                                button {
                                                    key: "{aid}",
                                                    class: "jump-settings-btn glass-panel-subtle",
                                                    style: "margin-bottom: 8px;",
                                                    onclick: move |_| {
                                                        nav.push(super::Route::Settings { room_id: rid.clone(), actor_id: aid.clone() });
                                                    },
                                                    "配置 {role_display} AI"
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // Player Roster
                div { class: "arena-roster-section",
                    h4 { class: "roster-title", "👥 对局参与者" }
                    div { class: "roster-list",
                        if roster_slots().is_empty() {
                            div { class: "empty-roster-msg", "等待玩家信息..." }
                        } else {
                            for (id, role, kind) in roster_slots().iter() {
                                {
                                    let is_active = active_actor().as_ref() == Some(id);
                                    let is_ai = kind == "ai" || kind == "Ai";
                                    let role_class = match role.as_str() {
                                        "Judge" => "role-judge",
                                        "Pro" => "role-pro",
                                        "Con" => "role-con",
                                        _ => "role-player",
                                    };
                                    rsx! {
                                        div {
                                            key: "{id}",
                                            class: if is_active { "player-roster-card active" } else { "player-roster-card" },
                                            div { class: "player-avatar-mini {role_class}",
                                                if is_ai { "🤖" } else { "🤵" }
                                            }
                                            div { class: "player-roster-details",
                                                div { class: "player-name-lbl", "{id}" }
                                                div { class: "player-role-lbl", "{role}" }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // Bottom Control / Return
                div { class: "sidebar-bottom-controls",
                    button {
                        class: "leave-arena-btn",
                        onclick: move |_| {
                            nav.push(super::Route::Lobby {});
                        },
                        "🚪 返回大厅"
                    }
                }
            }

            // ── Right Viewport ──
            div { class: "arena-viewport",
                if !*connected.read() {
                    div { class: "loading-canvas glass-panel",
                        span { class: "spinner" }
                        h3 { "正在建立网络连接" }
                        p { "正在通过 WebSocket 连接至对局服务器..." }
                    }
                } else if state.read().is_null() {
                    div { class: "loading-canvas glass-panel",
                        div { class: "skeleton-canvas animate-pulse" }
                        h3 { "正在获取对局快照" }
                        p { "已连接，正在同步初始状态数据..." }
                    }
                } else {
                    // Dynamically hand off to child game plugin
                    div { class: "game-plugin-container glass-panel",
                        GamePluginManager {
                            game_type: game_type(),
                            props: GamePluginProps {
                                state: state,
                                on_action: Callback::new(move |act| bridge.send(&act)),
                                actor_id: aid_for_plugin.clone(),
                            }
                        }
                    }
                }
            }
        }
    }
}
