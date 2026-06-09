use dioxus::prelude::*;
use serde_json::Value;
use tracing::{debug, info};

use crate::games::GamePluginManager;
use crate::services::websocket::{use_ws_bridge, WsBridge};

#[component]
pub fn Game(room_id: String, actor_id: String) -> Element {
    info!(target: "game", room_id = %room_id, actor_id = %actor_id, "进入游戏外壳，正在建立 WebSocket 连接...");
    let bridge = use_ws_bridge(&room_id, &actor_id);

    let game_type = use_memo(move || {
        let gt = bridge
            .opaque_state
            .read()
            .get("game_type")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();
        debug!(target: "game", game_type = %gt, "游戏类型已识别");
        gt
    });

    rsx! {
        div { class: "app-shell",
            aside { class: "sidebar",
                RoomCard { room_id: room_id.clone(), actor_id: actor_id.clone() }
                PlayerRoster {}
            }

            main { class: "game-viewport",
                GamePluginManager {
                    game_type: game_type(),
                    props: crate::games::GamePluginProps {
                        state: bridge.opaque_state,
                        on_action: Callback::new(move |action: Value| {
                            info!(target: "game", action = %action, "用户发射动作");
                            bridge.send(&action);
                        }),
                        actor_id: actor_id.clone(),
                    },
                }
            }
        }
    }
}

#[component]
fn RoomCard(room_id: String, actor_id: String) -> Element {
    let bridge = use_context::<WsBridge>();
    let connected = bridge.connected;
    let navigator = use_navigator();

    rsx! {
        div { class: "room-card",
            div { class: "room-header",
                span { class: "room-id", "🏠 {room_id}" }
                {
                    let rid_copy = room_id.clone();
                    rsx! {
                        button {
                            class: "copy-btn",
                            title: "复制房间号",
                            onclick: move |_| {
                                if let Some(win) = web_sys::window() {
                                    let _ = win.navigator().clipboard().write_text(&rid_copy);
                                    debug!(target: "game", room_id = %rid_copy, "房间号已复制到剪贴板");
                                }
                            },
                            "📋"
                        }
                    }
                }
            }
            div { class: "room-meta",
                span { class: "room-actor-id", "👤 {actor_id}" }
            }
            div { class: "connection-status",
                if connected() {
                    span { class: "status-dot online" }
                    span { class: "status-text", "已连接" }
                } else {
                    span { class: "status-dot offline" }
                    span { class: "status-text", "断线中..." }
                }
            }
            {
                let rid = room_id.clone();
                let aid = actor_id.clone();
                rsx! {
                    button {
                        class: "settings-link",
                        onclick: move |_| {
                            navigator.push(format!("/settings/{}/{}", rid, aid));
                        },
                        "⚙️ AI 配置"
                    }
                }
            }
        }
    }
}

#[component]
fn PlayerRoster() -> Element {
    let bridge = use_context::<WsBridge>();
    let state = bridge.opaque_state;

    let actors = use_memo(move || {
        let arr = state()
            .get("actors")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        debug!(target: "game::roster", count = arr.len(), "选手花名册已更新");
        arr
    });

    let active_actor = use_memo(move || {
        state()
            .get("active_actor")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string()
    });

    rsx! {
        div { class: "player-roster",
            h3 { class: "roster-title", "选手花名册" }
            for actor in actors().iter() {
                PlayerCard {
                    key: "{actor.get(\"id\").and_then(|v| v.as_str()).unwrap_or(\"?\")}",
                    actor: actor.clone(),
                    is_active: active_actor().as_str()
                        == actor.get("id")
                            .and_then(|v| v.as_str())
                            .unwrap_or(""),
                }
            }
            if actors().is_empty() {
                div { class: "empty-roster",
                    span { class: "empty-hint", "等待玩家加入..." }
                }
            }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
struct PlayerCardProps {
    actor: Value,
    is_active: bool,
}

#[component]
fn PlayerCard(props: PlayerCardProps) -> Element {
    let id = props.actor.get("id").and_then(|v| v.as_str()).unwrap_or("?");
    let kind = props.actor.get("kind").and_then(|v| v.as_str()).unwrap_or("");
    let role = props.actor.get("role").and_then(|v| v.as_str()).unwrap_or("");

    let icon = if kind == "Ai" { "🤖" } else { "👑" };
    let card_class = if props.is_active { "player-card active" } else { "player-card inactive" };

    rsx! {
        div { class: "{card_class}",
            span { class: "player-icon", "{icon}" }
            div { class: "player-info",
                span { class: "player-name", "{id}" }
                span { class: "player-role", "{role}" }
            }
            if props.is_active {
                span { class: "active-badge", "ACTIVE" }
            }
        }
    }
}
