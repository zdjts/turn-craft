use crate::api::{create_room, get_public_rooms, join_room, CreateRoomRequest, RoomSnapshotData};
use crate::games::registry::{GameConfigProps, REGISTRY};
use crate::routes::layout::use_toast;
use dioxus::prelude::*;
use serde_json::Value;
use std::collections::HashMap;

#[derive(Clone, PartialEq)]
enum LobbyMode {
    Browse,
    Config { game_type: String },
}

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
fn PublicRoomList(
    public_rooms: Signal<Vec<RoomSnapshotData>>,
    loading_public: Signal<bool>,
    room_filter: Option<String>,
    load_rooms: Callback<()>,
) -> Element {
    let toast = use_toast();
    let nav = use_navigator();

    let all_rooms = public_rooms.read();
    let rooms: Vec<&RoomSnapshotData> = match &room_filter {
        Some(ref gt) => all_rooms.iter().filter(|r| &r.game_type == gt).collect(),
        None => all_rooms.iter().collect(),
    };

    rsx! {
        div { class: "pg-lobby-right g-card",
            div { class: "pg-lobby-rooms-header",
                h3 { "🌐 活跃公开房间" }
                button {
                    class: "pg-lobby-refresh",
                    onclick: move |_| load_rooms.call(()),
                    title: "刷新列表",
                    "🔄"
                }
            }

            if loading_public() {
                div { class: "g-skeleton-list",
                    for _ in 0..3 {
                        div { class: "g-skeleton-row" }
                    }
                }
            } else if rooms.is_empty() {
                div { class: "g-empty",
                    div { class: "empty-icon", "🍃" }
                    p { "当前没有活跃的公开房间，你可以自己创建一个！" }
                }
            } else {
                div { class: "pg-lobby-rooms",
                    for room in rooms.iter() {
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
                                div { key: "{rid}", class: "pg-lobby-room-card g-card-subtle",
                                    div { class: "pg-lobby-room-top",
                                        span { class: "pg-lobby-room-game", "{game_icon} {game_name}" }
                                        span { class: "pg-lobby-room-slots", "空位: {empty_slots}" }
                                    }
                                    div { class: "pg-lobby-room-mid",
                                        div { class: "pg-lobby-room-id", "ID: {rid}" }
                                        div { class: "pg-lobby-room-meta", "局数上限: {rounds} 轮" }
                                        div { class: "pg-lobby-room-time", "创建时间: {time_str}" }
                                    }
                                    div { class: "pg-lobby-room-bot",
                                        if empty_slots > 0 {
                                            button {
                                                class: "pg-lobby-join player",
                                                onclick: move |_| {
                                                    let rid = rid.clone();
                                                    let aid = first_empty_slot.clone();
                                                    let nav = nav.clone();
                                                    let toast = toast.clone();
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
                                                "加入对局"
                                            }
                                        } else {
                                            button {
                                                class: "pg-lobby-join spectator",
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

// ═══════════════════════════════════════════════════════
//  View A — Game Browse (grid + quick-start)
// ═══════════════════════════════════════════════════════

#[component]
fn GameBrowseView(
    selected_game: Signal<Option<String>>,
    on_select: Callback<String>,
    on_quick_start: Callback<String>,
    on_enter_config: Callback<String>,
) -> Element {
    let games = REGISTRY.all_games();

    rsx! {
        div { class: "pg-lobby-games",
            for def in games.iter() {
                {
                    let gt = def.game_type;
                    let is_selected = selected_game.read().as_deref() == Some(gt);
                    rsx! {
                        div {
                            key: "{gt}",
                            class: if is_selected { "pg-lobby-game-card is-selected" } else { "pg-lobby-game-card" },
                            onclick: {
                                let gt = gt.to_string();
                                let mut on_select = on_select.clone();
                                move |_| on_select.call(gt.clone())
                            },
                            div { class: "pg-lobby-game-icon", "{def.icon}" }
                            div { class: "pg-lobby-game-name", "{def.name}" }
                            div { class: "pg-lobby-game-card-desc", "{def.description}" }
                            div { class: "pg-lobby-game-card-meta",
                                "{def.min_players}-{def.max_players} 人"
                            }
                            div { class: "pg-lobby-game-card-actions",
                                div { class: "pg-lobby-game-card-actions-inner",
                                    button {
                                        class: "pg-lobby-quick-start",
                                        onclick: {
                                            let gt = gt.to_string();
                                            let mut cb = on_quick_start.clone();
                                            move |e| { e.stop_propagation(); cb.call(gt.clone()); }
                                        },
                                        "⚡ 快速开始"
                                    }
                                    button {
                                        class: "pg-lobby-config-toggle",
                                        onclick: {
                                            let gt = gt.to_string();
                                            let mut cb = on_enter_config.clone();
                                            move |e| { e.stop_propagation(); cb.call(gt.clone()); }
                                        },
                                        "⚙ 自定义"
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

// ═══════════════════════════════════════════════════════
//  View B — Single game configuration
// ═══════════════════════════════════════════════════════

#[component]
fn GameConfigView(
    game_type: String,
    on_back: Callback<()>,
    role_config: Signal<HashMap<String, String>>,
    my_role: Signal<String>,
    max_round: Signal<usize>,
    game_config: Signal<Option<Value>>,
    is_public: Signal<bool>,
    creating: ReadOnlySignal<bool>,
    onCreate: Callback<()>,
) -> Element {
    let game_name = REGISTRY
        .get(&game_type)
        .map(|d| d.name)
        .unwrap_or("未知游戏");

    rsx! {
        div { class: "pg-lobby-config-panel g-card",
            button {
                class: "pg-lobby-config-back",
                onclick: move |_| on_back.call(()),
                "← 返回游戏列表"
            }

            h3 { "⚙️ 配置: {game_name}" }

            div { class: "pg-lobby-config",
                div { class: "g-field pg-lobby-inline",
                    label { "公开房间" }
                    input {
                        r#type: "checkbox",
                        class: "g-toggle",
                        checked: "{is_public}",
                        onchange: move |e| {
                            is_public.set(e.value() == "true");
                        }
                    }
                    span { class: "pg-lobby-hint", "允许此房间展示在公开列表中" }
                }

                DynamicLobbyCard {
                    key: "{game_type}",
                    game_type: game_type.clone(),
                    props: GameConfigProps {
                        role_config,
                        my_role,
                        max_round,
                        game_config,
                    }
                }

                button {
                    class: if *creating.read() { "pg-lobby-create is-loading" } else { "pg-lobby-create" },
                    onclick: move |_| onCreate.call(()),
                    disabled: *creating.read(),
                    if *creating.read() {
                        span { class: "g-spinner" }
                    } else {
                        "🏟️ 创建房间并进入"
                    }
                }
            }
        }
    }
}

// ═══════════════════════════════════════════════════════
//  Lobby — root component, dispatches mode
// ═══════════════════════════════════════════════════════

#[component]
pub fn Lobby() -> Element {
    let toast = use_toast();
    let nav = use_navigator();

    let mut mode = use_signal(|| LobbyMode::Browse);
    let mut selected_game = use_signal(|| Option::<String>::None);

    // Game type tracking (used internally for quick_start / config init)
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
    let mut is_public = use_signal(|| true);
    let mut creating = use_signal(|| false);

    // Public rooms
    let mut public_rooms = use_signal(|| Vec::<RoomSnapshotData>::new());
    let mut loading_public = use_signal(|| true);

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

    use_effect(move || {
        load_public_rooms();
    });

    // ── select_game: initialize config signals for a game type ──
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

    // ── handle_create_room: existing logic (unchanged) ──
    let mut handle_create_room = move |_| {
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

    // ── quick_start: default config → create room directly ──
    let mut quick_create = move |gt: String| {
        select_game(&gt);
        handle_create_room(());
    };

    // ── enter_config: switch to Config view ──
    let mut enter_config = move |gt: String| {
        select_game(&gt);
        selected_game.set(Some(gt.clone()));
        mode.set(LobbyMode::Config { game_type: gt });
    };

    // ── Back to Browse ──
    let back_to_browse = move |_: ()| {
        mode.set(LobbyMode::Browse);
    };

    let mut room_filter_signal = use_signal(|| Option::<String>::None);

    // Keep room_filter synced with current config game_type
    let mut sync_filter = move |gt: Option<String>| {
        room_filter_signal.set(gt);
    };
    let toggle_select = move |gt: String| {
        let mut sel = selected_game;
        let cur = sel.read().clone();
        if cur.as_deref() == Some(&gt) {
            sel.set(None);
        } else {
            sel.set(Some(gt));
        }
    };

    let config_game_type = match &*mode.read() {
        LobbyMode::Browse => selected_game.read().clone(),
        LobbyMode::Config { game_type } => Some(game_type.clone()),
    };

    rsx! {
        div { class: "pg-lobby animate-fade-in",
            div { class: "pg-lobby-banner",
                h1 { "⚔️ 欢迎来到 Turn Craft" }
                p { "选择您喜爱的回合制博弈游戏，搭配个性化 AI 助手，开启精彩协作！" }
            }

            div { class: "pg-lobby-layout",
                // Left Column — mode-dependent
                div { class: "pg-lobby-left",
                    match &*mode.read() {
                        LobbyMode::Browse => rsx! {
                            GameBrowseView {
                                selected_game,
                                on_select: Callback::new(move |gt: String| toggle_select(gt)),
                                on_quick_start: Callback::new(move |gt: String| quick_create(gt)),
                                on_enter_config: Callback::new(move |gt: String| enter_config(gt)),
                            }
                        },
                        LobbyMode::Config { game_type } => rsx! {
                            GameConfigView {
                                game_type: game_type.clone(),
                                on_back: Callback::new(back_to_browse),
                                role_config,
                                my_role,
                                max_round,
                                game_config,
                                is_public,
                                creating,
                                onCreate: Callback::new(handle_create_room),
                            }
                        },
                    }
                }

                // Right Column — always visible public room list
                PublicRoomList {
                    public_rooms,
                    loading_public,
                    room_filter: config_game_type,
                    load_rooms: Callback::new(move |_: ()| load_public_rooms()),
                }
            }
        }
    }
}
