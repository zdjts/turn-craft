use std::collections::HashMap;

use dioxus::prelude::*;
use tracing::{error, info, warn};

use crate::api::{create_room, CreateRoomRequest};
use crate::routes::lobby_actions::{
    select_lincoln_game, select_lobby_role, select_texas_game, set_player_count,
    set_spectator_mode,
};

/// 林肯辩论可用角色列表
const LINCOLN_ROLES: &[(&str, &str)] = &[
    ("Judge", "裁判 — 开题与总结"),
    ("Pro", "正方 — 立论"),
    ("Con", "反方 — 驳论"),
];

/// 德州扑克可用角色列表
const TEXAS_HOLDEM_ROLES: &[(&str, &str)] = &[
    ("player1", "玩家 1"),
    ("player2", "玩家 2"),
    ("player3", "玩家 3"),
    ("player4", "玩家 4"),
    ("player5", "玩家 5"),
    ("player6", "玩家 6"),
];

/// 可选人数配置
const PLAYER_COUNT_OPTIONS: &[usize] = &[2, 3, 4, 5, 6];

/// 大厅页面组件：创建房间入口
#[component]
pub fn Lobby() -> Element {
    let navigator = use_navigator();
    let mut player_id = use_signal(|| "judge_zeng".to_string());
    let mut max_round = use_signal(|| "16".to_string());
    let selected_role = use_signal(|| "Judge".to_string());
    let role_modes = use_signal(|| {
        HashMap::from([
            ("Judge".to_string(), "human".to_string()),
            ("Pro".to_string(), "ai".to_string()),
            ("Con".to_string(), "ai".to_string()),
        ])
    });
    let mut loading = use_signal(|| false);
    let mut error_msg = use_signal(|| Option::<String>::None);
    let selected_game = use_signal(|| "lincoln".to_string());

    // 德州扑克特定配置
    let mut small_blind = use_signal(|| "10".to_string());
    let mut big_blind = use_signal(|| "20".to_string());
    let mut starting_chips = use_signal(|| "1000".to_string());
    let player_count = use_signal(|| 6_usize);
    let spectator_mode = use_signal(|| false);

    let on_create = move |_| {
        let pid = player_id.read().trim().to_string();
        let rounds = max_round.read().trim().parse::<usize>().unwrap_or(16);
        let game = selected_game.read().clone();
        let is_spectator = *spectator_mode.read();
        let count = *player_count.read();

        if pid.is_empty() {
            warn!(target: "lobby", "用户尝试创建房间但未输入玩家 ID");
            error_msg.set(Some("请输入玩家 ID".to_string()));
            return;
        }

        // 根据观战模式和人数生成角色配置
        let (my_role, rc) = if game == "texas_holdem" {
            let mut modes = HashMap::new();
            let role = if *spectator_mode.read() {
                // 观战模式：所有玩家都是AI
                for i in 1..=*player_count.read() {
                    modes.insert(format!("player{}", i), "ai".to_string());
                }
                "spectator".to_string()
            } else {
                // 正常模式：第一个玩家是人类，其他是AI
                modes.insert("player1".to_string(), "human".to_string());
                for i in 2..=*player_count.read() {
                    modes.insert(format!("player{}", i), "ai".to_string());
                }
                "player1".to_string()
            };
            (role, modes)
        } else {
            // 林肯辩论保持原有逻辑
            (selected_role.read().clone(), role_modes.read().clone())
        };

        info!(target: "lobby", player_id = %pid, max_round = rounds, my_role = %my_role, role_config = ?rc, game_type = %game, spectator = is_spectator, player_count = count, "用户点击创建房间");
        loading.set(true);
        error_msg.set(None);

        spawn(async move {
            let req = CreateRoomRequest {
                game_type: game.clone(),
                max_round: rounds,
                my_role,
                role_config: rc,
                game_config: if game == "texas_holdem" {
                    Some(serde_json::json!({
                        "small_blind": small_blind.read().parse::<u32>().unwrap_or(10),
                        "big_blind": big_blind.read().parse::<u32>().unwrap_or(20),
                        "starting_chips": starting_chips.read().parse::<u32>().unwrap_or(1000),
                    }))
                } else {
                    None
                },
            };

            match create_room(&req).await {
                Ok(resp) if resp.status == "success" => {
                    if let (Some(rid), Some(aid)) = (resp.room_id, resp.actor_id) {
                        info!(target: "lobby", room_id = %rid, actor_id = %aid, "房间创建成功，正在跳转...");
                        navigator.push(format!("/game/{rid}/{aid}"));
                    } else {
                        error!(target: "lobby", "服务器返回 success 但缺 room_id 或 actor_id");
                        error_msg.set(Some("服务器响应不完整".to_string()));
                    }
                }
                Ok(resp) => {
                    warn!(target: "lobby", status = %resp.status, message = ?resp.message, "创建房间失败");
                    error_msg.set(Some(resp.message.unwrap_or("未知错误".to_string())));
                }
                Err(e) => {
                    error!(target: "lobby", error = %e, "创建房间请求异常");
                    error_msg.set(Some(e));
                }
            }
            loading.set(false);
        });
    };

    rsx! {
        div { class: "lobby-shell",
            div { class: "lobby-card",
                div { class: "lobby-brand",
                    h1 { "🏛️ 辩论竞技场" }
                    p { "创建房间，开启游戏" }
                }

                div { class: "lobby-form",
                    div { class: "form-field",
                        label { "玩家 ID" }
                        input {
                            placeholder: "judge_zeng",
                            value: "{player_id}",
                            oninput: move |e| player_id.set(e.value()),
                        }
                    }

                    // ── 游戏类型选择 ──
                    div { class: "form-field",
                        label { "游戏类型" }
                        div { class: "game-type-grid",
                            div {
                                class: if *selected_game.read() == "lincoln" { "game-type-card selected" } else { "game-type-card" },
                                onclick: move |_| select_lincoln_game(selected_game, selected_role, role_modes),
                                // div { class: "game-type-icon", "🏛️" }
                                div { class: "game-type-name", "林肯辩论" }
                                // div { class: "game-type-desc", "经典辩论模式" }
                            }
                            div {
                                class: if *selected_game.read() == "texas_holdem" { "game-type-card selected" } else { "game-type-card" },
                                onclick: move |_| select_texas_game(selected_game, selected_role, role_modes, player_count),
                                // div { class: "game-type-icon", "🃏" }
                                div { class: "game-type-name", "德州扑克" }
                                // div { class: "game-type-desc", "经典扑克游戏" }
                            }
                        }
                    }

                    // ── 角色选择 ──
                    div { class: "form-field",
                        label { "你的角色" }
                        div { class: "role-grid",
                            if *selected_game.read() == "lincoln" {
                                for (role_name, role_desc) in LINCOLN_ROLES.iter() {
                                    {
                                        let rn = role_name.to_string();
                                        let is_selected = *selected_role.read() == rn;
                                        let mode = role_modes.read().get(&rn).cloned().unwrap_or("ai".to_string());
                                        let is_human = mode == "human";
                                        rsx! {
                                            div {
                                                class: if is_selected {
                                                    "role-card selected"
                                                } else {
                                                    "role-card"
                                                },
                                                onclick: {
                                                    let rn = rn.clone();
                                                    move |_| select_lobby_role(selected_role, role_modes, rn.clone(), LINCOLN_ROLES)
                                                },
                                                div { class: "role-card-header",
                                                    span { class: "role-card-name", "{role_name}" }
                                                    span {
                                                        class: if is_human { "role-badge human" } else { "role-badge ai" },
                                                        if is_human { "真人" } else { "AI" }
                                                    }
                                                }
                                                div { class: "role-card-desc", "{role_desc}" }
                                            }
                                        }
                                    }
                                }
                            } else if *selected_game.read() == "texas_holdem" {
                                for i in 0..*player_count.read() {
                                // for (role_name, role_desc) in TEXAS_HOLDEM_ROLES.iter() {
                                    {
                                        let rn = TEXAS_HOLDEM_ROLES[i].0.to_string();
                                        let role_desc = TEXAS_HOLDEM_ROLES[i].1.to_string();
                                        let is_selected = *selected_role.read() == rn;
                                        let mode = role_modes.read().get(&rn).cloned().unwrap_or("ai".to_string());
                                        let is_human = mode == "human";
                                        rsx! {
                                            div {
                                                class: if is_selected {
                                                    "role-card selected"
                                                } else {
                                                    "role-card"
                                                },
                                                onclick: {
                                                    let rn = rn.clone();
                                                    move |_| select_lobby_role(selected_role, role_modes, rn.clone(), TEXAS_HOLDEM_ROLES)
                                                },
                                                div { class: "role-card-header",
                                                    span { class: "role-card-name", "{rn}" }
                                                    span {
                                                        class: if is_human { "role-badge human" } else { "role-badge ai" },
                                                        if is_human { "真人" } else { "AI" }
                                                    }
                                                }
                                                div { class: "role-card-desc", "{role_desc}" }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // ── 德州扑克配置 ──
                    if *selected_game.read() == "texas_holdem" {
                        div { class: "form-field",
                            label { "游戏人数" }
                            div { class: "player-count-grid",
                                for count_opt in PLAYER_COUNT_OPTIONS.iter() {
                                    {
                                        let c = *count_opt;
                                        let is_selected = *player_count.read() == c;
                                        rsx! {
                                            button {
                                                class: if is_selected { "count-btn selected" } else { "count-btn" },
                                                onclick: move |_| set_player_count(player_count, role_modes, c),
                                                "{c} 人"
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        div { class: "form-field",
                            label { "游戏模式" }
                            div { class: "mode-toggle",
                                button {
                                    class: if !*spectator_mode.read() { "mode-btn selected" } else { "mode-btn" },
                                    onclick: move |_| set_spectator_mode(spectator_mode, false),
                                    div { class: "mode-icon", "🎮" }
                                    div { class: "mode-label", "亲自上阵" }
                                    div { class: "mode-desc", "你作为玩家参与对局" }
                                }
                                button {
                                    class: if *spectator_mode.read() { "mode-btn selected" } else { "mode-btn" },
                                    onclick: move |_| set_spectator_mode(spectator_mode, true),
                                    div { class: "mode-icon", "👀" }
                                    div { class: "mode-label", "观战模式" }
                                    div { class: "mode-desc", "观看 AI 之间的对局" }
                                }
                            }
                        }

                        div { class: "form-field",
                            label { "德州扑克配置" }
                            div { class: "texas-config",
                                div { class: "config-field",
                                    label { "小盲注" }
                                    input {
                                        r#type: "number",
                                        value: "{small_blind}",
                                        oninput: move |e| small_blind.set(e.value()),
                                    }
                                }
                                div { class: "config-field",
                                    label { "大盲注" }
                                    input {
                                        r#type: "number",
                                        value: "{big_blind}",
                                        oninput: move |e| big_blind.set(e.value()),
                                    }
                                }
                            }

                                div { class: "config-field",
                                    label { "起始筹码" }
                                    input {
                                        r#type: "number",
                                        value: "{starting_chips}",
                                        oninput: move |e| starting_chips.set(e.value()),
                                    }
                                }
                        }
                    }

                    if *selected_game.read() == "lincoln" {
                        div { class: "form-field",
                            label { "最大轮次" }
                            input {
                                placeholder: "16",
                                value: "{max_round}",
                                oninput: move |e| max_round.set(e.value()),
                            }
                        }
                    }

                    if let Some(ref msg) = *error_msg.read() {
                        div { class: "form-error",
                            "⚠️ {msg}"
                        }
                    }

                    button {
                        class: "lobby-submit",
                        disabled: *loading.read(),
                        onclick: on_create,
                        if *loading.read() {
                            "⏳ 创建中..."
                        } else {
                            "🚀 创建房间"
                        }
                    }
                }

                p { class: "lobby-footer",
                    "创建后将自动跳转到对局界面"
                }
            }
        }
    }
}
