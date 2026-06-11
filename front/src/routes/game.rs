use dioxus::html::h1;
use dioxus::prelude::*;
use serde_json::Value;
use tracing::{debug, info};

use crate::games::GamePluginManager;
use crate::routes::game_actions::{copy_room_id, open_settings};
use crate::services::websocket::{use_ws_bridge, WsBridge};

/// 游戏页面组件：管理 WebSocket 连接和游戏渲染
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

/// 房间信息卡片：显示房间 ID、连接状态
#[component]
fn RoomCard(room_id: String, actor_id: String) -> Element {
    let bridge = use_context::<WsBridge>();
    let connected = bridge.connected;
    let navigator = use_navigator();

    rsx! {
    button {
        class: "back-to-lobby-btn",
        onclick: move |_| {
            navigator.push("/");
        },
        "返回大厅"
    }
        div { class: "room-card",
            div { class: "room-header",
                span { class: "room-id", "🏠 {room_id}" }
                {
                    let rid_copy = room_id.clone();
                    rsx! {
                            button {
                                class: "copy-btn",
                                title: "复制房间号",
                                onclick: move |_| copy_room_id(rid_copy.clone()),
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
                        onclick: move |_| open_settings(navigator, rid.clone(), aid.clone()),
                        "⚙️ AI 配置"
                    }
                }
            }
        }
    }
}

/// 选手花名册：显示所有参与者
#[component]
fn PlayerRoster() -> Element {
    let bridge = use_context::<WsBridge>();
    let state = bridge.opaque_state;

    let actors = use_memo(move || {
        let s = state();
        debug!(target: "game::roster", state = %s, "收到状态");
        // 支持 actors（林肯辩论）和 players（德州扑克）两种字段名
        let arr = s
            .get("actors")
            .or_else(|| s.get("players"))
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        debug!(target: "game::roster", count = arr.len(), "选手花名册已更新");
        arr
    });

    let active_actor = use_memo(move || {
        let s = state();
        // 支持 active_actor 和 active_player 两种字段名
        s.get("active_actor")
            .or_else(|| s.get("active_player"))
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

/// 选手卡片属性
#[derive(Props, Clone, PartialEq)]
struct PlayerCardProps {
    actor: Value,
    is_active: bool,
}

/// 选手卡片组件：显示单个参与者信息
#[component]
fn PlayerCard(props: PlayerCardProps) -> Element {
    let id = props
        .actor
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("?");
    let kind = props
        .actor
        .get("kind")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let role = props
        .actor
        .get("role")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let icon = if kind == "Ai" { "🤖" } else { "👑" };
    let card_class = if props.is_active {
        "player-card active"
    } else {
        "player-card inactive"
    };

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
