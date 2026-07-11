use dioxus::prelude::*;
use serde::Deserialize;
use serde_json::Value;
use tracing::{debug, warn};

use super::GamePluginProps;

// ═══════════════════════════════════════════════════════
//  强类型结构
// ═══════════════════════════════════════════════════════

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub enum Suit {
    Hearts,
    Diamonds,
    Clubs,
    Spades,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub enum Rank {
    Two,
    Three,
    Four,
    Five,
    Six,
    Seven,
    Eight,
    Nine,
    Ten,
    Jack,
    Queen,
    King,
    Ace,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct Card {
    pub suit: Suit,
    pub rank: Rank,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PlayerView {
    pub id: String,
    pub kind: String,
    pub position: String,
    pub chips: u32,
    pub hand_count: usize,
    pub current_bet: u32,
    pub total_bet: u32,
    pub folded: bool,
    pub all_in: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ShowdownResultView {
    pub player_id: String,
    pub hand: Vec<Card>,
    pub hand_rank: String,
    pub is_winner: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SpectatorHand {
    pub player_id: String,
    pub hand: Vec<Card>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HistoryEntryView {
    pub actor_id: String,
    pub action_desc: String,
    pub phase: String,
    pub ai_content: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TexasHoldemViewState {
    pub game_type: String,
    pub room_id: String,
    pub phase: String,
    pub pot: u32,
    pub current_bet: u32,
    pub community_cards: Vec<Card>,
    pub players: Vec<PlayerView>,
    pub active_player: Option<String>,
    pub dealer_index: usize,
    pub small_blind: u32,
    pub big_blind: u32,
    pub finished: bool,
    pub your_hand: Vec<Card>,
    pub showdown_results: Vec<ShowdownResultView>,
    pub spectator_hands: Vec<SpectatorHand>,
    pub history: Vec<HistoryEntryView>,
}

// ═══════════════════════════════════════════════════════
//  解析函数
// ═══════════════════════════════════════════════════════

fn parse_card(v: &Value) -> Option<Card> {
    let suit = match v.get("suit")?.as_str()? {
        "Hearts" => Suit::Hearts,
        "Diamonds" => Suit::Diamonds,
        "Clubs" => Suit::Clubs,
        "Spades" => Suit::Spades,
        _ => return None,
    };
    let rank = match v.get("rank")?.as_str()? {
        "Two" => Rank::Two,
        "Three" => Rank::Three,
        "Four" => Rank::Four,
        "Five" => Rank::Five,
        "Six" => Rank::Six,
        "Seven" => Rank::Seven,
        "Eight" => Rank::Eight,
        "Nine" => Rank::Nine,
        "Ten" => Rank::Ten,
        "Jack" => Rank::Jack,
        "Queen" => Rank::Queen,
        "King" => Rank::King,
        "Ace" => Rank::Ace,
        _ => return None,
    };
    Some(Card { suit, rank })
}

fn parse_player(v: &Value) -> Option<PlayerView> {
    Some(PlayerView {
        id: v.get("id")?.as_str()?.to_string(),
        kind: v
            .get("kind")
            .and_then(|v| v.as_str())
            .unwrap_or("Human")
            .to_string(),
        position: v
            .get("position")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        chips: v.get("chips").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
        hand_count: v
            .get("hand")
            .and_then(|v| v.as_array())
            .map(|a| a.len())
            .unwrap_or(0),
        current_bet: v.get("current_bet").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
        total_bet: v.get("total_bet").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
        folded: v.get("folded").and_then(|v| v.as_bool()).unwrap_or(false),
        all_in: v.get("all_in").and_then(|v| v.as_bool()).unwrap_or(false),
    })
}

fn parse_showdown(v: &Value) -> Option<ShowdownResultView> {
    let hand = v
        .get("hand")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(parse_card).collect())
        .unwrap_or_default();
    Some(ShowdownResultView {
        player_id: v.get("player_id")?.as_str()?.to_string(),
        hand,
        hand_rank: v
            .get("hand_rank")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown")
            .to_string(),
        is_winner: v
            .get("is_winner")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
    })
}

fn parse_spectator_hand(v: &Value) -> Option<SpectatorHand> {
    let hand = v
        .get("hand")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(parse_card).collect())
        .unwrap_or_default();
    Some(SpectatorHand {
        player_id: v.get("player_id")?.as_str()?.to_string(),
        hand,
    })
}

fn parse_history_entry(v: &Value) -> Option<HistoryEntryView> {
    let actor_id = v.get("actor_id")?.as_str()?.to_string();
    let phase = v
        .get("phase")
        .and_then(|v| v.as_str())
        .unwrap_or("Unknown")
        .to_string();
    let ai_content = v
        .get("ai_content")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let action_val = v.get("action")?;
    let action_desc = if let Some(s) = action_val.as_str() {
        match s {
            "Fold" => "弃牌".to_string(),
            "Check" => "过牌".to_string(),
            "Call" => "跟注".to_string(),
            "AllIn" => "All-In".to_string(),
            _ => s.to_string(),
        }
    } else if let Some(obj) = action_val.as_object() {
        if let Some(amt) = obj.get("Raise").and_then(|v| v.as_u64()) {
            format!("加注到 {}", amt)
        } else {
            "未知操作".to_string()
        }
    } else {
        "未知操作".to_string()
    };

    Some(HistoryEntryView {
        actor_id,
        action_desc,
        phase,
        ai_content,
    })
}

fn parse_state(raw: &Value) -> Option<TexasHoldemViewState> {
    let game_type = raw.get("game_type")?.as_str()?.to_string();
    let room_id = raw
        .get("room_id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let phase = raw
        .get("phase")
        .and_then(|v| v.as_str())
        .unwrap_or("Unknown")
        .to_string();
    let pot = raw.get("pot").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
    let current_bet = raw.get("current_bet").and_then(|v| v.as_u64()).unwrap_or(0) as u32;

    let community_cards = raw
        .get("community_cards")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(parse_card).collect())
        .unwrap_or_default();

    let players = raw
        .get("players")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(parse_player).collect())
        .unwrap_or_default();

    let active_player = raw
        .get("active_player")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let dealer_index = raw
        .get("dealer_index")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as usize;
    let small_blind = raw.get("small_blind").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
    let big_blind = raw.get("big_blind").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
    let finished = raw
        .get("finished")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let your_hand = raw
        .get("your_hand")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(parse_card).collect())
        .unwrap_or_default();

    let showdown_results = raw
        .get("showdown_results")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(parse_showdown).collect())
        .unwrap_or_default();

    let spectator_hands = raw
        .get("spectator_hands")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(parse_spectator_hand).collect())
        .unwrap_or_default();

    let history = raw
        .get("history")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(parse_history_entry).collect())
        .unwrap_or_default();

    Some(TexasHoldemViewState {
        game_type,
        room_id,
        phase,
        pot,
        current_bet,
        community_cards,
        players,
        active_player,
        dealer_index,
        small_blind,
        big_blind,
        finished,
        your_hand,
        showdown_results,
        spectator_hands,
        history,
    })
}

