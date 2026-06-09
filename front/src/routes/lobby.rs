use std::collections::HashMap;

use dioxus::prelude::*;
use tracing::{error, info, warn};

use crate::api::{CreateRoomRequest, create_room};

const LINCOLN_ROLES: &[(&str, &str)] = &[
    ("Judge", "裁判 — 开题与总结"),
    ("Pro", "正方 — 立论"),
    ("Con", "反方 — 驳论"),
];

#[component]
pub fn Lobby() -> Element {
    let navigator = use_navigator();
    let mut player_id = use_signal(|| "judge_zeng".to_string());
    let mut max_round = use_signal(|| "16".to_string());
    let mut selected_role = use_signal(|| "Judge".to_string());
    let mut role_modes = use_signal(|| {
        HashMap::from([
            ("Judge".to_string(), "human".to_string()),
            ("Pro".to_string(), "ai".to_string()),
            ("Con".to_string(), "ai".to_string()),
        ])
    });
    let mut loading = use_signal(|| false);
    let mut error_msg = use_signal(|| Option::<String>::None);

    let on_create = move |_| {
        let pid = player_id.read().trim().to_string();
        let rounds = max_round
            .read()
            .trim()
            .parse::<usize>()
            .unwrap_or(16);
        let my_role = selected_role.read().clone();
        let rc = role_modes.read().clone();

        if pid.is_empty() {
            warn!(target: "lobby", "用户尝试创建房间但未输入玩家 ID");
            error_msg.set(Some("请输入玩家 ID".to_string()));
            return;
        }

        info!(target: "lobby", player_id = %pid, max_round = rounds, my_role = %my_role, role_config = ?rc, "用户点击创建房间");
        loading.set(true);
        error_msg.set(None);

        spawn(async move {
            let req = CreateRoomRequest {
                game_type: "lincoln".to_string(),
                max_round: rounds,
                my_role,
                role_config: rc,
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
                    p { "创建房间，开启林肯 — 道格拉斯辩论" }
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

                    div { class: "form-field",
                        label { "最大轮次" }
                        input {
                            placeholder: "16",
                            value: "{max_round}",
                            oninput: move |e| max_round.set(e.value()),
                        }
                    }

                    // ── 角色选择 ──
                    div { class: "form-field",
                        label { "你的角色" }
                        div { class: "role-grid",
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
                                                move |_| {
                                                    selected_role.set(rn.clone());
                                                    // 自动将选中角色设为 human，其他设为 ai
                                                    let mut modes = HashMap::new();
                                                    for (name, _) in LINCOLN_ROLES.iter() {
                                                        let n = name.to_string();
                                                        if n == rn {
                                                            modes.insert(n, "human".to_string());
                                                        } else {
                                                            modes.insert(n, "ai".to_string());
                                                        }
                                                    }
                                                    role_modes.set(modes);
                                                }
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
