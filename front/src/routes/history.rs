use crate::api::{delete_room, get_history_rooms, set_room_public, RoomSnapshotData};
use crate::games::registry::REGISTRY;
use crate::routes::layout::use_toast;
use dioxus::prelude::*;

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
                    toast.show(
                        format!("获取历史对局失败: {e}"),
                        crate::routes::layout::ToastType::Error,
                    );
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
                    toast.show(
                        "公开属性更新成功".to_string(),
                        crate::routes::layout::ToastType::Success,
                    );
                    load_rooms();
                }
                Err(e) => {
                    toast.show(
                        format!("更新失败: {e}"),
                        crate::routes::layout::ToastType::Error,
                    );
                }
            }
        });
    };

    let mut delete_confirm = use_signal(|| Option::<String>::None);

    let mut handle_delete = move |room_id: String| {
        delete_confirm.set(Some(room_id));
    };

    let confirm_delete = move || {
        let room_to_delete = delete_confirm.read().clone();
        delete_confirm.set(None);
        if let Some(room_id) = room_to_delete {
            spawn(async move {
                match delete_room(&room_id).await {
                    Ok(_) => {
                        toast.show(
                            "房间删除成功".to_string(),
                            crate::routes::layout::ToastType::Success,
                        );
                        load_rooms();
                    }
                    Err(e) => {
                        toast.show(
                            format!("删除失败: {e}"),
                            crate::routes::layout::ToastType::Error,
                        );
                    }
                }
            });
        }
    };

    let cancel_delete = move || {
        delete_confirm.set(None);
    };

    rsx! {
        div { class: "pg-history animate-fade-in",
            div { class: "page-header",
                div { class: "header-left",
                    h1 { "📜 历史对局房间" }
                    p { "您创建的所有对局记录，支持设置公开展示属性。" }
                }
                button {
                    class: "refresh-btn-large g-card-subtle",
                    onclick: move |_| load_rooms(),
                    "🔄 刷新记录"
                }
            }

            if *loading.read() {
                div { class: "g-skeleton-list",
                    for _ in 0..4 {
                        div { class: "g-skeleton-row-lg" }
                    }
                }
            } else if rooms.read().is_empty() {
                div { class: "g-empty g-card",
                    div { class: "empty-icon", "📜" }
                    h3 { "暂无对局历史" }
                    p { "您尚未创建过对局，快去大厅发起一场博弈吧！" }
                    button {
                        class: "pg-history-go",
                        onclick: move |_| { nav.push(super::Route::Lobby {}); },
                        "前往大厅"
                    }
                }
            } else {
                div { class: "pg-history-list",
                    for room in rooms.read().iter() {
                        {
                            let rid = room.room_id.clone();
                            let is_pub = room.is_public;
                            let game_def = REGISTRY.get(&room.game_type);
                            let game_name = game_def.map(|g| g.name).unwrap_or("未知游戏");
                            let game_icon = game_def.map(|g| g.icon).unwrap_or("❓");
                            let rounds = room.max_round;
                            let time_str = room.created_at.chars().take(16).collect::<String>().replace("T", " ");
                            let is_done = room.engine_state.get("finished").and_then(|v| v.as_bool()).unwrap_or(false);

                            rsx! {
                                div { key: "{rid}", class: "pg-history-card g-card animate-slide-up",
                                    div { class: "pg-pg-history-card-left",
                                        div { class: "game-badge", "{game_icon} {game_name}" }
                                        div { class: "room-id-mono", "ID: {rid}" }
                                    }

                                    div { class: "pg-pg-history-card-mid",
                                        div { class: "meta-item",
                                            span { class: "label", "状态:" }
                                            span { class: "value", if is_done { "✅ 已结算" } else { "⏳ 未完成" } }
                                        }
                                        div { class: "meta-item",
                                            span { class: "label", "总局数限制:" }
                                            span { class: "value", "{rounds} 轮" }
                                        }
                                        div { class: "meta-item",
                                            span { class: "label", "创建时间:" }
                                            span { class: "value", "{time_str}" }
                                        }
                                    }

                                    div { class: "pg-pg-history-card-right",
                                        // Toggle visibility switch
                                        {
                                            let rid_toggle = rid.clone();
                                            let rid_replay = rid.clone();
                                            let rid_spectate = rid.clone();
                                            let rid_delete = rid.clone();
                                            rsx! {
                                                div { class: "pg-history-visibility",
                                                    span { class: "pg-history-vis-label", "公开状态:" }
                                                    button {
                                                        class: if is_pub { "pg-history-toggle is-active" } else { "pg-history-toggle" },
                                                        onclick: move |_| toggle_public(rid_toggle.clone(), is_pub),
                                                        if is_pub { "公开中 ▸" } else { "私有 ▸" }
                                                    }
                                                }

                                                // Action buttons
                                                div { class: "actions-row",
                                                    button {
                                                        class: "g-btn-ghost replay",
                                                        onclick: move |_| {
                                                            nav.push(super::Route::Replay { room_id: rid_replay.clone() });
                                                        },
                                                        "🎞️ 回放"
                                                    }
                                                    button {
                                                        class: "g-btn-ghost spectate",
                                                        onclick: move |_| {
                                                            nav.push(super::Route::Game { room_id: rid_spectate.clone(), actor_id: "spectator".to_string() });
                                                        },
                                                        "👁️ 观战"
                                                    }
                                                    button {
                                                        class: "g-btn-ghost delete",
                                                        onclick: move |_| handle_delete(rid_delete.clone()),
                                                        "🗑️ 销毁"
                }
            }

            if let Some(ref room_id) = *delete_confirm.read() {
                {
                    let mut confirm = confirm_delete.clone();
                    let mut cancel = cancel_delete.clone();
                    let rid = room_id.clone();
                    rsx! {
                        div { class: "modal-overlay",
                            div { class: "modal-confirm g-card",
                                h3 { "确认删除" }
                                p { "确定要永久删除房间 {rid} 吗？此操作不可撤销。" }
                                div { class: "modal-actions",
                                    button {
                                        class: "modal-btn cancel",
                                        onclick: move |_| cancel(),
                                        "取消"
                                    }
                                    button {
                                        class: "modal-btn confirm-delete",
                                        onclick: move |_| confirm(),
                                        "确认删除"
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
                }
            }
        }
    }
}
