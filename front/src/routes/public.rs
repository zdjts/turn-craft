use dioxus::prelude::*;
use crate::api::{get_public_rooms, RoomSnapshotData};
use crate::games::registry::REGISTRY;
use crate::routes::layout::use_toast;

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
                    toast.show(format!("获取公开房间失败: {e}"), crate::routes::layout::ToastType::Error);
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
        div { class: "public-rooms-page animate-fade-in",
            div { class: "page-header",
                div { class: "header-left",
                    h1 { "🌐 广场公开对局" }
                    p { "浏览当前平台上所有人设为公开的活跃对局，你可以随时加入游戏或旁听观战。" }
                }
                button {
                    class: "refresh-btn-large glass-panel-subtle",
                    onclick: move |_| load_rooms(),
                    "🔄 刷新列表"
                }
            }

            if *loading.read() {
                div { class: "skeleton-grid",
                    for _ in 0..6 {
                        div { class: "skeleton-card-grid" }
                    }
                }
            } else if rooms.read().is_empty() {
                div { class: "empty-state-card glass-panel",
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
                div { class: "public-rooms-grid",
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
                                    s.get("occupant").and_then(|o| o.get("Empty")).is_some()
                                }).count()
                            } else {
                                0
                            };

                            // Precalculate first empty slot to avoid capturing `room` in the onclick closure!
                            let mut first_empty_slot = "spectator".to_string();
                            if let Some(arr) = room.actor_slots.as_array() {
                                for slot_val in arr {
                                    if slot_val.get("occupant").and_then(|o| o.get("Empty")).is_some() {
                                        if let Some(name) = slot_val.get("slot_name").and_then(|n| n.as_str()) {
                                            first_empty_slot = name.to_string();
                                            break;
                                        }
                                    }
                                }
                            }

                            rsx! {
                                div { key: "{rid}", class: "public-grid-card glass-panel animate-scale-up",
                                    div { class: "grid-card-banner",
                                        span { class: "game-icon-large", "{game_icon}" }
                                        h3 { "{game_name}" }
                                    }

                                    div { class: "grid-card-body",
                                        div { class: "room-id-badge", "ID: {rid}" }
                                        
                                        div { class: "slots-indicator-bar",
                                            div { class: "slots-label",
                                                span { "空余席位:" }
                                                span { class: "slots-count", "{empty_slots}" }
                                            }
                                        }

                                        div { class: "grid-meta-list",
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

                                    div { class: "grid-card-footer",
                                        if empty_slots > 0 {
                                            button {
                                                class: "grid-join-btn primary",
                                                onclick: move |_| {
                                                    nav.push(super::Route::Game { room_id: rid.clone(), actor_id: first_empty_slot.clone() });
                                                },
                                                "🎮 加入对局"
                                            }
                                        } else {
                                            button {
                                                class: "grid-join-btn secondary",
                                                onclick: move |_| {
                                                    nav.push(super::Route::Game { room_id: rid.clone(), actor_id: "spectator".to_string() });
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