// ═══════════════════════════════════════════════════════
//  辅助函数
// ═══════════════════════════════════════════════════════

impl Suit {
    pub fn symbol(&self) -> &str {
        match self {
            Suit::Hearts => "♥",
            Suit::Diamonds => "♦",
            Suit::Clubs => "♣",
            Suit::Spades => "♠",
        }
    }
    pub fn color_class(&self) -> &str {
        match self {
            Suit::Hearts | Suit::Diamonds => "red",
            Suit::Clubs | Suit::Spades => "black",
        }
    }
}

impl Rank {
    pub fn display(&self) -> &str {
        match self {
            Rank::Two => "2",
            Rank::Three => "3",
            Rank::Four => "4",
            Rank::Five => "5",
            Rank::Six => "6",
            Rank::Seven => "7",
            Rank::Eight => "8",
            Rank::Nine => "9",
            Rank::Ten => "10",
            Rank::Jack => "J",
            Rank::Queen => "Q",
            Rank::King => "K",
            Rank::Ace => "A",
        }
    }
}

fn phase_display(phase: &str) -> &str {
    match phase {
        "WaitingForPlayers" => "等待开始",
        "PreFlop" => "翻牌前",
        "Flop" => "翻牌",
        "Turn" => "转牌",
        "River" => "河牌",
        "Showdown" => "摊牌",
        "Finished" => "已结束",
        _ => phase,
    }
}

