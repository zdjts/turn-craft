use std::collections::HashMap;

use crate::api::{create_room, get_room, CreateRoomRequest, RoomSnapshotData};
use crate::games::lincoln::{HistoryEntry, LincolnState};
use crate::games::registry::REGISTRY;
use crate::routes::layout::use_toast;
use dioxus::prelude::*;
use serde_json::Value;

#[component]
pub fn Replay(room_id: String) -> Element {
    let toast = use_toast();
    let nav = use_navigator();
    let rid = room_id.clone();

    let mut room_data = use_signal(|| Option::<RoomSnapshotData>::None);
    let mut loading = use_signal(|| true);

    use_effect({
        let rid = rid.clone();
        move || {
            let rid = rid.clone();
            spawn(async move {
                loading.set(true);
                match get_room(&rid).await {
                    Ok(r) => {
                        room_data.set(Some(r));
                    }
                    Err(e) => {
                        toast.show(
                            format!("加载对局记录失败: {e}"),
                            crate::routes::layout::ToastType::Error,
                        );
                    }
                }
                loading.set(false);
            });
        }
    });

    // 分享文本
    let share_text = use_memo({
        let rid = rid.clone();
        move || {
        let share_rid = rid.clone();
        match room_data.read().as_ref() {
            Some(rd) => {
                let name = REGISTRY.get(&rd.game_type).map(|d| d.name).unwrap_or("?");
                let done = rd.engine_state.get("finished").and_then(|v| v.as_bool()).unwrap_or(false);
                let rnd = rd.engine_state.get("round").and_then(|v| v.as_u64()).unwrap_or(0);
                let pot = rd.engine_state.get("pot").and_then(|v| v.as_u64()).unwrap_or(0);
                let slot_count = rd.actor_slots.as_array().map(|a| a.len()).unwrap_or(0);
                let extra = if rd.game_type == "texas_holdem" { format!("奖池: {pot} | {slot_count} 人") } else { format!("共 {rnd} 轮 | {slot_count} 人") };
                format!("Turn Craft | {name} {}\n{extra}\n房间: {share_rid}", if done { "✅" } else { "⏳" })
            }
            None => String::new(),
        }
    }
    });

    // Play Again 数据
    let play_again_data = use_memo(move || {
        room_data.read().as_ref().map(|rd| {
            let gt = rd.game_type.clone();
            let max_rnd = rd.max_round;
            let slots: Vec<String> = rd.actor_slots.as_array().map(|arr| {
                arr.iter().filter_map(|s| s.get("slot_name").and_then(|n| n.as_str()).map(|n| n.to_string())).collect()
            }).unwrap_or_default();
            let mut configs = HashMap::new();
            if let Some(arr) = rd.actor_slots.as_array() {
                for s in arr {
                    if let (Some(name), Some(occ)) = (s.get("slot_name").and_then(|n| n.as_str()), s.get("occupant")) {
                        configs.insert(name.to_string(), if occ.as_str() == Some("Ai") { "ai".into() } else { "human".into() });
                    }
                }
            }
            let my_slot = rd.actor_slots.as_array().and_then(|arr| {
                arr.iter().find(|s| matches!(s.get("occupant").and_then(|o| o.as_str()), Some("Empty") | Some("Human")))
                    .and_then(|s| s.get("slot_name").and_then(|n| n.as_str()).map(|n| n.to_string()))
            }).unwrap_or_else(|| "spectator".to_string());
            let game_config = rd.engine_state.get("game_config").cloned();
            (gt, max_rnd, slots, configs, my_slot, game_config)
        })
    });

    let handle_play_again = {
        let nav = nav.clone();
        let toast = toast.clone();
        move |_| {
            if let Some(ref data) = *play_again_data.read() {
                let (ref gt, max_rnd, ref slots, ref configs, ref my_slot, ref game_config) = *data;
                let req = CreateRoomRequest {
                    game_type: gt.clone(),
                    max_round: max_rnd,
                    my_slot: my_slot.clone(),
                    slots: slots.clone(),
                    slot_configs: configs.clone(),
                    game_config: game_config.clone(),
                    is_public: true,
                };
                let nav = nav.clone();
                let toast = toast.clone();
                spawn(async move {
                    match create_room(&req).await {
                        Ok(resp) if resp.status == "success" => {
                            if let (Some(rid), Some(aid)) = (resp.room_id, resp.actor_id) {
                                nav.push(super::Route::Game { room_id: rid, actor_id: aid });
                            }
                        }
                        Ok(resp) => toast.show(resp.message.unwrap_or_else(|| "创建失败".into()), crate::routes::layout::ToastType::Error),
                        Err(e) => toast.show(format!("网络请求失败: {e}"), crate::routes::layout::ToastType::Error),
                    }
                });
            }
        }
    };

    rsx! {
        div { class: "pg-replay animate-fade-in",
            div { class: "page-header",
                div { class: "header-left",
                    h1 { "🎞️ 对局回放记录" }
                    p { "这里是该房间历史状态与局内对话的静态复盘记录。" }
                }
                div { class: "header-right",
                    button {
                        class: "pg-replay-back g-card-subtle",
                        onclick: move |_| { nav.push(super::Route::History {}); },
                        "⬅️ 返回历史列表"
                    }
                    button {
                        class: "pg-replay-share g-card-subtle",
                        style: "margin-left: 8px;",
                        onclick: {
                            let text = share_text();
                            move |_| {
                                if let Some(win) = web_sys::window() {
                                    let _ = win.navigator().clipboard().write_text(&text);
                                }
                            }
                        },
                        "📋 分享结果"
                    }
                }
            }

            if *loading.read() {
                div { class: "pg-arena-loading g-card",
                    span { class: "g-spinner" }
                    p { "正在读取对局记录..." }
                }
            } else if let Some(ref r) = *room_data.read() {
                div { class: "pg-replay-detail g-card",
                    // Header Meta
                    div { class: "pg-replay-meta-header",
                        div { class: "pg-replay-meta-badge",
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

                    // Play Again
                    if play_again_data.read().is_some() {
                        div { class: "pg-replay-actions g-card-subtle",
                            button {
                                class: "pg-replay-play-again",
                                onclick: handle_play_again,
                                "🔄 再来一局（相同配置）"
                            }
                        }
                    }

                    // Body
                    div { class: "pg-replay-body",
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
                div { class: "pg-replay-error g-card",
                    p { "未能加载到该对局数据。" }
                }
            }
        }
    }
}

#[component]
fn LincolnReplayView(engine_state: Value) -> Element {
    let state = use_memo(move || serde_json::from_value::<LincolnState>(engine_state.clone()).ok());

    let mut show_ai_content = use_signal(|| true);

    rsx! {
        div { class: "lincoln-replay-view",
            if let Some(ref s) = *state.read() {
                div { class: "lincoln-replay-inner",
                    div { class: "pg-replay-round",
                        span { "🏛️ 林肯辩论历史辩词 (共 {s.round} 轮)" }
                        button {
                            class: "g-card-subtle gm-ai-toggle",
                            style: "margin-left: 15px; font-size: 0.85em; padding: 4px 12px; cursor: pointer;",
                            onclick: move |_| {
                                let cur = *show_ai_content.read();
                                show_ai_content.set(!cur);
                            },
                            if *show_ai_content.read() { "👀 隐藏 AI 发言" } else { "🙈 显示 AI 发言" }
                        }
                    }
                    div { class: "timeline-scroll pg-replay-timeline",
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
        div { key: "{entry.actor_id}:{entry.content.len()}", class: "gm-timeline-item",
            div { class: "gm-timeline-avatar {role_cls}", "{icon}" }
            div { class: "gm-timeline-body",
                div { class: "gm-timeline-meta",
                    span { class: "gm-timeline-author", "{entry.actor_id}" }
                    span { class: "gm-timeline-tag {role_cls}", "{label}" }
                }
                div { class: "gm-timeline-content {role_cls}",
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
    let pot = engine_state
        .get("pot")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let phase = engine_state
        .get("phase")
        .and_then(|v| v.as_str())
        .unwrap_or("Unknown");

    let es_for_memo = engine_state.clone();
    let es_for_showdown = engine_state.clone();

    // Community cards
    let community_cards_list = use_memo(move || {
        let mut list = Vec::new();
        if let Some(cards) = es_for_memo
            .get("community_cards")
            .and_then(|c| c.as_array())
        {
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
                div { class: "pg-replay-stat g-card-subtle",
                    div { class: "stat-val", "💰 {pot}" }
                    div { class: "stat-lbl", "总奖池" }
                }
                div { class: "pg-replay-stat g-card-subtle",
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
    let card_color = if suit == "Hearts" || suit == "Diamonds" {
        "card-red"
    } else {
        "card-black"
    };
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
    let winner = res
        .get("is_winner")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    rsx! {
        div { class: if winner { "pg-replay-showdown winner g-card-subtle" } else { "pg-replay-showdown" },
            span { class: "showdown-winner-icon", if winner { "👑" } else { "👤" } }
            span { class: "showdown-player", "{p_id}" }
            span { class: "showdown-rank", "{rank_desc}" }
            if winner {
                span { class: "winner-tag", "获胜者" }
            }
        }
    }
}
