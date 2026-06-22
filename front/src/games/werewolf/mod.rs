use dioxus::prelude::*;
use serde_json::Value;
use std::collections::HashMap;

use super::GamePluginProps;
use crate::games::registry::GameConfigProps;

#[component]
pub fn WerewolfLobbyCard(props: GameConfigProps) -> Element {
    let mut role_config = props.role_config;
    let mut my_role = props.my_role;

    let mut spectator_mode = use_signal(|| false);

    use_effect(move || {
        if my_role.read().is_empty() {
            my_role.set("Player1".to_string());
            let mut defaults = std::collections::HashMap::new();
            defaults.insert("Player1".to_string(), "human".to_string());
            for i in 2..=7 {
                defaults.insert(format!("Player{}", i), "ai".to_string());
            }
            role_config.set(defaults);
        }
    });

    rsx! {
        div { class: "form-field",
            label { "游戏模式" }
            div { class: "mode-toggle",
                button {
                    class: if !*spectator_mode.read() { "mode-btn selected" } else { "mode-btn" },
                    onclick: move |_| {
                        spectator_mode.set(false);
                        let mut modes = std::collections::HashMap::new();
                        modes.insert("Player1".to_string(), "human".to_string());
                        for i in 2..=7 {
                            modes.insert(format!("Player{}", i), "ai".to_string());
                        }
                        my_role.set("Player1".to_string());
                        role_config.set(modes);
                    },
                    div { class: "mode-icon", "🎮" }
                    div { class: "mode-label", "亲自上阵" }
                    div { class: "mode-desc", "您将作为 Player1 参与游戏，其余为 AI" }
                }
                button {
                    class: if *spectator_mode.read() { "mode-btn selected" } else { "mode-btn" },
                    onclick: move |_| {
                        spectator_mode.set(true);
                        let mut modes = std::collections::HashMap::new();
                        for i in 1..=7 {
                            modes.insert(format!("Player{}", i), "ai".to_string());
                        }
                        my_role.set("spectator".to_string());
                        role_config.set(modes);
                    },
                    div { class: "mode-icon", "👀" }
                    div { class: "mode-label", "观战模式" }
                    div { class: "mode-desc", "观看 7 个 AI 之间的对局" }
                }
            }
        }
    }
}

pub fn parse_phase_name(phase: Option<&Value>) -> String {
    if let Some(p) = phase {
        if let Some(s) = p.as_str() {
            s.to_string()
        } else if let Some(obj) = p.as_object() {
            obj.keys()
                .next()
                .cloned()
                .unwrap_or_else(|| "Init".to_string())
        } else {
            "Init".to_string()
        }
    } else {
        "Init".to_string()
    }
}

pub fn get_player_role(players: Option<&Value>, my_id: &str) -> String {
    players
        .and_then(|p| p.as_array())
        .and_then(|arr| {
            arr.iter()
                .find(|p| p.get("id").and_then(|id| id.as_str()) == Some(my_id))
        })
        .and_then(|p| p.get("role"))
        .and_then(|r| r.as_str())
        .unwrap_or("未知")
        .to_string()
}

pub fn get_player_alive(players: Option<&Value>, my_id: &str) -> bool {
    players
        .and_then(|p| p.as_array())
        .and_then(|arr| {
            arr.iter()
                .find(|p| p.get("id").and_then(|id| id.as_str()) == Some(my_id))
        })
        .and_then(|p| p.get("is_alive"))
        .and_then(|a| a.as_bool())
        .unwrap_or(false)
}

