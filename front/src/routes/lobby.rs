use crate::api::{create_room, get_public_rooms, CreateRoomRequest, RoomSnapshotData};
use crate::games::registry::{GameConfigProps, REGISTRY};
use crate::routes::layout::use_toast;
use dioxus::prelude::*;
use serde_json::Value;
use std::collections::HashMap;

#[component]
fn DynamicLobbyCard(game_type: String, props: GameConfigProps) -> Element {
    if let Some(def) = REGISTRY.get(&game_type) {
        let Comp = def.lobby_card;
        rsx! {
            Comp {
                role_config: props.role_config,
                my_role: props.my_role,
                max_round: props.max_round,
                game_config: props.game_config,
            }
        }
    } else {
        rsx! { div {} }
    }
}

#[component]
pub fn Lobby() -> Element {
    let toast = use_toast();
    let nav = use_navigator();

    // Selected game state
    let mut selected_game_type = use_signal(|| "lincoln".to_string());

    // Room config signals
    let mut role_config = use_signal(|| {
        HashMap::from([
            ("Judge".to_string(), "human".to_string()),
            ("Pro".to_string(), "ai".to_string()),
            ("Con".to_string(), "ai".to_string()),
        ])
    });
    let mut my_role = use_signal(|| "Judge".to_string());
    let mut max_round = use_signal(|| 16_usize);
    let mut game_config = use_signal(|| Option::<Value>::None);
    let mut is_public = use_signal(|| true); // Creator sets public status

    // State for public rooms list
    let mut public_rooms = use_signal(|| Vec::<RoomSnapshotData>::new());
    let mut loading_public = use_signal(|| true);
    let mut creating = use_signal(|| false);

    // Load public rooms
    let mut load_public_rooms = move || {
        loading_public.set(true);
        spawn(async move {
            match get_public_rooms().await {
                Ok(rooms) => {
                    public_rooms.set(rooms);
                }
                Err(e) => {
                    toast.show(
                        format!("获取公开房间失败: {e}"),
                        crate::routes::layout::ToastType::Error,
                    );
                }
            }
            loading_public.set(false);
        });
    };

    // Load public rooms on mount
    use_effect(move || {
        load_public_rooms();
    });

    // Handle game type switch - initializes defaults cleanly to avoid RefCell panics
    let mut select_game = move |gt: &str| {
        selected_game_type.set(gt.to_string());
        if let Some(def) = REGISTRY.get(gt) {
            let default_cfg = (def.default_config)();
            role_config.set(default_cfg.role_config);
            my_role.set(default_cfg.my_role);
            max_round.set(default_cfg.max_round);
            game_config.set(default_cfg.game_config);
        }
    };

    // Handle Create Room
    let handle_create_room = move |_| {
        if *creating.read() {
            return;
        }

        creating.set(true);
        let game_type = selected_game_type.read().clone();
        let max_rnd = *max_round.read();
        let my_slot = my_role.read().clone();
        let configs = role_config.read().clone();
        let g_cfg = game_config.read().clone();
        let is_pub = *is_public.read();

        // derive slot names list ordered appropriately
        let slots = if let Some(def) = REGISTRY.get(game_type.as_str()) {
            (def.generate_slots)(&configs)
        } else {
            let mut s: Vec<String> = configs.keys().cloned().collect();
            s.sort();
            s
        };

        spawn(async move {
            let req = CreateRoomRequest {
                game_type,
                max_round: max_rnd,
                my_slot,
                slots,
                slot_configs: configs,
                game_config: g_cfg,
                is_public: is_pub,
            };

            match create_room(&req).await {
                Ok(resp) => {
                    if resp.status == "success" {
                        if let (Some(rid), Some(aid)) = (resp.room_id, resp.actor_id) {
                            toast.show(
                                "房间创建成功！正在进入...".to_string(),
                                crate::routes::layout::ToastType::Success,
                            );
                            nav.push(super::Route::Game {
                                room_id: rid,
                                actor_id: aid,
                            });
                        } else {
                            toast.show(
                                "房间创建响应异常".to_string(),
                                crate::routes::layout::ToastType::Error,
                            );
                        }
                    } else {
                        toast.show(
                            resp.message.unwrap_or_else(|| "创建房间失败".to_string()),
                            crate::routes::layout::ToastType::Error,
                        );
                    }
                }
                Err(e) => {
                    toast.show(
                        format!("网络请求失败: {e}"),
                        crate::routes::layout::ToastType::Error,
                    );
                }
            }
            creating.set(false);
        });
    };

    let games = REGISTRY.all_games();

    rsx! {
        div { class: "lobby-container animate-fade-in",
            // Header Section
            div { class: "lobby-header-banner",
                h1 { "⚔️ 欢迎来到 Turn Craft" }
                p { "选择您喜爱的回合制博弈游戏，搭配个性化 AI 助手，开启精彩协作！" }
            }

            // Two-column layout: Left (Carousel + Config), Right (Active Rooms)
            div { class: "lobby-layout",
                // Left Column
                div { class: "lobby-left-col",
                    // Game Selector Wall
                    div { class: "section-card glass-panel",
                        h3 { "🎮 选择博弈类型" }
                        div { class: "game-carousel-wrapper",
                            for def in games.iter() {
                                {
                                    let gt = def.game_type;
                                    let is_active = *selected_game_type.read() == gt;
                                    rsx! {
                                        div {
                                            key: "{gt}",
                                            class: if is_active { "game-select-card active" } else { "game-select-card" },
                                            onclick: move |_| select_game(gt),
                                            div { class: "game-select-icon", "{def.icon}" }
                                            div { class: "game-select-name", "{def.name}" }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Room Settings Panel
                    div { class: "section-card glass-panel config-panel-container",
                        h3 { "⚙️ 配置对局参数" }


                        // Render dynamically based on selected game
                        if let Some(def) = REGISTRY.get(&selected_game_type.read()) {
                            {
                                let lobby_card = def.lobby_card;
                                rsx! {
                                    div { class: "game-config-form",
                                        // Standard parameters
                                        div { class: "form-field inline-field",
                                            label { "公开房间" }
                                            input {
                                                r#type: "checkbox",
                                                class: "styled-checkbox",
                                                checked: "{is_public}",
                                                onchange: move |e| {
                                                    is_public.set(e.value() == "true");
                                                }
                                            }
                                            span { class: "field-hint", "允许此房间展示在公开列表中" }
                                        }

                                        // Game-specific parameters
                                        DynamicLobbyCard {
                                            key: "{selected_game_type.read()}",
                                            game_type: selected_game_type.read().clone(),
                                            props: GameConfigProps {
                                                role_config,
                                                my_role,
                                                max_round,
                                                game_config,
                                            }
                                        }

                                        // Action Button
                                        button {
                                            class: if *creating.read() { "create-room-btn loading" } else { "create-room-btn" },
                                            onclick: handle_create_room,
                                            disabled: *creating.read(),
                                            if *creating.read() {
                                                span { class: "spinner" }
                                            } else {
                                                "🏟️ 创建房间并进入"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // Right Column: Active Public Rooms
                div { class: "lobby-right-col glass-panel",
                    div { class: "public-rooms-header",
                        h3 { "🌐 活跃公开房间" }
                        button {
                            class: "refresh-btn",
                            onclick: move |_| load_public_rooms(),
                            title: "刷新列表",
                            "🔄"
                        }
                    }

                    if *loading_public.read() {
                        div { class: "skeleton-list",
                            for _ in 0..3 {
                                div { class: "skeleton-item" }
                            }
                        }
                    } else if public_rooms.read().is_empty() {
                        div { class: "empty-state-card",
                            div { class: "empty-icon", "🍃" }
                            p { "当前没有活跃的公开房间，你可以自己创建一个！" }
                        }
                    } else {
                        div { class: "public-rooms-list",
                            for room in public_rooms.read().iter() {
                                {
                                    let rid = room.room_id.clone();
                                    let game_def = REGISTRY.get(&room.game_type);
                                    let game_name = game_def.map(|g| g.name).unwrap_or("未知游戏");
                                    let game_icon = game_def.map(|g| g.icon).unwrap_or("❓");
                                    let rounds = room.max_round;
                                    let time_str = room.created_at.chars().take(16).collect::<String>().replace("T", " ");

                                    // Detect occupied status
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
                                        div { key: "{rid}", class: "public-room-card glass-panel-subtle",
                                            div { class: "room-card-top",
                                                span { class: "room-card-game", "{game_icon} {game_name}" }
                                                span { class: "room-card-slots-badge", "空位: {empty_slots}" }
                                            }
                                            div { class: "room-card-mid",
                                                div { class: "room-id-label", "ID: {rid}" }
                                                div { class: "room-meta-label", "局数上限: {rounds} 轮" }
                                                div { class: "room-time-label", "创建时间: {time_str}" }
                                            }
                                            div { class: "room-card-bot",
                                                // If slots are available, we can join as player, otherwise join as spectator
                                                if empty_slots > 0 {
                                                    button {
                                                        class: "join-btn player",
                                                        onclick: move |_| {
                                                            nav.push(super::Route::Game { room_id: rid.clone(), actor_id: first_empty_slot.clone() });
                                                        },
                                                        "加入对局"
                                                    }
                                                } else {
                                                    button {
                                                        class: "join-btn spectator",
                                                        onclick: move |_| {
                                                            nav.push(super::Route::Game { room_id: rid.clone(), actor_id: "spectator".to_string() });
                                                        },
                                                        "观战模式"
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
