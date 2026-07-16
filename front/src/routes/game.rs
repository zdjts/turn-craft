use crate::games::{GamePluginManager, GamePluginProps};
use crate::routes::layout::use_toast;
use crate::services::connection::{ConnState, ConnectionManager};
use dioxus::prelude::*;
use serde_json::json;

#[component]
pub fn Game(room_id: String, actor_id: String) -> Element {
    let toast = use_toast();
    let nav = use_navigator();
    let mut conn = use_context::<ConnectionManager>();

    // Bind connection on mount
    {
        let rid = room_id.clone();
        let aid = actor_id.clone();
        use_effect(move || {
            conn.connect(&rid, &aid);
        });
    }

    let conn_state = conn.state;
    let connected = use_memo(move || *conn_state.read() == ConnState::Connected);
    let state = conn.opaque_state;

    let rid_for_copy = room_id.clone();
    let aid_for_plugin = actor_id.clone();

    let copy_room_id = move |_| {
        if let Some(win) = web_sys::window() {
            let clip = win.navigator().clipboard();
            let _ = clip.write_text(&rid_for_copy);
            toast.show(
                "房间 ID 已复制到剪贴板".to_string(),
                crate::routes::layout::ToastType::Success,
            );
        }
    };

    let game_type = use_memo(move || {
        state.read().get("game_type")
            .and_then(|g| g.as_str())
            .unwrap_or("unknown")
            .to_string()
    });

    let active_actor = use_memo(move || {
        let s = state.read();
        s.get("active_actor")
            .or_else(|| s.get("active_player"))
            .and_then(|a| a.as_str())
            .map(|s| s.to_string())
    });

    let roster_slots = use_memo(move || {
        let mut list = Vec::new();
        let s = state.read();
        if let Some(actors) = s.get("actors").and_then(|a| a.as_array()) {
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
            for p in players {
                if let (Some(id), Some(kind)) = (
                    p.get("id").and_then(|v| v.as_str()),
                    p.get("kind").and_then(|v| v.as_str()),
                ) {
                    let pos = p.get("position")
                        .or_else(|| p.get("role"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("未知");
                    list.push((id.to_string(), pos.to_string(), kind.to_string()));
                }
            }
        }
        list
    });

    let is_closed = use_memo(move || *conn_state.read() == ConnState::Closed);
    let conn_state_label = use_memo(move || conn_state.read().as_str().to_string());
    let has_action_error = use_memo(move || conn.action_error.read().is_some());
    let current_error = use_memo(move || conn.action_error.read().clone());
    let can_retry_val = use_memo(move || *conn.can_retry.read());

    // 回合指示
    let turn_info = use_memo(move || {
        let s = state.read();
        let active = s.get("active_actor")
            .or_else(|| s.get("active_player"))
            .and_then(|a| a.as_str());
        let phase = s.get("phase").or_else(|| s.get("cur_role"))
            .or_else(|| s.get("phase_hint"))
            .and_then(|p| p.as_str());
        let round = s.get("round").and_then(|r| r.as_u64());
        (active.map(|s| s.to_string()), phase.map(|s| s.to_string()), round)
    });

    let is_my_turn = use_memo(move || {
        turn_info().0.as_deref() == Some(&actor_id)
    });

    rsx! {
        div { class: "pg-arena",
            div { class: "pg-arena-sidebar g-card",
                div { class: "pg-arena-info",
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

                    div { class: "pg-arena-conn",
                        div {
                            class: if connected() { "status-dot online" } else { "status-dot offline" }
                        }
                        span { class: "status-text", "{conn_state_label}" }
                    }

                    // 回合状态指示
                    if connected() {
                        div { class: "pg-arena-turn-info",
                            div { class: "pg-arena-turn-info-row",
                                if let (Some(active), Some(phase)) = (turn_info().0.as_ref(), turn_info().1.as_ref()) {
                                    span { class: "turn-label",
                                        if is_my_turn() {
                                            "🎯 你的回合"
                                        } else {
                                            "⏳ {active} 的回合"
                                        }
                                    }
                                    span { class: "phase-label", "{phase}" }
                                } else if let (Some(active), None) = (turn_info().0.as_ref(), turn_info().1.as_ref()) {
                                    span { class: "turn-label", "⏳ {active} 的回合" }
                                } else {
                                    span { class: "turn-label", "等待开始..." }
                                }
                            }
                            if let Some(r) = turn_info().2 {
                                div { class: "round-label", "第 {r} 轮" }
                            }
                        }
                    }

                    // AI 失败重试提示
                    if has_action_error() {
                        div { class: "retry-banner",
                            div { class: "retry-banner-content",
                                span { "⚠️ 操作失败: {current_error:?}" }
                                div { class: "retry-banner-actions",
                                    if can_retry_val() {
                                        button {
                                            class: "retry-btn",
                                            onclick: move |_| {
                                                conn.send(&json!({"type": "retry"}));
                                                conn.action_error.set(None);
                                            },
                                            "🔄 重试"
                                        }
                                    }
                                    button {
                                        class: "skip-btn",
                                        onclick: move |_| {
                                            conn.send(&json!({"type": "skip"}));
                                            conn.action_error.set(None);
                                        },
                                        "⏭️ 跳过"
                                    }
                                }
                            }
                        }
                    }
                }

                div { class: "pg-arena-actions",
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
                                                    class: "pg-arena-jump g-card-subtle",
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

                div { class: "pg-arena-roster",
                    h4 { class: "roster-title", "👥 对局参与者" }
                    div { class: "pg-arena-roster-list",
                        if roster_slots().is_empty() {
                            div { class: "pg-arena-empty", "等待玩家信息..." }
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
                                            class: if is_active { "gm-roster-player is-active" } else { "gm-roster-player" },
                                            div { class: "pg-arena-player-avatar {role_class}",
                                                if is_ai { "🤖" } else { "🤵" }
                                            }
                                            div { class: "pg-arena-player-details",
                                                div { class: "pg-arena-player-name", "{id}" }
                                                div { class: "pg-arena-player-role", "{role}" }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                div { class: "pg-arena-controls",
                    button {
                        class: "pg-arena-leave",
                        onclick: move |_| {
                            conn.disconnect();
                            nav.push(super::Route::Lobby {});
                        },
                        "🚪 返回大厅"
                    }
                }
            }

            div { class: "pg-arena-viewport",
                if is_closed() {
                    div { class: "loading-canvas g-card",
                        div { class: "closed-icon", style: "font-size: 3rem; text-align: center;", "🚪" }
                        h3 { "房间已关闭" }
                        p { "该房间不存在或已结束，请返回大厅。" }
                        button {
                            class: "pg-arena-leave",
                            style: "margin-top: 16px;",
                            onclick: move |_| { nav.push(super::Route::Lobby {}); },
                            "← 返回大厅"
                        }
                    }
                } else if !connected() {
                    div { class: "loading-canvas g-card",
                        span { class: "g-spinner" }
                        h3 { "正在建立网络连接" }
                        p { "正在通过 WebSocket 连接至对局服务器..." }
                    }
                } else if state.read().is_null() {
                    div { class: "loading-canvas g-card",
                        div { class: "skeleton-canvas animate-pulse" }
                        h3 { "正在获取对局快照" }
                        p { "已连接，正在同步初始状态数据..." }
                    }
                } else {
                    div { class: "game-plugin-container g-card",
                        GamePluginManager {
                            game_type: game_type(),
                            props: GamePluginProps {
                                state,
                                on_action: Callback::new(move |act| conn.send(&act)),
                                actor_id: aid_for_plugin.clone(),
                            }
                        }
                    }
                }
            }
        }
    }
}
