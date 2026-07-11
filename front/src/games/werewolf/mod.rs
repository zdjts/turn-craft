use dioxus::prelude::*;
use serde_json::Value;

use super::GamePluginProps;
use crate::games::registry::GameConfigProps;
use crate::services::websocket::WsBridge;

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
        div { class: "g-field",
            label { "游戏模式" }
            div { class: "mode-toggle",
                button {
                    class: if !*spectator_mode.read() { "mode-btn selected" } else { "mode-btn" },
                    onclick: move |_| {
                        spectator_mode.set(false);
                        let mut modes = std::collections::HashMap::new();
                        modes.insert("Player1".to_string(), "human".to_string());
                        for i in 2..=7 {
                            let prev = role_config.read().get(&format!("Player{}", i)).cloned().unwrap_or_else(|| "ai".to_string());
                            modes.insert(format!("Player{}", i), prev);
                        }
                        my_role.set("Player1".to_string());
                        role_config.set(modes);
                    },
                    div { class: "mode-icon", "🎮" }
                    div { class: "mode-label", "亲自上阵" }
                    div { class: "mode-desc", "你可以设置多个座位为真人联机" }
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
                    div { class: "mode-desc", "观看全 AI 之间的对局" }
                }
            }
        }

        if !*spectator_mode.read() {
            div { class: "g-field",
                label { "联机席位配置" }
                div { class: "seats-toggle-grid",
                    for i in 2..=7 {
                        {
                            let slot_name = format!("Player{}", i);
                            let is_human = role_config.read().get(&slot_name).map(|s| s.as_str()) == Some("human");
                            rsx! {
                                button {
                                    key: "{slot_name}",
                                    class: if is_human { "seat-btn human" } else { "seat-btn ai" },
                                    onclick: move |_| {
                                        let mut modes = role_config.read().clone();
                                        if modes.get(&slot_name).map(|s| s.as_str()) == Some("human") {
                                            modes.insert(slot_name.clone(), "ai".to_string());
                                        } else {
                                            modes.insert(slot_name.clone(), "human".to_string());
                                        }
                                        role_config.set(modes);
                                    },
                                    div { class: "seat-icon", if is_human { "👤" } else { "🤖" } }
                                    div { class: "seat-label", "Player {i}" }
                                    div { class: "seat-status", if is_human { "开放联机" } else { "AI 接管" } }
                                }
                            }
                        }
                    }
                }
            }
        }

        div { class: "g-field",
            label { "角色配置 (7人局)" }
            div { class: "role-grid",
                for i in 1..=7 {
                    {
                        let rn = format!("Player{}", i);
                        let is_self = *my_role.read() == rn;
                        rsx! {
                            div {
                                class: if is_self { "role-card selected" } else { "role-card" },
                                div { class: "role-card-header",
                                    span {
                                        class: "role-card-name",
                                        if is_self { "👉 " }
                                        "{rn}"
                                        if is_self { " (我的角色)" }
                                    }
                                }
                                div { class: "role-card-desc", if is_self { "你的席位" } else { "玩家槽位" } }
                            }
                        }
                    }
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
        div { class: "pg-lincoln",
            // ── 时间轴区域 ──
            div { class: "gm-timeline",
                // 顶部信息栏
                div { class: "gm-phase",
                    div { class: "gm-phase-title",
                        "🐺 狼人杀 — 7人标准局"
                    }
                    div { class: "gm-phase-round",
                        "第 {day} 天 — {phase_name}"
                    }
                    button {
                        class: "g-card-subtle gm-ai-toggle",
                        style: "margin-left: auto; font-size: 0.85em; padding: 4px 12px; cursor: pointer;",
                        onclick: move |_| {
                            let cur = *show_ai_content.read();
                            show_ai_content.set(!cur);
                        },
                        if *show_ai_content.read() { "👀 隐藏 AI 思考" } else { "🙈 显示 AI 思考" }
                    }
                }

                {
                    let players = state.get("players").and_then(|p| p.as_array()).cloned().unwrap_or_default();
                    rsx! {
                        div { class: "players-status-bar", style: "display: flex; gap: 8px; padding: 10px 20px; flex-wrap: wrap; background: var(--bg-card); border-bottom: 1px solid var(--border-subtle);",
                            for p in players.iter() {
                                {
                                    let id = p.get("id").and_then(|i| i.as_str()).unwrap_or("");
                                    let alive = p.get("is_alive").and_then(|a| a.as_bool()).unwrap_or(false);
                                    let known_role = p.get("role").and_then(|r| r.as_str());

                                    let op = if alive { "1.0" } else { "0.4" };
                                    let txt_color = if alive { "var(--text-primary)" } else { "var(--text-muted)" };
                                    let mut bg = if alive { "var(--accent-dim)" } else { "transparent" };

                                    // Highlight self or known wolf teammates
                                    if id == my_id {
                                        bg = "rgba(255, 215, 0, 0.2)"; // Gold for self
                                    } else if known_role == Some("Werewolf") {
                                        bg = "rgba(255, 50, 50, 0.2)"; // Red for wolf teammates
                                    }

                                    rsx! {
                                        div {
                                            key: "{id}",
                                            style: "padding: 4px 10px; border-radius: 12px; font-size: 0.85em; opacity: {op}; color: {txt_color}; background: {bg}; border: 1px solid var(--border-subtle); transition: all 0.3s; display: flex; align-items: center; gap: 4px;",
                                            if !alive { "💀 " } else { "👤 " }
                                            "{id}"
                                            if known_role == Some("Werewolf") {
                                                span { style: "font-size: 1.1em;", "🐺" }
                                            } else if let Some(r) = known_role {
                                                // Just in case other roles are exposed later
                                                span { style: "font-size: 1.1em; opacity: 0.7;", "[{r}]" }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                if !is_finished {
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
                                        div { class: "gm-timeline-item", key: "{idx}",
                                            div { class: if is_sys { "gm-timeline-avatar" } else { "gm-timeline-avatar pro" },
                                                if is_sys { "⚖️" } else { "👤" }
                                            }
                                            div { class: "gm-timeline-body",
                                                div { class: "gm-timeline-meta",
                                                    span { class: "gm-timeline-author", if is_sys { "系统播报" } else { "{actor}" } }
                                                    span { class: "gm-timeline-tag", "Day {d}" }
                                                }
                                                div { class: if is_sys { "gm-timeline-content sys-msg" } else { "gm-timeline-content" },
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

                // ── 流式输出气泡 (AI 正在发言中) ──
                {
                    let bridge = use_context::<WsBridge>();
                    let streaming = bridge.streaming_text.read();
                    let active_actor = state.get("active_actor").and_then(|v| v.as_str());
                    let streaming_entry = active_actor
                        .and_then(|active_id| {
                            streaming.get(active_id).map(|text| (active_id.to_string(), text.clone()))
                        });
                    if let Some((active_id, text)) = streaming_entry {
                        if !text.is_empty() {
                            rsx! {
                                div { class: "gm-timeline-item gm-streaming",
                                    div { class: "gm-timeline-avatar pro", "👤" }
                                    div { class: "gm-timeline-body",
                                        div { class: "gm-timeline-meta",
                                            span { class: "gm-timeline-author", "{active_id}" }
                                            span { class: "gm-streaming-indicator", "⏳ 生成中..." }
                                        }
                                        div { class: "gm-timeline-content",
                                            "{text}"
                                            span { class: "cursor-blink", "█" }
                                        }
                                    }
                                }
                            }
                        } else {
                            rsx! {}
                        }
                    } else {
                        rsx! {}
                    }
                }

                if is_finished {
                    div { class: "showdown-panel", style: "margin-top: 20px;",
                        div { class: "showdown-title",
                            if let Some(winner) = state.get("phase").and_then(|p| p.get("GameOver")).and_then(|g| g.as_str()) {
                                if winner == "Wolves" { "🐺 狼人阵营胜利！" } else { "🧑‍🌾 好人阵营胜利！" }
                            } else {
                                "🏆 游戏结束！"
                            }
                        }
                        div { class: "showdown-cards",
                            {
                                let players = state.get("players").and_then(|p| p.as_array()).cloned().unwrap_or_default();
                                rsx! {
                                    for p in players.iter() {
                                        {
                                            let id = p.get("id").and_then(|i| i.as_str()).unwrap_or("");
                                            let role = p.get("role").and_then(|r| r.as_str()).unwrap_or("未知");
                                            let alive = p.get("is_alive").and_then(|a| a.as_bool()).unwrap_or(false);
                                            let winner_side = state.get("phase").and_then(|ph| ph.get("GameOver")).and_then(|g| g.as_str()).unwrap_or("");

                                            let is_wolf = role == "Werewolf";
                                            let is_winner = (winner_side == "Wolves" && is_wolf) || (winner_side == "Humans" && !is_wolf);

                                            rsx! {
                                                div {
                                                    class: if is_winner { "showdown-player winner" } else { "showdown-player" },
                                                    key: "{id}",
                                                    div { class: "showdown-name", "{id}" }
                                                    div { class: "showdown-hand", style: "font-size: 1.5em; margin: 10px 0;",
                                                        match role {
                                                            "Werewolf" => "🐺",
                                                            "Seer" => "👁️",
                                                            "Witch" => "🧪",
                                                            "Hunter" => "🔫",
                                                            "Villager" => "🧑‍🌾",
                                                            _ => "❓",
                                                        }
                                                    }
                                                    div { class: "showdown-rank",
                                                        "{role}"
                                                        if !alive { " (阵亡)" }
                                                    }
                                                    if is_winner {
                                                        div { class: "showdown-winner-badge", "🏆" }
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

            // ── 操作面板 ──
            div { class: if (is_my_turn && !is_finished) || phase_name == "Init" || (phase_name == "DayVote" && my_alive) || (phase_name == "NightWolf" && my_role == "Werewolf" && my_alive) { "gm-action-bar" } else { "gm-action-bar locked" },
                if phase_name == "Init" {
                    div { class: "gm-action-row", style: "justify-content: center;",
                        button {
                            class: "gm-action-submit",
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
                    div { class: "gm-action-row", style: "justify-content: flex-end;",
                        button {
                            class: "gm-action-submit",
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
                    div { class: "gm-action-hint", "等待其他玩家行动..." }
                } else {
                    div { class: "gm-action-hint", "游戏已结束" }
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
        div { class: "gm-action-row", style: "flex-wrap: wrap; gap: 10px;",
            if phase_name == "NightWolf" && my_role == "Werewolf" {
                textarea {
                    class: "gm-action-input",
                    placeholder: "狼队频道：输入战术沟通（按回车发送，或输入后直接点击下方杀人按钮同时发送）...",
                    value: "{text_input}",
                    style: "width: 100%; margin-bottom: 10px;",
                    oninput: move |e| text_input.set(e.value()),
                    onkeydown: move |e: Event<KeyboardData>| {
                        if e.key() == Key::Enter {
                            if e.modifiers().ctrl() {
                                text_input.write().push('\n');
                            } else {
                                let content = text_input.read().trim().to_string();
                                if !content.is_empty() {
                                    on_action.call(serde_json::json!({ "action_type": "speak", "content": content }));
                                    text_input.write().clear();
                                }
                            }
                        }
                    },
                }
                div { style: "width: 100%; display: flex; flex-wrap: wrap; gap: 10px;",
                    span { class: "gm-action-hint", "🐺 选择击杀目标: " }
                    for target in alive_players {
                        button {
                            class: "gm-action-submit",
                            style: "background: var(--red);",
                            onclick: move |_| {
                                let content = text_input.read().trim().to_string();
                                if content.is_empty() {
                                    on_action.call(serde_json::json!({ "action_type": "kill", "target": target }));
                                } else {
                                    on_action.call(serde_json::json!({ "action_type": "kill", "target": target, "content": content }));
                                    text_input.write().clear();
                                }
                            },
                            "杀 {target}"
                        }
                    }
                }
            } else if phase_name == "NightSeer" && my_role == "Seer" {
                span { class: "gm-action-hint", "👁️ 选择查验目标: " }
                for target in alive_players {
                    button {
                        class: "gm-action-submit",
                        style: "background: #8b5cf6;",
                        onclick: move |_| {
                            on_action.call(serde_json::json!({ "action_type": "check", "target": target }));
                        },
                        "查 {target}"
                    }
                }
            } else if phase_name == "NightWitch" && my_role == "Witch" {
                span { class: "gm-action-hint", "🧪 女巫行动: " }
                if state.get("witch_has_save").and_then(|v| v.as_bool()).unwrap_or(false) {
                    button {
                        class: "gm-action-submit",
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
                            class: "gm-action-submit",
                            style: "background: var(--red);",
                            onclick: move |_| {
                                on_action.call(serde_json::json!({ "action_type": "poison", "target": target }));
                            },
                            "毒 {target}"
                        }
                    }
                }
                button {
                    class: "gm-action-submit",
                    style: "background: #6b7280;",
                    onclick: move |_| {
                        on_action.call(serde_json::json!({ "action_type": "skip" }));
                    },
                    "跳过"
                }
            } else if phase_name == "DaySpeech" {
                textarea {
                    class: "gm-action-input",
                    placeholder: "请输入你的发言...",
                    value: "{text_input}",
                    oninput: move |e| text_input.set(e.value()),
                    onkeydown: move |e: Event<KeyboardData>| {
                        if e.key() == Key::Enter {
                            if e.modifiers().ctrl() {
                                text_input.write().push('\n');
                            } else {
                                let content = text_input.read().trim().to_string();
                                if !content.is_empty() {
                                    on_action.call(serde_json::json!({ "action_type": "speak", "content": content }));
                                }
                                text_input.write().clear();
                            }
                        }
                    },
                }
                button {
                    class: "gm-action-submit",
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
                span { class: "gm-action-hint", "🗳️ 请投票: " }
                for target in alive_players {
                    button {
                        class: "gm-action-submit",
                        onclick: move |_| {
                            on_action.call(serde_json::json!({ "action_type": "vote", "target": target }));
                        },
                        "投 {target}"
                    }
                }
                button {
                    class: "gm-action-submit",
                    style: "background: #6b7280;",
                    onclick: move |_| {
                        on_action.call(serde_json::json!({ "action_type": "skip" }));
                    },
                    "弃票"
                }
            } else if let Some(obj) = state.get("phase").and_then(|p| p.as_object()) {
                if obj.contains_key("DayHunterShoot") && my_role == "Hunter" {
                    span { class: "gm-action-hint", "🔫 开枪带人: " }
                    for target in alive_players {
                        button {
                            class: "gm-action-submit",
                            style: "background: var(--red);",
                            onclick: move |_| {
                                on_action.call(serde_json::json!({ "action_type": "shoot", "target": target }));
                            },
                            "带走 {target}"
                        }
                    }
                    button {
                        class: "gm-action-submit",
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