fn hand_rank_display(rank: &str) -> &str {
    match rank {
        "HighCard" => "高牌",
        "OnePair" => "一对",
        "TwoPair" => "两对",
        "ThreeOfAKind" => "三条",
        "Straight" => "顺子",
        "Flush" => "同花",
        "FullHouse" => "葫芦",
        "FourOfAKind" => "四条",
        "StraightFlush" => "同花顺",
        "RoyalFlush" => "皇家同花顺",
        _ => rank,
    }
}

// ═══════════════════════════════════════════════════════
//  主组件
// ═══════════════════════════════════════════════════════

#[component]
pub fn TexasHoldemGame(props: GamePluginProps) -> Element {
    let state = props.state;
    let on_action = props.on_action;
    let my_actor_id = props.actor_id.clone();

    let texas = use_memo(move || {
        let raw: Value = state();
        if raw == Value::Null {
            return None;
        }
        match parse_state(&raw) {
            Some(s) => {
                debug!(target: "texas_holdem", phase = %s.phase, players = s.players.len(), "状态解析成功");
                Some(s)
            }
            None => {
                warn!(target: "texas_holdem", raw = %raw, "状态解析失败");
                None
            }
        }
    });

    // 检查是否是观察者
    let is_spectator = my_actor_id == "spectator" || my_actor_id.starts_with("human_spectator");

    // 获取当前状态（克隆为 owned 值，避免借用问题）
    let state_data = texas().clone();

    rsx! {
        div { class: "poker-game-container",
            if let Some(s) = state_data.as_ref() {
                {
                    let s = s.clone();
                    rsx! { GameStateView { state: s, on_action, my_actor_id: my_actor_id.clone(), is_spectator } }
                }
            } else {
                div { class: "loading-screen",
                    div { class: "loading-g-spinner" }
                    div { class: "loading-text", "正在连接牌桌..." }
                }
            }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
struct GameStateViewProps {
    state: TexasHoldemViewState,
    on_action: Callback<Value>,
    my_actor_id: String,
    is_spectator: bool,
}

#[component]
fn GameStateView(props: GameStateViewProps) -> Element {
    let s = props.state;
    let on_action = props.on_action;
    let my_actor_id = props.my_actor_id.clone();
    let is_spectator = props.is_spectator;

    let is_my_turn = s.active_player.as_deref() == Some(my_actor_id.as_str());
    let my_player = s.players.iter().find(|p| p.id == my_actor_id).cloned();

    let mut raise_amount = use_signal(|| "0".to_string());
    let mut show_ai_content = use_signal(|| false);

    rsx! {
        // ── 顶部信息栏 ──
        div { class: "poker-top-bar",
            div { class: "game-info",
                span { class: "game-title", "德州扑克" }
                span { class: "separator", "|" }
                span { class: "phase-text", "{phase_display(&s.phase)}" }
            }
            div { class: "blind-info",
                "盲注: {s.small_blind}/{s.big_blind}"
            }
        }

        // ── 牌桌区域 ──
        div { class: "poker-table-wrapper",
            div { class: "poker-table",
                // 底池
                div { class: "poker-pot",
                    div { class: "pot-chips-icon", "💰" }
                    div { class: "pot-amount", "{s.pot}" }
                }

                // 公共牌
                div { class: "poker-community",
                    for (idx, card) in s.community_cards.iter().enumerate() {
                        PokerCard {
                            key: "{idx}",
                            card: card.clone(),
                            size: "large".to_string(),
                        }
                    }
                    for idx in s.community_cards.len()..5 {
                        div { class: "card-slot empty", key: "ph-{idx}" }
                    }
                }

                // 玩家座位（围绕牌桌）
                for (idx, player) in s.players.iter().enumerate() {
                    {
                        let is_me = player.id == my_actor_id;
                        let is_dealer = idx == s.dealer_index;
                        let is_active = s.active_player.as_deref() == Some(&player.id);
                        let seat_class = format!("poker-seat seat-{}{}{}{}",
                            idx,
                            if is_active { " active" } else { "" },
                            if player.folded { " folded" } else { "" },
                            if player.all_in { " all-in" } else { "" },
                        );
                        let player_name = if is_me {
                            format!("你 ({})", player.id)
                        } else {
                            player.id.clone()
                        };
                        // let my_hand = if is_me { s.your_hand.clone() } else { Vec::new() };
                        let display_hand = if is_me {
                            s.your_hand.clone()
                        } else if is_spectator {
                        // 在 spectator_hands 中查找当前玩家的牌
                            s.spectator_hands
                            .iter()
                            .find(|sh| sh.player_id == player.id)
                            .map(|sh| sh.hand.clone())
                            .unwrap_or_default()
                        } else {
                            Vec::new()
                        };

                        rsx! {
                            div {
                                class: "{seat_class}",
                                key: "{player.id}",

                                if is_dealer {
                                    div { class: "dealer-button", "D" }
                                }

                                if !player.position.is_empty() {
                                    div { class: "position-tag", "{player.position}" }
                                }

                                div { class: "poker-player-info",
                                    div { class: "avatar",
                                        if player.kind == "Ai" { "🤖" } else { "👤" }
                                    }
                                    div { class: "poker-player-name", "{player_name}" }
                                }

                                div { class: "poker-player-chips",
                                    "💰 {player.chips}"
                                }

                                if player.current_bet > 0 {
                                    div { class: "poker-player-bet",
                                        div { class: "bet-chip", "🪙" }
                                        div { class: "bet-amount", "{player.current_bet}" }
                                    }
                                }

                                if !display_hand.is_empty() {
                                    div { class: "poker-player-hand",
                                        for (idx2, card) in display_hand.iter().enumerate() {
                                            PokerCard {
                                                key: "{player.id}-{idx2}",
                                                card: card.clone(),
                                                size: "small".to_string(),
                                            }
                                        }
                                    }
                                }

                                if player.folded {
                                    div { class: "poker-player-status folded", "弃牌" }
                                } else if player.all_in {
                                    div { class: "poker-player-status allin", "ALL IN" }
                                }

                                if is_active {
                                    div { class: "active-indicator",
                                        div { class: "active-dot" }
                                        div { class: "active-text", "思考中..." }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // ── 操作区域 ──
        div { class: "poker-actions",
            if s.phase == "WaitingForPlayers" {
                div { class: "waiting-start",
                    div { class: "waiting-icon", "🎮" }
                    div { class: "waiting-text", "准备开始游戏" }
                    button {
                        class: "btn-start-game",
                        onclick: move |_| {
                            on_action.call(serde_json::json!({"action": "start"}));
                        },
                        "开始游戏"
                    }
                }
            } else if s.finished {
                div { class: "game-over-panel",
                    div { class: "game-over-icon", "🏆" }
                    div { class: "game-over-text", "游戏结束" }
                    if !s.showdown_results.is_empty() {
                        div { class: "winner-info",
                            for result in s.showdown_results.iter().filter(|r| r.is_winner) {
                                div { class: "winner-name", key: "{result.player_id}",
                                    "🏆 {result.player_id} 获胜!"
                                }
                            }
                        }
                    }
                    button {
                        class: "btn-new-game",
                        onclick: move |_| {
                            on_action.call(serde_json::json!({"action": "start"}));
                        },
                        "再来一局"
                    }
                }
            } else if is_my_turn {
                div { class: "my-turn-panel",
                    div { class: "turn-header",
                        div { class: "turn-icon", "🎯" }
                        div { class: "turn-text", "轮到你行动" }
                    }

                    div { class: "poker-actions-row",
                        button {
                            class: "poker-btn poker-btn-fold",
                            onclick: move |_| {
                                on_action.call(serde_json::json!({"action": "fold"}));
                            },
                            "弃牌"
                        }

                        {
                            let can_check = my_player.as_ref().map_or(false, |p| p.current_bet >= s.current_bet);
                            let need_call = s.current_bet > my_player.as_ref().map_or(0, |p| p.current_bet);
                            let call_amount = s.current_bet.saturating_sub(my_player.as_ref().map_or(0, |p| p.current_bet));

                            if can_check {
                                rsx! {
                                    button {
                                        class: "poker-btn poker-btn-check",
                                        onclick: move |_| {
                                            on_action.call(serde_json::json!({"action": "check"}));
                                        },
                                        "过牌"
                                    }
                                }
                            } else if need_call {
                                rsx! {
                                    button {
                                        class: "poker-btn poker-btn-call",
                                        onclick: move |_| {
                                            on_action.call(serde_json::json!({"action": "call"}));
                                        },
                                        "跟注 {call_amount}"
                                    }
                                }
                            } else {
                                rsx! { div {} }
                            }
                        }

                        div { class: "raise-group",
                            input {
                                class: "raise-input",
                                r#type: "number",
                                value: "{raise_amount}",
                                oninput: move |e| raise_amount.set(e.value()),
                                placeholder: "金额",
                            }
                            {
                                let current_bet = s.current_bet;
                                let on_action_clone = on_action;
                                rsx! {
                                    button {
                                        class: "poker-btn poker-btn-raise",
                                        onclick: move |_| {
                                            let amount = raise_amount.read().parse::<u32>().unwrap_or(0);
                                            if amount > current_bet {
                                                on_action_clone.call(serde_json::json!({"action": "raise", "amount": amount}));
                                            }
                                        },
                                        "加注"
                                    }
                                }
                            }
                        }

                        button {
                            class: "poker-btn poker-btn-allin",
                            onclick: move |_| {
                                on_action.call(serde_json::json!({"action": "all_in"}));
                            },
                            "ALL IN"
                        }
                    }
                }
            } else if is_spectator {
                div { class: "spectator-panel",
                    div { class: "spectator-icon", "👀" }
                    div { class: "spectator-text", "观战模式" }
                    div { class: "spectator-hint",
                        if let Some(ref ap) = s.active_player {
                            "等待 {ap} 行动..."
                        } else {
                            "等待游戏继续..."
                        }
                    }
                }
            } else {
                div { class: "waiting-panel",
                    div { class: "waiting-g-spinner" }
                    div { class: "waiting-text",
                        if let Some(ref ap) = s.active_player {
                            "等待 {ap} 行动..."
                        } else {
                            "等待中..."
                        }
                    }
                }
            }
        }

        // ── 摊牌结果 ──
        if (s.phase == "Showdown" || s.phase == "Finished") && !s.showdown_results.is_empty() {
            div { class: "showdown-panel",
                div { class: "showdown-title", "摊牌结果" }
                div { class: "showdown-cards",
                    for result in s.showdown_results.iter() {
                        div {
                            class: if result.is_winner { "showdown-player winner" } else { "showdown-player" },
                            key: "{result.player_id}",
                            div { class: "showdown-name", "{result.player_id}" }
                            div { class: "showdown-hand",
                                for (idx, card) in result.hand.iter().enumerate() {
                                    PokerCard {
                                        key: "sd-{result.player_id}-{idx}",
                                        card: card.clone(),
                                        size: "tiny".to_string(),
                                    }
                                }
                            }
                            div { class: "showdown-rank", "{hand_rank_display(&result.hand_rank)}" }
                            if result.is_winner {
                                div { class: "showdown-winner-badge", "🏆" }
                            }
                        }
                    }
                }
            }
        }

        // ── 历史流水 ──
        div { class: "history-panel g-card poker-history-panel",
            div { class: "history-header poker-history-header",
                h4 { "📜 历史流水记录" }
                button {
                    class: "g-card-subtle gm-ai-toggle poker-history-toggle",
                    onclick: move |_| {
                        let cur = *show_ai_content.read();
                        show_ai_content.set(!cur);
                    },
                    if *show_ai_content.read() { "👀 隐藏 AI 心声" } else { "🙈 显示 AI 心声" }
                }
            }
            div { class: "history-list poker-history-list",
                if s.history.is_empty() {
                    div { class: "poker-history-empty", "暂无记录" }
                } else {
                    for entry in s.history.iter().rev() {
                        div { class: "history-item poker-history-item",
                            div { class: "history-action",
                                span { class: "poker-history-actor", "{entry.actor_id}" }
                                span { class: "poker-history-desc", "[{phase_display(&entry.phase)}] {entry.action_desc}" }
                            }
                            if *show_ai_content.read() {
                                if let Some(content) = &entry.ai_content {
                                    div { class: "history-ai-content poker-history-ai-card",
                                        "🤖: {content}"
                                    }
                                } else if s.players.iter().any(|p| p.id == entry.actor_id && p.kind == "Ai") {
                                    div { class: "history-ai-content poker-history-ai-hint",
                                        "[由于后端未下发或无记录，无法显示AI心声]"
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
//  扑克牌组件
// ═══════════════════════════════════════════════════════

#[derive(Props, Clone, PartialEq)]
struct PokerCardProps {
    card: Card,
    size: String,
}

#[component]
fn PokerCard(props: PokerCardProps) -> Element {
    let card = &props.card;
    let size_class = match props.size.as_str() {
        "large" => "card-large",
        "small" => "card-small",
        "tiny" => "card-tiny",
        _ => "card-medium",
    };
    let color_class = card.suit.color_class();

    rsx! {
        div { class: "poker-card {size_class} {color_class}",
            div { class: "card-corner top-left",
                div { class: "card-rank", "{card.rank.display()}" }
                div { class: "card-suit", "{card.suit.symbol()}" }
            }
            div { class: "card-center",
                "{card.suit.symbol()}"
            }
            div { class: "card-corner bottom-right",
                div { class: "card-rank", "{card.rank.display()}" }
                div { class: "card-suit", "{card.suit.symbol()}" }
            }
        }
    }
}

const PLAYER_COUNT_OPTIONS: &[usize] = &[2, 3, 4, 5, 6];

pub fn TexasHoldemLobbyCard(props: crate::games::registry::GameConfigProps) -> Element {
    let mut role_config = props.role_config;
    let mut my_role = props.my_role;
    let mut max_round = props.max_round;
    let mut game_config = props.game_config;

    let mut small_blind = use_signal(|| "10".to_string());
    let mut big_blind = use_signal(|| "20".to_string());
    let mut starting_chips = use_signal(|| "1000".to_string());
    let mut player_count = use_signal(|| 6_usize);
    let mut spectator_mode = use_signal(|| false);

    // Initializer: run once when mounting if state not yet configured
    use_effect(move || {
        let current_role = my_role.read().clone();
        if current_role.is_empty()
            || (!current_role.starts_with("player") && current_role != "spectator")
        {
            let sb = small_blind.read().parse::<u32>().unwrap_or(10);
            let bb = big_blind.read().parse::<u32>().unwrap_or(20);
            let sc = starting_chips.read().parse::<u32>().unwrap_or(1000);

            game_config.set(Some(serde_json::json!({
                "small_blind": sb,
                "big_blind": bb,
                "starting_chips": sc,
            })));

            let count = *player_count.read();
            let mut modes = std::collections::HashMap::new();
            modes.insert("player1".to_string(), "human".to_string());
            for i in 2..=count {
                modes.insert(format!("player{}", i), "ai".to_string());
            }
            my_role.set("player1".to_string());
            role_config.set(modes);
            max_round.set(100);
        }
    });

    rsx! {
        div { class: "g-field",
            label { "游戏人数" }
            div { class: "player-count-grid",
                for count_opt in PLAYER_COUNT_OPTIONS.iter() {
                    {
                        let c = *count_opt;
                        let is_selected = *player_count.read() == c;
                        rsx! {
                            button {
                                class: if is_selected { "count-btn selected" } else { "count-btn" },
                                onclick: move |_| {
                                    player_count.set(c);
                                    let is_spec = *spectator_mode.read();
                                    let mut modes = std::collections::HashMap::new();
                                    if is_spec {
                                        for i in 1..=c {
                                            modes.insert(format!("player{}", i), "ai".to_string());
                                        }
                                        my_role.set("spectator".to_string());
                                    } else {
                                        modes.insert("player1".to_string(), "human".to_string());
                                        for i in 2..=c {
                                            modes.insert(format!("player{}", i), "ai".to_string());
                                        }
                                        my_role.set("player1".to_string());
                                    }
                                    role_config.set(modes);
                                },
                                "{c} 人"
                            }
                        }
                    }
                }
            }
        }

        div { class: "g-field",
            label { "游戏模式" }
            div { class: "mode-toggle",
                button {
                    class: if !*spectator_mode.read() { "mode-btn selected" } else { "mode-btn" },
                    onclick: move |_| {
                        spectator_mode.set(false);
                        let count = *player_count.read();
                        let mut modes = std::collections::HashMap::new();
                        modes.insert("player1".to_string(), "human".to_string());
                        for i in 2..=count {
                            // 保留之前的设置，如果之前就是 human 就保留
                            let prev = role_config.read().get(&format!("player{}", i)).cloned().unwrap_or_else(|| "ai".to_string());
                            modes.insert(format!("player{}", i), prev);
                        }
                        my_role.set("player1".to_string());
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
                        let count = *player_count.read();
                        let mut modes = std::collections::HashMap::new();
                        for i in 1..=count {
                            modes.insert(format!("player{}", i), "ai".to_string());
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
                    for i in 2..=*player_count.read() {
                        {
                            let slot_name = format!("player{}", i);
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
            label { "德州扑克配置" }
            div { class: "texas-config",
                div { class: "config-field",
                    label { "小盲注" }
                    input {
                        r#type: "number",
                        value: "{small_blind}",
                        oninput: move |e| {
                            let val = e.value();
                            small_blind.set(val.clone());
                            let sb = val.parse::<u32>().unwrap_or(10);
                            let bb = big_blind.read().parse::<u32>().unwrap_or(20);
                            let sc = starting_chips.read().parse::<u32>().unwrap_or(1000);
                            game_config.set(Some(serde_json::json!({
                                "small_blind": sb,
                                "big_blind": bb,
                                "starting_chips": sc,
                            })));
                        },
                    }
                }
                div { class: "config-field",
                    label { "大盲注" }
                    input {
                        r#type: "number",
                        value: "{big_blind}",
                        oninput: move |e| {
                            let val = e.value();
                            big_blind.set(val.clone());
                            let sb = small_blind.read().parse::<u32>().unwrap_or(10);
                            let bb = val.parse::<u32>().unwrap_or(20);
                            let sc = starting_chips.read().parse::<u32>().unwrap_or(1000);
                            game_config.set(Some(serde_json::json!({
                                "small_blind": sb,
                                "big_blind": bb,
                                "starting_chips": sc,
                            })));
                        },
                    }
                }
            }

            div { class: "config-field",
                label { "起始筹码" }
                input {
                    r#type: "number",
                    value: "{starting_chips}",
                    oninput: move |e| {
                        let val = e.value();
                        starting_chips.set(val.clone());
                        let sb = small_blind.read().parse::<u32>().unwrap_or(10);
                        let bb = big_blind.read().parse::<u32>().unwrap_or(20);
                        let sc = val.parse::<u32>().unwrap_or(1000);
                        game_config.set(Some(serde_json::json!({
                            "small_blind": sb,
                            "big_blind": bb,
                            "starting_chips": sc,
                        })));
                    },
                }
            }
        }
    }
}