#[component]
pub fn WerewolfGame(props: GamePluginProps) -> Element {
    let state = props.state.read().clone();
    let is_finished = state.get("phase").and_then(|p| p.get("GameOver")).is_some();
    let my_id = props.actor_id.clone();

    let my_role = get_player_role(state.get("players"), &my_id);
    let my_alive = get_player_alive(state.get("players"), &my_id);
    let phase_name = parse_phase_name(state.get("phase"));

    let day = state.get("day").and_then(|v| v.as_u64()).unwrap_or(1);
    let is_my_turn = state.get("active_actor").and_then(|v| v.as_str()) == Some(&my_id);

    let mut show_ai_content = use_signal(|| true);

    rsx! {
        div { class: "lincoln-shell",
            // ── 时间轴区域 ──
            div { class: "timeline-scroll",
                // 顶部信息栏
                div { class: "timeline-header",
                    div { class: "timeline-title",
                        "🐺 狼人杀 — 7人标准局"
                    }
                    div { class: "timeline-round",
                        "第 {day} 天 — {phase_name}"
                    }
                    button {
                        class: "glass-panel-subtle toggle-ai-btn",
                        style: "margin-left: auto; font-size: 0.85em; padding: 4px 12px; cursor: pointer;",
                        onclick: move |_| {
                            let cur = *show_ai_content.read();
                            show_ai_content.set(!cur);
                        },
                        if *show_ai_content.read() { "👀 隐藏 AI 思考" } else { "🙈 显示 AI 思考" }
                    }
                }

                if is_finished {
                    div { class: "text-red-400 text-center font-bold text-xl my-4", "🏆 游戏结束！" }
                } else {
                    div { class: "text-center text-sm mb-4", style: "color: var(--accent);",
                        {
                            let alive_text = if my_alive { "存活" } else { "已阵亡" };
                            format!("你的身份: {}  |  状态: {}", my_role, alive_text)
                        }
                    }
                }

                // 历史流水
                if let Some(history) = state.get("history").and_then(|h| h.as_array()) {
                    for (idx, evt) in history.iter().enumerate() {
                        if let Some(content) = evt.get("content").and_then(|v| v.as_str()) {
                            if !content.is_empty() {
                                {
                                    let actor = evt.get("actor_id").and_then(|v| v.as_str()).unwrap_or("System");
                                    let is_sys = actor == "System";
                                    let act_type = evt.get("action_type").and_then(|v| v.as_str()).unwrap_or("");
                                    let d = evt.get("day").and_then(|v| v.as_u64()).unwrap_or(0);

                                    rsx! {
                                        div { class: "bubble-row", key: "{idx}",
                                            div { class: if is_sys { "bubble-avatar" } else { "bubble-avatar pro" },
                                                if is_sys { "⚖️" } else { "👤" }
                                            }
                                            div { class: "bubble-body",
                                                div { class: "bubble-meta",
                                                    span { class: "bubble-name", if is_sys { "系统播报" } else { "{actor}" } }
                                                    span { class: "bubble-tag", "Day {d}" }
                                                }
                                                div { class: "bubble-content",
                                                    "{content}"
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

            // ── 操作面板 ──
            div { class: if (is_my_turn && !is_finished) || phase_name == "Init" || (phase_name == "DayVote" && my_alive) { "action-console" } else { "action-console locked" },
                if phase_name == "Init" {
                    div { class: "console-row", style: "justify-content: center;",
                        button {
                            class: "console-submit",
                            style: "background: var(--accent); color: white; padding: 12px 24px; font-size: 1.1em;",
                            onclick: move |_| {
                                props.on_action.call(serde_json::json!({
                                    "action_type": "start",
                                }));
                            },
                            "🎮 开始游戏"
                        }
                    }
                } else if (is_my_turn || (phase_name == "DayVote" && my_alive)) && !is_finished {
                    WerewolfActionPanel {
                        phase_name: phase_name.clone(),
                        my_role: my_role.clone(),
                        state: state.clone(),
                        on_action: move |act: Value| {
                            props.on_action.call(act);
                        }
                    }
                } else if !is_finished && my_alive && my_role == "Werewolf" && (phase_name == "DaySpeech" || phase_name == "DayVote") {
                    div { class: "console-row", style: "justify-content: flex-end;",
                        button {
                            class: "console-submit",
                            style: "background: var(--red); color: white;",
                            onclick: move |_| {
                                props.on_action.call(serde_json::json!({
                                    "action_type": "explode",
                                }));
                            },
                            "🔥 自爆"
                        }
                    }
                } else if !is_finished {
                    div { class: "console-hint", "等待其他玩家行动..." }
                } else {
                    div { class: "console-hint", "游戏已结束" }
                }
            }
        }
    }
}

#[component]
pub fn WerewolfActionPanel(
    phase_name: String,
    my_role: String,
    state: Value,
    on_action: EventHandler<Value>,
) -> Element {
    let mut text_input = use_signal(|| "".to_string());

    let alive_players: Vec<String> = state
        .get("players")
        .and_then(|p| p.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|p| {
                    if p.get("is_alive").and_then(|v| v.as_bool()).unwrap_or(false) {
                        p.get("id")
                            .and_then(|id| id.as_str())
                            .map(|s| s.to_string())
                    } else {
                        None
                    }
                })
                .collect()
        })
        .unwrap_or_default();

    rsx! {
        div { class: "console-row", style: "flex-wrap: wrap; gap: 10px;",
            if phase_name == "NightWolf" && my_role == "Werewolf" {
                span { class: "console-hint", "🐺 选择击杀目标: " }
                for target in alive_players {
                    button {
                        class: "console-submit",
                        style: "background: var(--red);",
                        onclick: move |_| {
                            on_action.call(serde_json::json!({ "action_type": "kill", "target": target }));
                        },
                        "杀 {target}"
                    }
                }
            } else if phase_name == "NightSeer" && my_role == "Seer" {
                span { class: "console-hint", "👁️ 选择查验目标: " }
                for target in alive_players {
                    button {
                        class: "console-submit",
                        style: "background: #8b5cf6;",
                        onclick: move |_| {
                            on_action.call(serde_json::json!({ "action_type": "check", "target": target }));
                        },
                        "查 {target}"
                    }
                }
            } else if phase_name == "NightWitch" && my_role == "Witch" {
                span { class: "console-hint", "🧪 女巫行动: " }
                if state.get("witch_has_save").and_then(|v| v.as_bool()).unwrap_or(false) {
                    button {
                        class: "console-submit",
                        style: "background: #10b981;",
                        onclick: move |_| {
                            on_action.call(serde_json::json!({ "action_type": "save" }));
                        },
                        "使用解药"
                    }
                }
                if state.get("witch_has_poison").and_then(|v| v.as_bool()).unwrap_or(false) {
                    for target in alive_players {
                        button {
                            class: "console-submit",
                            style: "background: var(--red);",
                            onclick: move |_| {
                                on_action.call(serde_json::json!({ "action_type": "poison", "target": target }));
                            },
                            "毒 {target}"
                        }
                    }
                }
                button {
                    class: "console-submit",
                    style: "background: #6b7280;",
                    onclick: move |_| {
                        on_action.call(serde_json::json!({ "action_type": "skip" }));
                    },
                    "跳过"
                }
            } else if phase_name == "DaySpeech" {
                textarea {
                    class: "console-textarea",
                    placeholder: "请输入你的发言...",
                    value: "{text_input}",
                    oninput: move |e| text_input.set(e.value()),
                    onkeydown: move |e: Event<KeyboardData>| {
                        if e.key() == Key::Enter && e.modifiers().ctrl() {
                            let content = text_input.read().trim().to_string();
                            if !content.is_empty() {
                                on_action.call(serde_json::json!({ "action_type": "speak", "content": content }));
                                text_input.write().clear();
                            }
                        }
                    },
                }
                button {
                    class: "console-submit",
                    onclick: move |_| {
                        let content = text_input.read().trim().to_string();
                        if !content.is_empty() {
                            on_action.call(serde_json::json!({ "action_type": "speak", "content": content }));
                            text_input.write().clear();
                        }
                    },
                    "发送发言"
                }
            } else if phase_name == "DayVote" {
                span { class: "console-hint", "🗳️ 请投票: " }
                for target in alive_players {
                    button {
                        class: "console-submit",
                        onclick: move |_| {
                            on_action.call(serde_json::json!({ "action_type": "vote", "target": target }));
                        },
                        "投 {target}"
                    }
                }
                button {
                    class: "console-submit",
                    style: "background: #6b7280;",
                    onclick: move |_| {
                        on_action.call(serde_json::json!({ "action_type": "skip" }));
                    },
                    "弃票"
                }
            } else if let Some(obj) = state.get("phase").and_then(|p| p.as_object()) {
                if obj.contains_key("DayHunterShoot") && my_role == "Hunter" {
                    span { class: "console-hint", "🔫 开枪带人: " }
                    for target in alive_players {
                        button {
                            class: "console-submit",
                            style: "background: var(--red);",
                            onclick: move |_| {
                                on_action.call(serde_json::json!({ "action_type": "shoot", "target": target }));
                            },
                            "带走 {target}"
                        }
                    }
                    button {
                        class: "console-submit",
                        style: "background: #6b7280;",
                        onclick: move |_| {
                            on_action.call(serde_json::json!({ "action_type": "skip" }));
                        },
                        "不开枪"
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_parse_phase_name() {
        assert_eq!(parse_phase_name(None), "Init");

        let p1 = json!("DaySpeech");
        assert_eq!(parse_phase_name(Some(&p1)), "DaySpeech");

        let p2 = json!({"DayHunterShoot": ["hunter", "NightWolf"]});
        assert_eq!(parse_phase_name(Some(&p2)), "DayHunterShoot");

        let p3 = json!({"GameOver": "Wolves"});
        assert_eq!(parse_phase_name(Some(&p3)), "GameOver");
    }

    #[test]
    fn test_get_player_role() {
        let state = json!([
            {"id": "Player1", "role": "Werewolf", "is_alive": true},
            {"id": "Player2", "role": "Seer", "is_alive": false}
        ]);

        assert_eq!(get_player_role(Some(&state), "Player1"), "Werewolf");
        assert_eq!(get_player_role(Some(&state), "Player2"), "Seer");
        assert_eq!(get_player_role(Some(&state), "Player3"), "未知");
        assert_eq!(get_player_role(None, "Player1"), "未知");
    }

    #[test]
    fn test_get_player_alive() {
        let state = json!([
            {"id": "Player1", "role": "Werewolf", "is_alive": true},
            {"id": "Player2", "role": "Seer", "is_alive": false}
        ]);

        assert!(get_player_alive(Some(&state), "Player1"));
        assert!(!get_player_alive(Some(&state), "Player2"));
        assert!(!get_player_alive(Some(&state), "Player3"));
        assert!(!get_player_alive(None, "Player1"));
    }
}
