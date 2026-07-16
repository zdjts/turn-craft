use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::{debug, info, warn};

pub mod actions;
use super::GamePluginProps;
use crate::services::connection::ConnectionManager;
use actions::submit_litigation;

// ═══════════════════════════════════════════════════════
//  强类型结构 — 从不透明 Value 解冻
// ═══════════════════════════════════════════════════════

/// 林肯辩论状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LincolnState {
    pub game_type: String,
    pub room_id: String,
    pub actors: Vec<ActorInfo>,
    pub active_actor: Option<String>,
    pub round: u64,
    pub max_round: u64,
    pub finished: bool,
    pub history: Vec<HistoryEntry>,
}

/// 参与者信息
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ActorInfo {
    pub id: String,
    pub kind: String,
    pub role: String,
}

/// 历史条目
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HistoryEntry {
    pub actor_id: String,
    pub role: String,
    pub content: String,
}

// ═══════════════════════════════════════════════════════
//  组件
// ═══════════════════════════════════════════════════════

/// 林肯辩论游戏组件
#[component]
pub fn LincolnGame(props: GamePluginProps) -> Element {
    let state = props.state;
    let on_action = props.on_action;
    let my_actor_id = props.actor_id.clone();

    let lincoln = use_memo(move || {
        let raw: Value = state();
        match serde_json::from_value::<LincolnState>(raw.clone()) {
            Ok(s) => {
                debug!(target: "lincoln", round = s.round, active = ?s.active_actor, history_len = s.history.len(), "状态解冻成功");
                Some(s)
            }
            Err(e) => {
                warn!(target: "lincoln", error = %e, raw = %raw, "LincolnState 反序列化失败");
                None
            }
        }
    });

    let mut draft = use_signal(|| String::new());

    let is_my_turn = use_memo(move || {
        lincoln()
            .as_ref()
            .map(|s| s.active_actor.as_deref() == Some(my_actor_id.as_str()))
            .unwrap_or(false)
    });

    let mut show_ai_content = use_signal(|| true);

    rsx! {
        div { class: "pg-lincoln",
            // ── 时间轴区域 ──
            div { class: "gm-timeline",
                if let Some(ref s) = lincoln() {
                    // 顶部信息栏
                    div { class: "gm-phase",
                        div { class: "gm-phase-title",
                            "🏛️ 林肯 — 道格拉斯辩论"
                        }
                        div { class: "gm-phase-round",
                            "轮次 {s.round} / {s.max_round}"
                        }
                        button {
                            class: "g-card-subtle gm-ai-toggle",
                            style: "margin-left: auto; font-size: 0.85em; padding: 4px 12px; cursor: pointer;",
                            onclick: move |_| {
                                let cur = *show_ai_content.read();
                                show_ai_content.set(!cur);
                            },
                            if *show_ai_content.read() { "👀 隐藏 AI 发言" } else { "🙈 显示 AI 发言" }
                        }
                    }

                    // 历史气泡
                    for (idx, entry) in s.history.iter().enumerate() {
                        HistoryBubble {
                            key: "{idx}:{entry.actor_id}",
                            entry: entry.clone(),
                            is_ai: s.actors.iter().any(|a| a.id == entry.actor_id && a.kind.to_lowercase() == "ai"),
                            show_ai_content: *show_ai_content.read(),
                        }
                    }

                    // ── 流式输出气泡 (AI 正在生成中) ──
                    {
                        let bridge = use_context::<ConnectionManager>();
                        let streaming = bridge.streaming_text.read();
                        // 找到当前 active_actor 是否有流式内容
                        let streaming_entry = s.active_actor.as_ref()
                            .and_then(|active_id| {
                                streaming.get(active_id).map(|text| (active_id.clone(), text.clone()))
                            });
                        if let Some((active_id, text)) = streaming_entry {
                            if !text.is_empty() {
                                // 找到该 actor 的角色信息
                                let actor_info = s.actors.iter().find(|a| a.id == active_id);
                                let role_str = actor_info.map(|a| a.role.as_str()).unwrap_or("?");
                                let (role_cls, icon, label) = match role_str {
                                    "Judge" => ("judge", "👑", "裁判"),
                                    "Pro" => ("pro", "🟢", "正方"),
                                    "Con" => ("con", "🔴", "反方"),
                                    _ => ("", "❓", "未知"),
                                };
                                rsx! {
                                    div { class: "gm-timeline-item gm-streaming",
                                        div { class: "gm-timeline-avatar {role_cls}",
                                            "{icon}"
                                        }
                                        div { class: "gm-timeline-body",
                                            div { class: "gm-timeline-meta",
                                                span { class: "gm-timeline-author", "{active_id}" }
                                                span { class: "gm-timeline-tag {role_cls}", "{label}" }
                                                span { class: "gm-streaming-indicator", "⏳ 生成中..." }
                                            }
                                            div { class: "gm-timeline-content {role_cls}",
                                                "{text}"
                                                span { class: "gm-streaming-cursor", "█" }
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

                    // 空状态
                    if s.history.is_empty() {
                        div { class: "gm-empty",
                            div { class: "gm-empty-icon", "⚖️" }
                            p { class: "gm-empty-text",
                                "等待裁判宣读辩题..."
                            }
                        }
                    }
                } else {
                    div { class: "gm-syncing",
                        div { class: "sync-g-spinner" }
                        span { "正在同步对局状态..." }
                    }
                }
            }

            // ── 行动舱 ──
            div {
                class: if is_my_turn() {
                    "gm-action-bar"
                } else {
                    "gm-action-bar locked"
                },
                div { class: "gm-action-row",
                    textarea {
                        class: "gm-action-input",
                        placeholder: if is_my_turn() {
                            "作为裁判，宣读你的辩题..."
                        } else {
                            "等待你的回合..."
                        },
                        value: "{draft}",
                        oninput: move |e| draft.set(e.value()),
                        onkeydown: move |e: Event<KeyboardData>| {
                            if e.key() == Key::Enter {
                                if e.modifiers().ctrl() {
                                    draft.write().push('\n');
                                } else {
                                    let content = draft.read().trim().to_string();
                                    if !content.is_empty() {
                                        info!(target: "lincoln::action", content_len = content.len(), "裁判通过 Enter 提交发言");
                                        on_action.call(serde_json::json!({"content": content}));
                                    }
                                    draft.write().clear();
                                }
                            }
                        },
                    }
                    button {
                        class: "gm-action-submit",
                        disabled: !is_my_turn(),
                        onclick: move |_| submit_litigation(draft, on_action, "button"),
                        "提交"
                    }
                }
                div { class: "gm-action-hint",
                    "Ctrl + Enter 快速发送"
                }
            }
        }
    }
}

// ═══════════════════════════════════════════════════════
//  气泡子组件
// ═══════════════════════════════════════════════════════

/// 历史气泡属性
#[derive(Props, Clone, PartialEq)]
struct HistoryBubbleProps {
    entry: HistoryEntry,
    #[props(default = false)]
    is_ai: bool,
    #[props(default = true)]
    show_ai_content: bool,
}

/// 历史气泡组件：显示单条发言
#[component]
fn HistoryBubble(props: HistoryBubbleProps) -> Element {
    let e = &props.entry;

    let (role_cls, icon, label) = match e.role.as_str() {
        "Judge" => ("judge", "👑", "裁判"),
        "Pro" => ("pro", "🟢", "正方"),
        "Con" => ("con", "🔴", "反方"),
        _ => ("", "❓", "未知"),
    };

    let should_hide = props.is_ai && !props.show_ai_content;

    rsx! {
        div { class: "gm-timeline-item",
            div { class: "gm-timeline-avatar {role_cls}",
                "{icon}"
            }
            div { class: "gm-timeline-body",
                div { class: "gm-timeline-meta",
                    span { class: "gm-timeline-author", "{e.actor_id}" }
                    span { class: "gm-timeline-tag {role_cls}",
                        "{label}"
                    }
                }
                div { class: "gm-timeline-content {role_cls}",
                    if should_hide {
                        span { class: "hidden-content-hint", style: "color: #888; font-style: italic;", "🤖 AI 发言已隐藏" }
                    } else {
                        "{e.content}"
                    }
                }
            }
        }
    }
}

const LINCOLN_ROLES: &[(&str, &str)] = &[
    ("Judge", "裁判 — 开题与总结"),
    ("Pro", "正方 — 立论"),
    ("Con", "反方 — 驳论"),
];

pub fn lincoln_lobby_card(props: crate::games::registry::GameConfigProps) -> Element {
    let mut role_config = props.role_config;
    let mut my_role = props.my_role;
    let mut max_round = props.max_round;
    let mut game_config = props.game_config;

    // Ensure state is initialized for Lincoln if not already
    use_effect(move || {
        if my_role.read().is_empty() || !["Judge", "Pro", "Con"].contains(&my_role.read().as_str())
        {
            my_role.set("Judge".to_string());
            role_config.set(std::collections::HashMap::from([
                ("Judge".to_string(), "human".to_string()),
                ("Pro".to_string(), "ai".to_string()),
                ("Con".to_string(), "ai".to_string()),
            ]));
            game_config.set(None);
        }
    });

    let mut select_role = move |rn: String| {
        my_role.set(rn.clone());
        let mut modes = role_config.read().clone();
        modes.insert(rn, "human".to_string());
        role_config.set(modes);
    };

    let mut toggle_role_mode = move |rn: String| {
        let mut modes = role_config.read().clone();
        let current = modes.get(&rn).cloned().unwrap_or("ai".to_string());
        if current == "human" {
            // Cannot change my own role to ai
            if *my_role.read() != rn {
                modes.insert(rn, "ai".to_string());
            }
        } else {
            modes.insert(rn, "human".to_string());
        }
        role_config.set(modes);
    };

    rsx! {
        div { class: "g-field",
            label { "角色配置" }
            div { class: "role-grid",
                for (role_name, role_desc) in LINCOLN_ROLES.iter() {
                    {
                        let rn = role_name.to_string();
                        let is_selected = *my_role.read() == rn;
                        let mode = role_config.read().get(&rn).cloned().unwrap_or("ai".to_string());
                        let is_human = mode == "human";

                        let rn_for_select = rn.clone();
                        let rn_for_toggle = rn.clone();

                        rsx! {
                            div {
                                class: if is_selected { "role-card selected" } else { "role-card" },
                                div { class: "role-card-header",
                                    span {
                                        class: "role-card-name",
                                        style: "cursor: pointer;",
                                        onclick: move |_| select_role(rn_for_select.clone()),
                                        if is_selected { "👉 " }
                                        "{role_name} (我的角色)"
                                    }
                                    button {
                                        class: if is_human { "g-badge-success" } else { "g-badge-info" },
                                        style: "border: none; cursor: pointer;",
                                        onclick: move |e| {
                                            e.stop_propagation();
                                            toggle_role_mode(rn_for_toggle.clone());
                                        },
                                        if is_human { "开放联机" } else { "AI 接管" }
                                    }
                                }
                                div { class: "role-card-desc", "{role_desc}" }
                            }
                        }
                    }
                }
            }
        }

        div { class: "g-field",
            label { "最大轮次" }
            input {
                r#type: "number",
                placeholder: "16",
                value: "{max_round}",
                oninput: move |e| {
                    if let Ok(val) = e.value().parse::<usize>() {
                        max_round.set(val);
                    }
                },
            }
        }
    }
}
