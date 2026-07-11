use crate::api::{get_public_rooms, join_room, RoomSnapshotData};
use crate::games::registry::REGISTRY;
use crate::routes::layout::use_toast;
use dioxus::prelude::*;

#[component]
pub fn PublicRooms() -> Element {
    let toast = use_toast();
    let nav = use_navigator();
    let mut rooms = use_signal(|| Vec::<RoomSnapshotData>::new());
    let mut loading = use_signal(|| true);

    let mut load_rooms = move || {
        loading.set(true);
        spawn(async move {
            match get_public_rooms().await {
                Ok(r) => {
                    rooms.set(r);
                }
                Err(e) => {
                    toast.show(
                        format!("获取公开房间失败: {e}"),
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

    rsx! {
        div { class: "pg-public animate-fade-in",
            div { class: "page-header",
                div { class: "header-left",
                    h1 { "🌐 广场公开对局" }
                    p { "浏览当前平台上所有人设为公开的活跃对局，你可以随时加入游戏或旁听观战。" }
                }
                button {
                    class: "refresh-btn-large g-card-subtle",
                    onclick: move |_| load_rooms(),
                    "🔄 刷新列表"
                }
            }

            if *loading.read() {
                div { class: "g-skeleton-grid",
                    for _ in 0..6 {
                        div { class: "g-skeleton-card" }
                    }
                }
            } else if rooms.read().is_empty() {
                div { class: "g-empty g-card",
                    div { class: "empty-icon", "🌐" }
                    h3 { "暂无活跃公开房间" }
                    p { "目前没有玩家公开他们的房间。您可以自己创建一个公开房间，等待别人加入！" }
                    button {
                        class: "go-lobby-btn",
                        onclick: move |_| { nav.push(super::Route::Lobby {}); },
                        "创建我的房间"
                    }
                }
            } else {
                div { class: "pg-public-grid",
                    for room in rooms.read().iter() {
                        {
                            let rid = room.room_id.clone();
                            let game_def = REGISTRY.get(&room.game_type);
                            let game_name = game_def.map(|g| g.name).unwrap_or("未知游戏");
                            let game_icon = game_def.map(|g| g.icon).unwrap_or("❓");
                            let rounds = room.max_round;
                            let time_str = room.created_at.chars().take(16).collect::<String>().replace("T", " ");

                            let empty_slots = if let Some(arr) = room.actor_slots.as_array() {
                                arr.iter().filter(|s| {
                                    s.get("occupant").and_then(|o| o.as_str()) == Some("Empty")
                                }).count()
                            } else {
                                0
                            };

                            // Precalculate first empty slot to avoid capturing `room` in the onclick closure!
                            let mut first_empty_slot = "spectator".to_string();
                            if let Some(arr) = room.actor_slots.as_array() {
                                for slot_val in arr {
                                    if slot_val.get("occupant").and_then(|o| o.as_str()) == Some("Empty") {
                                        if let Some(name) = slot_val.get("slot_name").and_then(|n| n.as_str()) {
                                            first_empty_slot = name.to_string();
                                            break;
                                        }
                                    }
                                }
                            }

                            rsx! {
                                div { key: "{rid}", class: "pg-public-card g-card animate-scale-up",
                                    div { class: "pg-public-banner",
                                        span { class: "game-icon-large", "{game_icon}" }
                                        h3 { "{game_name}" }
                                    }

                                    div { class: "pg-public-body",
                                        div { class: "room-id-badge", "ID: {rid}" }

                                        div { class: "slots-indicator-bar",
                                            div { class: "slots-label",
                                                span { "空余席位:" }
                                                span { class: "slots-count", "{empty_slots}" }
                                            }
                                        }

                                        div { class: "pg-public-meta-list",
                                            div { class: "meta-row",
                                                span { class: "label", "轮次限制" }
                                                span { class: "value", "{rounds} 轮" }
                                            }
                                            div { class: "meta-row",
                                                span { class: "label", "创建时间" }
                                                span { class: "value", "{time_str}" }
                                            }
                                        }
                                    }

                                    div { class: "pg-public-footer",
                                        if empty_slots > 0 {
                                            button {
                                                class: "pg-public-join primary",
                                                onclick: move |_| {
                                                    let rid = rid.clone();
                                                    let aid = first_empty_slot.clone();
                                                    let toast = toast.clone();
                                                    let nav = nav.clone();
                                                    spawn(async move {
                                                        match join_room(&rid, &aid).await {
                                                            Ok(_) => {
                                                                nav.push(super::Route::Game { room_id: rid, actor_id: aid });
                                                            }
                                                            Err(e) => {
                                                                toast.show(
                                                                    format!("加入失败: {}", e),
                                                                    crate::routes::layout::ToastType::Error,
                                                                );
                                                            }
                                                        }
                                                    });
                                                },
                                                "🎮 加入对局"
                                            }
                                        } else {
                                            button {
                                                class: "pg-public-join secondary",
                                                onclick: move |_| {
                                                    let rid = rid.clone();
                                                    let toast = toast.clone();
                                                    let nav = nav.clone();
                                                    spawn(async move {
                                                        match join_room(&rid, "spectator").await {
                                                            Ok(_) => {
                                                                nav.push(super::Route::Game { room_id: rid, actor_id: "spectator".to_string() });
                                                            }
                                                            Err(e) => {
                                                                toast.show(
                                                                    format!("加入失败: {}", e),
                                                                    crate::routes::layout::ToastType::Error,
                                                                );
                                                            }
                                                        }
                                                    });
                                                },
                                                "👁️ 观战旁听"
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
