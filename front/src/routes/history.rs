use dioxus::prelude::*;
use crate::api::{get_history_rooms, set_room_public, delete_room, RoomSnapshotData};
use crate::games::registry::REGISTRY;
use crate::routes::layout::use_toast;

#[component]
pub fn History() -> Element {
    let toast = use_toast();
    let nav = use_navigator();
    let mut rooms = use_signal(|| Vec::<RoomSnapshotData>::new());
    let mut loading = use_signal(|| true);

    let mut load_rooms = move || {
        loading.set(true);
        spawn(async move {
            match get_history_rooms().await {
                Ok(r) => {
                    rooms.set(r);
                }
                Err(e) => {
                    toast.show(format!("获取历史对局失败: {e}"), crate::routes::layout::ToastType::Error);
                }
            }
            loading.set(false);
        });
    };

    use_effect(move || {
        let mut lr = load_rooms.clone();
        lr();
    });

    let toggle_public = move |room_id: String, current_public: bool| {
        let new_public = !current_public;
        spawn(async move {
            match set_room_public(&room_id, new_public).await {
                Ok(_) => {
                    toast.show("公开属性更新成功".to_string(), crate::routes::layout::ToastType::Success);
                    load_rooms();
                }
                Err(e) => {
                    toast.show(format!("更新失败: {e}"), crate::routes::layout::ToastType::Error);
                }
            }
        });
    };

    let handle_delete = move |room_id: String| {
        spawn(async move {
            match delete_room(&room_id).await {
                Ok(_) => {
                    toast.show("房间删除成功".to_string(), crate::routes::layout::ToastType::Success);
                    load_rooms();
                }
                Err(e) => {
                    toast.show(format!("删除失败: {e}"), crate::routes::layout::ToastType::Error);
                }
            }
        });
    };

    rsx! {
        div { class: "history-container animate-fade-in",
            div { class: "page-header",
                div { class: "header-left",
                    h1 { "📜 历史对局房间" }
                    p { "您创建的所有对局记录，支持设置公开展示属性。" }
                }
                button {
                    class: "refresh-btn-large glass-panel-subtle",
                    onclick: move |_| load_rooms(),
                    "🔄 刷新记录"
                }
            }

            if *loading.read() {
                div { class: "skeleton-list",
                    for _ in 0..4 {
                        div { class: "skeleton-item-large" }
                    }
                }
            } else if rooms.read().is_empty() {
                div { class: "empty-state-card glass-panel",
                    div { class: "empty-icon", "📜" }
                    h3 { "暂无对局历史" }
                    p { "您尚未创建过对局，快去大厅发起一场博弈吧！" }
                    button {
                        class: "go-lobby-btn",
                        onclick: move |_| { nav.push(super::Route::Lobby {}); },
                        "前往大厅"
                    }
                }
            } else {
                div { class: "history-list",
                    for room in rooms.read().iter() {
                        {
                            let rid = room.room_id.clone();
                            let is_pub = room.is_public;
                            let game_def = REGISTRY.get(&room.game_type);
                            let game_name = game_def.map(|g| g.name).unwrap_or("未知游戏");
                            let game_icon = game_def.map(|g| g.icon).unwrap_or("❓");
                            let rounds = room.max_round;
                            let time_str = room.created_at.chars().take(16).collect::<String>().replace("T", " ");

                            rsx! {
                                div { key: "{rid}", class: "history-card glass-panel animate-slide-up",
                                    div { class: "history-card-left",
                                        div { class: "game-badge", "{game_icon} {game_name}" }
                                        div { class: "room-id-mono", "ID: {rid}" }
                                    }

                                    div { class: "history-card-mid",
                                        div { class: "meta-item",
                                            span { class: "label", "总局数限制:" }
                                            span { class: "value", "{rounds} 轮" }
                                        }
                                        div { class: "meta-item",
                                            span { class: "label", "创建时间:" }
                                            span { class: "value", "{time_str}" }
                                        }
                                    }

                                    div { class: "history-card-right",
                                        // Toggle visibility switch
                                        {
                                            let rid_toggle = rid.clone();
                                            let rid_replay = rid.clone();
                                            let rid_spectate = rid.clone();
                                            let rid_delete = rid.clone();
                                            rsx! {
                                                div { class: "visibility-toggle-wrapper",
                                                    span { class: "visibility-label", "公开状态:" }
                                                    button {
                                                        class: if is_pub { "toggle-switch active" } else { "toggle-switch" },
                                                        onclick: move |_| toggle_public(rid_toggle.clone(), is_pub),
                                                        if is_pub { "公开中" } else { "私有" }
                                                    }
                                                }

                                                // Action buttons
                                                div { class: "actions-row",
                                                    button {
                                                        class: "action-btn replay",
                                                        onclick: move |_| {
                                                            nav.push(super::Route::Replay { room_id: rid_replay.clone() });
                                                        },
                                                        "🎞️ 回放"
                                                    }
                                                    button {
                                                        class: "action-btn spectate",
                                                        onclick: move |_| {
                                                            nav.push(super::Route::Game { room_id: rid_spectate.clone(), actor_id: "spectator".to_string() });
                                                        },
                                                        "👁️ 观战"
                                                    }
                                                    button {
                                                        class: "action-btn delete",
                                                        onclick: move |_| handle_delete(rid_delete.clone()),
                                                        "🗑️ 销毁"
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
