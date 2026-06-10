use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::{debug, info, warn};

use super::GamePluginProps;

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

    rsx! {
        div { class: "lincoln-shell",
            // ── 时间轴区域 ──
            div { class: "timeline-scroll",
                if let Some(ref s) = lincoln() {
                    // 顶部信息栏
                    div { class: "timeline-header",
                        div { class: "timeline-title",
                            "🏛️ 林肯 — 道格拉斯辩论"
                        }
                        div { class: "timeline-round",
                            "轮次 {s.round} / {s.max_round}"
                        }
                    }

                    // 历史气泡
                    for (idx, entry) in s.history.iter().enumerate() {
                        HistoryBubble {
                            key: "{idx}:{entry.actor_id}",
                            entry: entry.clone(),
                        }
                    }

                    // 空状态
                    if s.history.is_empty() {
                        div { class: "timeline-empty",
                            div { class: "timeline-empty-icon", "⚖️" }
                            p { class: "timeline-empty-text",
                                "等待裁判宣读辩题..."
                            }
                        }
                    }
                } else {
                    div { class: "timeline-syncing",
                        div { class: "sync-spinner" }
                        span { "正在同步对局状态..." }
                    }
                }
            }

            // ── 行动舱 ──
            div {
                class: if is_my_turn() {
                    "action-console"
                } else {
                    "action-console locked"
                },
                div { class: "console-row",
                    textarea {
                        class: "console-textarea",
                        placeholder: if is_my_turn() {
                            "作为裁判，宣读你的辩题..."
                        } else {
                            "等待你的回合..."
                        },
                        value: "{draft}",
                        oninput: move |e| draft.set(e.value()),
                        onkeydown: move |e: Event<KeyboardData>| {
                            if e.key() == Key::Enter && e.modifiers().ctrl() {
                                let content = draft.read().trim().to_string();
                                if content.is_empty() {
                                    warn!(target: "lincoln::action", "用户尝试发送空发言，已忽略");
                                    return;
                                }
                                info!(target: "lincoln::action", content_len = content.len(), "裁判通过 Ctrl+Enter 发射发言");
                                on_action.call(serde_json::json!({"content": content}));
                                draft.write().clear();
                            }
                        },
                    }
                    button {
                        class: "console-submit",
                        disabled: !is_my_turn(),
                        onclick: move |_| {
                            let content = draft.read().trim().to_string();
                            if content.is_empty() {
                                warn!(target: "lincoln::action", "用户尝试发送空发言，已忽略");
                                return;
                            }
                            info!(target: "lincoln::action", content_len = content.len(), "裁判通过按钮发射发言");
                            on_action.call(serde_json::json!({"content": content}));
                            draft.write().clear();
                        },
                        "发射"
                    }
                }
                div { class: "console-hint",
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

    rsx! {
        div { class: "bubble-row",
            div { class: "bubble-avatar {role_cls}",
                "{icon}"
            }
            div { class: "bubble-body",
                div { class: "bubble-meta",
                    span { class: "bubble-name", "{e.actor_id}" }
                    span { class: "bubble-tag {role_cls}",
                        "{label}"
                    }
                }
                div { class: "bubble-content {role_cls}",
                    "{e.content}"
                }
            }
        }
    }
}
