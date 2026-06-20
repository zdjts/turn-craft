use dioxus::prelude::*;
use serde_json::Value;
use crate::api::{get_room, RoomSnapshotData};
use crate::games::registry::REGISTRY;
use crate::games::lincoln::{LincolnState, HistoryEntry};
use crate::routes::layout::use_toast;

#[component]
pub fn Replay(room_id: String) -> Element {
    let toast = use_toast();
    let nav = use_navigator();
    let mut room_data = use_signal(|| Option::<RoomSnapshotData>::None);
    let mut loading = use_signal(|| true);

    use_effect(move || {
        let rid = room_id.clone();
        spawn(async move {
            loading.set(true);
            match get_room(&rid).await {
                Ok(r) => {
                    room_data.set(Some(r));
                }
                Err(e) => {
                    toast.show(format!("加载对局记录失败: {e}"), crate::routes::layout::ToastType::Error);
                }
            }
            loading.set(false);
        });
    });

    rsx! {
        div { class: "replay-container animate-fade-in",
            div { class: "page-header",
                div { class: "header-left",
                    h1 { "🎞️ 对局回放记录" }
                    p { "这里是该房间历史状态与局内对话的静态复盘记录。" }
                }
                button {
                    class: "back-btn glass-panel-subtle",
                    onclick: move |_| { nav.push(super::Route::History {}); },
                    "⬅️ 返回历史列表"
                }
            }

            if *loading.read() {
                div { class: "loading-canvas glass-panel",
                    span { class: "spinner" }
                    p { "正在读取对局记录..." }
                }
            } else if let Some(ref r) = *room_data.read() {
                div { class: "replay-details-layout glass-panel",
                    // Header Meta
                    div { class: "replay-meta-header",
                        div { class: "meta-badge",
                            if let Some(game_def) = REGISTRY.get(&r.game_type) {
                                "{game_def.icon} {game_def.name}"
                            } else {
                                "❓ 未知游戏"
                            }
                        }
                        div { class: "meta-item", "房间 ID: {r.room_id}" }
                        div { class: "meta-item", "总局数: {r.max_round} 轮" }
                        div { class: "meta-item", "创建时间: {r.created_at.replace(\"T\", \" \")}" }
                    }

                    // Body
                    div { class: "replay-body",
                        if r.game_type == "lincoln" {
                            LincolnReplayView { engine_state: r.engine_state.clone() }
                        } else if r.game_type == "texas_holdem" {
                            TexasHoldemReplayView { engine_state: r.engine_state.clone() }
                        } else {
                            div { class: "unsupported-replay",
                                p { "该游戏类型暂时不支持高级复盘显示。" }
                            }
                        }
                    }
                }
            } else {
                div { class: "error-canvas glass-panel",
                    p { "未能加载到该对局数据。" }
                }
            }
        }
    }
}

#[component]
fn LincolnReplayView(engine_state: Value) -> Element {
    let state = use_memo(move || {
        serde_json::from_value::<LincolnState>(engine_state.clone()).ok()
    });

    let mut show_ai_content = use_signal(|| true);

    rsx! {
        div { class: "lincoln-replay-view",
            if let Some(ref s) = *state.read() {
                div { class: "lincoln-replay-inner",
                    div { class: "replay-round-indicator", 
                        span { "🏛️ 林肯辩论历史辩词 (共 {s.round} 轮)" }
                        button {
                            class: "glass-panel-subtle toggle-ai-btn",
                            style: "margin-left: 15px; font-size: 0.85em; padding: 4px 12px; cursor: pointer;",
                            onclick: move |_| {
                                let cur = *show_ai_content.read();
                                show_ai_content.set(!cur);
                            },
                            if *show_ai_content.read() { "👀 隐藏 AI 发言" } else { "🙈 显示 AI 发言" }
                        }
                    }
                    div { class: "timeline-scroll replay-timeline",
                        if s.history.is_empty() {
                            p { style: "color: var(--text-muted); text-align: center; padding: 20px;", "没有发言记录" }
                        } else {
                            for entry in s.history.iter() {
                                LincolnHistoryBubble { 
                                    entry: entry.clone(),
                                    is_ai: s.actors.iter().any(|a| a.id == entry.actor_id && a.kind.to_lowercase() == "ai"),
                                    show_ai_content: *show_ai_content.read(),
                                }
                            }
                        }
                    }
                }
            } else {
                p { "解析 LincolnState 数据失败" }
            }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
struct LincolnHistoryBubbleProps {
    entry: HistoryEntry,
    #[props(default = false)]
    is_ai: bool,
    #[props(default = true)]
    show_ai_content: bool,
}

#[component]
fn LincolnHistoryBubble(props: LincolnHistoryBubbleProps) -> Element {
    let entry = &props.entry;
    let (role_cls, icon, label) = match entry.role.as_str() {
        "Judge" => ("judge", "👑", "裁判"),
        "Pro" => ("pro", "🟢", "正方"),
        "Con" => ("con", "🔴", "反方"),
        _ => ("", "❓", "未知"),
    };

    let should_hide = props.is_ai && !props.show_ai_content;

    rsx! {
        div { key: "{entry.actor_id}:{entry.content.len()}", class: "bubble-row",
            div { class: "bubble-avatar {role_cls}", "{icon}" }
            div { class: "bubble-body",
                div { class: "bubble-meta",
                    span { class: "bubble-name", "{entry.actor_id}" }
                    span { class: "bubble-tag {role_cls}", "{label}" }
                }
                div { class: "bubble-content {role_cls}", 
                    if should_hide {
                        span { class: "hidden-content-hint", style: "color: #888; font-style: italic;", "🤖 AI 发言已隐藏" }
                    } else {
                        "{entry.content}"
                    }
                }
            }
        }
    }
}

#[component]
fn TexasHoldemReplayView(engine_state: Value) -> Element {
    let pot = engine_state.get("pot").and_then(|v| v.as_u64()).unwrap_or(0);
    let phase = engine_state.get("phase").and_then(|v| v.as_str()).unwrap_or("Unknown");
    
    let es_for_memo = engine_state.clone();
    let es_for_showdown = engine_state.clone();

    // Community cards
    let community_cards_list = use_memo(move || {
        let mut list = Vec::new();
        if let Some(cards) = es_for_memo.get("community_cards").and_then(|c| c.as_array()) {
            for c in cards {
                let suit = c.get("suit").and_then(|s| s.as_str()).unwrap_or("");
                let rank = c.get("rank").and_then(|r| r.as_str()).unwrap_or("");
                list.push((suit.to_string(), rank.to_string()));
            }
        }
        list
    });

    rsx! {
        div { class: "texas-replay-view",
            div { class: "texas-replay-stats",
                div { class: "stat-card glass-panel-subtle",
                    div { class: "stat-val", "💰 {pot}" }
                    div { class: "stat-lbl", "总奖池" }
                }
                div { class: "stat-card glass-panel-subtle",
                    div { class: "stat-val", "{phase}" }
                    div { class: "stat-lbl", "最后阶段" }
                }
            }

            // Community Cards Display
            div { class: "community-cards-section",
                h4 { "🃏 公共牌" }
                div { class: "community-cards-list",
                    if community_cards_list().is_empty() {
                        p { class: "empty-lbl", "无公共牌" }
                    } else {
                        for (suit, rank) in community_cards_list().iter() {
                            MiniPokerCard { suit: suit.clone(), rank: rank.clone() }
                        }
                    }
                }
            }

            // Showdown results if any
            if let Some(results) = es_for_showdown.get("showdown_results").and_then(|r| r.as_array()) {
                if !results.is_empty() {
                    div { class: "showdown-replay-section",
                        h4 { "🏆 摊牌与结算结果" }
                        div { class: "showdown-list",
                            for res in results.iter() {
                                ShowdownReplayItem { res: res.clone() }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn MiniPokerCard(suit: String, rank: String) -> Element {
    let suit_symbol = match suit.as_str() {
        "Hearts" => "♥",
        "Diamonds" => "♦",
        "Clubs" => "♣",
        "Spades" => "♠",
        _ => "?",
    };
    let card_color = if suit == "Hearts" || suit == "Diamonds" { "card-red" } else { "card-black" };
    rsx! {
        div { class: "poker-card-mini {card_color}",
            span { class: "rank", "{rank}" }
            span { class: "suit", "{suit_symbol}" }
        }
    }
}

#[component]
fn ShowdownReplayItem(res: Value) -> Element {
    let p_id = res.get("player_id").and_then(|v| v.as_str()).unwrap_or("?");
    let rank_desc = res.get("hand_rank").and_then(|v| v.as_str()).unwrap_or("?");
    let winner = res.get("is_winner").and_then(|v| v.as_bool()).unwrap_or(false);
    rsx! {
        div { class: if winner { "showdown-item winner glass-panel-subtle" } else { "showdown-item" },
            span { class: "showdown-winner-icon", if winner { "👑" } else { "👤" } }
            span { class: "showdown-player", "{p_id}" }
            span { class: "showdown-rank", "{rank_desc}" }
            if winner {
                span { class: "winner-tag", "获胜者" }
            }
        }
    }
}
