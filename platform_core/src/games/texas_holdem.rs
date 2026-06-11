use std::fmt::Debug;

use serde::{Deserialize, Serialize};

use crate::traits::{ActionKind, EngineEvent, GameEngine};

/// 扑克花色
#[derive(Clone, PartialEq, Eq, Debug, Copy, Serialize, Deserialize)]
pub enum Suit {
    Hearts,   // ♥
    Diamonds, // ♦
    Clubs,    // ♣
    Spades,   // ♠
}

/// 扑克点数
#[derive(Clone, PartialEq, Eq, Debug, Copy, Serialize, Deserialize)]
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

impl Rank {
    /// 获取点数的数值大小，用于比较
    pub fn value(&self) -> u8 {
        match self {
            Rank::Two => 2,
            Rank::Three => 3,
            Rank::Four => 4,
            Rank::Five => 5,
            Rank::Six => 6,
            Rank::Seven => 7,
            Rank::Eight => 8,
            Rank::Nine => 9,
            Rank::Ten => 10,
            Rank::Jack => 11,
            Rank::Queen => 12,
            Rank::King => 13,
            Rank::Ace => 14,
        }
    }
}

/// 扑克牌
#[derive(Clone, PartialEq, Eq, Debug, Copy, Serialize, Deserialize)]
pub struct Card {
    pub suit: Suit,
    pub rank: Rank,
}

impl Card {
    pub fn new(suit: Suit, rank: Rank) -> Self {
        Self { suit, rank }
    }
}

/// 牌型排名（从低到高）
#[derive(Clone, PartialEq, Eq, Debug, Copy, Serialize, Deserialize, PartialOrd, Ord)]
pub enum HandRankCategory {
    HighCard,
    OnePair,
    TwoPair,
    ThreeOfAKind,
    Straight,
    Flush,
    FullHouse,
    FourOfAKind,
    StraightFlush,
    RoyalFlush,
}

/// 牌型评估结果
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct HandEvaluation {
    pub category: HandRankCategory,
    pub kickers: Vec<u8>,
}

impl PartialOrd for HandEvaluation {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for HandEvaluation {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match self.category.cmp(&other.category) {
            std::cmp::Ordering::Equal => self.kickers.cmp(&other.kickers),
            ord => ord,
        }
    }
}

/// 游戏阶段
#[derive(Clone, PartialEq, Eq, Debug, Copy, Serialize, Deserialize)]
pub enum GamePhase {
    WaitingForPlayers,
    PreFlop,
    Flop,
    Turn,
    River,
    Showdown,
    Finished,
}

/// 玩家动作
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum PlayerAction {
    Fold,
    Check,
    Call,
    Raise(u32),
    AllIn,
}

/// 玩家状态
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PokerPlayer {
    pub id: String,
    pub kind: String,
    pub chips: u32,
    pub hand: Vec<Card>,
    pub current_bet: u32,
    pub total_bet: u32,
    pub folded: bool,
    pub all_in: bool,
    /// 本轮是否已行动过（用于判断下注轮是否结束）
    #[serde(default)]
    pub acted_this_round: bool,
}

/// 历史记录条目
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ActionHistory {
    pub actor_id: String,
    pub action: PlayerAction,
    pub phase: GamePhase,
    pub chips_after: u32,
}

/// 摊牌结果条目
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ShowdownResult {
    pub player_id: String,
    pub hand: Vec<Card>,
    pub hand_rank: HandRankCategory,
    pub is_winner: bool,
}

/// 德州扑克引擎
pub struct TexasHoldemEngine {
    pub room_id: String,
    pub players: Vec<PokerPlayer>,
    pub deck: Vec<Card>,
    pub community_cards: Vec<Card>,
    pub pot: u32,
    pub current_bet: u32,
    pub phase: GamePhase,
    pub dealer_index: usize,
    pub active_index: usize,
    pub small_blind: u32,
    pub big_blind: u32,
    pub history: Vec<ActionHistory>,
    pub finished: bool,
    pub round_reset_bets: bool,
    pub showdown_results: Vec<ShowdownResult>,
}

impl TexasHoldemEngine {
    /// 创建新的德州扑克引擎
    pub fn new(room_id: String, small_blind: u32, big_blind: u32) -> Self {
        Self {
            room_id,
            players: Vec::new(),
            deck: Vec::new(),
            community_cards: Vec::new(),
            pot: 0,
            current_bet: 0,
            phase: GamePhase::WaitingForPlayers,
            dealer_index: 0,
            active_index: 0,
            small_blind,
            big_blind,
            history: Vec::new(),
            finished: false,
            round_reset_bets: false,
            showdown_results: Vec::new(),
        }
    }

    /// 添加玩家
    pub fn add_player(&mut self, id: String, kind: ActionKind, chips: u32) {
        self.players.push(PokerPlayer {
            id,
            kind: match kind {
                ActionKind::Ai => "Ai".to_string(),
                ActionKind::Human => "Human".to_string(),
            },
            chips,
            hand: Vec::new(),
            current_bet: 0,
            total_bet: 0,
            folded: false,
            all_in: false,
            acted_this_round: false,
        });
    }

    /// 生成并洗牌
    fn init_deck(&mut self) {
        self.deck.clear();
        let suits = [Suit::Hearts, Suit::Diamonds, Suit::Clubs, Suit::Spades];
        let ranks = [
            Rank::Two,
            Rank::Three,
            Rank::Four,
            Rank::Five,
            Rank::Six,
            Rank::Seven,
            Rank::Eight,
            Rank::Nine,
            Rank::Ten,
            Rank::Jack,
            Rank::Queen,
            Rank::King,
            Rank::Ace,
        ];
        for suit in &suits {
            for rank in &ranks {
                self.deck.push(Card::new(*suit, *rank));
            }
        }
        // Fisher-Yates 洗牌
        for i in (1..self.deck.len()).rev() {
            let j = random_index(i + 1);
            self.deck.swap(i, j);
        }
    }

    /// 发牌给玩家
    fn deal_hands(&mut self) {
        for player in &mut self.players {
            player.hand.clear();
            if let Some(card) = self.deck.pop() {
                player.hand.push(card);
            }
            if let Some(card) = self.deck.pop() {
                player.hand.push(card);
            }
        }
    }

    /// 发公共牌
    fn deal_community(&mut self, count: usize) {
        // 烧牌
        self.deck.pop();
        for _ in 0..count {
            if let Some(card) = self.deck.pop() {
                self.community_cards.push(card);
            }
        }
    }

    /// 计算每个玩家的位置标签（BTN/SB/BB/UTG/MP/CO 等）
    fn calculate_positions(&self) -> Vec<String> {
        let n = self.players.len();
        if n == 0 {
            return vec![];
        }
        let mut positions = vec![String::new(); n];

        // 庄位
        positions[self.dealer_index] = "BTN".to_string();

        if n == 2 {
            // 单挑：BTN = SB，另一人 = BB
            let other = (self.dealer_index + 1) % n;
            positions[self.dealer_index] = "BTN/SB".to_string();
            positions[other] = "BB".to_string();
        } else {
            // 多人：SB = 庄家左手第一位，BB = 左手第二位
            let sb_index = (self.dealer_index + 1) % n;
            let bb_index = (self.dealer_index + 2) % n;
            positions[sb_index] = "SB".to_string();
            positions[bb_index] = "BB".to_string();

            // 其余位置按顺序标注
            let pos_labels = if n <= 6 {
                vec!["UTG", "MP", "CO", "BTN", "SB", "BB"]
            } else {
                vec!["UTG", "UTG+1", "MP", "MP+1", "CO", "BTN", "SB", "BB"]
            };
            // 从 UTG（BB 下一位）开始，逆时针标注
            let start = (bb_index + 1) % n;
            let mut label_idx = 0;
            for i in 0..n {
                let idx = (start + i) % n;
                if positions[idx].is_empty() {
                    positions[idx] = pos_labels.get(label_idx).unwrap_or(&"").to_string();
                    label_idx += 1;
                }
            }
        }

        positions
    }

    /// 获取当前活跃（未弃牌且未 all-in）的玩家数量
    fn active_players_count(&self) -> usize {
        self.players
            .iter()
            .filter(|p| !p.folded && !p.all_in)
            .count()
    }

    /// 获取未弃牌的玩家数量
    fn non_folded_count(&self) -> usize {
        self.players.iter().filter(|p| !p.folded).count()
    }

    /// 获取下一位需要行动的玩家索引
    fn next_active_player(&self, from: usize) -> Option<usize> {
        let len = self.players.len();
        for i in 1..=len {
            let idx = (from + i) % len;
            if !self.players[idx].folded && !self.players[idx].all_in {
                return Some(idx);
            }
        }
        None
    }

    /// 检查是否所有活跃玩家的下注都相等
    fn all_bets_equal(&self) -> bool {
        let active_bets: Vec<u32> = self
            .players
            .iter()
            .filter(|p| !p.folded && !p.all_in)
            .map(|p| p.current_bet)
            .collect();
        if active_bets.is_empty() {
            return true;
        }
        active_bets.windows(2).all(|w| w[0] == w[1])
    }

    /// 检查是否所有活跃（未弃牌、未all-in）玩家都已行动过
    fn all_active_players_acted(&self) -> bool {
        self.players
            .iter()
            .filter(|p| !p.folded && !p.all_in)
            .all(|p| p.acted_this_round)
    }

    /// 检查下注轮是否结束（所有活跃玩家都已行动且下注相等）
    fn is_betting_round_complete(&self) -> bool {
        let active_count = self.active_players_count();
        if active_count == 0 {
            return true;
        }
        self.all_active_players_acted() && self.all_bets_equal()
    }

    /// 重置当前轮的下注
    fn reset_round_bets(&mut self) {
        for player in &mut self.players {
            player.current_bet = 0;
        }
        self.current_bet = 0;
    }

    /// 收集下注到奖池
    fn collect_bets(&mut self) {
        for player in &mut self.players {
            self.pot += player.current_bet;
            player.current_bet = 0;
        }
    }

    /// 开始新的一手牌
    fn start_new_hand(&mut self) {
        // 重置玩家状态
        for player in &mut self.players {
            player.hand.clear();
            player.current_bet = 0;
            player.total_bet = 0;
            player.folded = false;
            player.all_in = false;
            player.acted_this_round = false;
        }

        self.community_cards.clear();
        self.pot = 0;
        self.current_bet = 0;
        self.round_reset_bets = false;
        self.showdown_results.clear();
        self.history.clear();
        self.active_index = 0;

        // 检查是否有足够玩家
        let alive: Vec<usize> = self
            .players
            .iter()
            .enumerate()
            .filter(|(_, p)| p.chips > 0)
            .map(|(i, _)| i)
            .collect();

        if alive.len() < 2 {
            self.phase = GamePhase::Finished;
            self.finished = true;
            return;
        }

        // 移动庄家位
        loop {
            self.dealer_index = (self.dealer_index + 1) % self.players.len();
            if self.players[self.dealer_index].chips > 0 {
                break;
            }
        }

        // 初始化牌堆
        self.init_deck();
        self.deal_hands();

        // 下盲注
        let sb_index = self
            .next_active_player(self.dealer_index)
            .unwrap_or(self.dealer_index);
        let bb_index = self.next_active_player(sb_index).unwrap_or(sb_index);

        let sb_amount = self.small_blind.min(self.players[sb_index].chips);
        self.players[sb_index].chips -= sb_amount;
        self.players[sb_index].current_bet = sb_amount;
        self.players[sb_index].total_bet += sb_amount;
        if self.players[sb_index].chips == 0 {
            self.players[sb_index].all_in = true;
        }

        let bb_amount = self.big_blind.min(self.players[bb_index].chips);
        self.players[bb_index].chips -= bb_amount;
        self.players[bb_index].current_bet = bb_amount;
        self.players[bb_index].total_bet += bb_amount;
        if self.players[bb_index].chips == 0 {
            self.players[bb_index].all_in = true;
        }

        self.current_bet = bb_amount;
        self.pot = sb_amount + bb_amount;

        // Pre-flop: 从大盲注后一位开始行动
        self.active_index = self.next_active_player(bb_index).unwrap_or(bb_index);
        self.phase = GamePhase::PreFlop;
    }

    /// 推进到下一阶段
    fn advance_phase(&mut self) {
        self.collect_bets();
        self.reset_round_bets();

        // 重置所有玩家的行动标记
        for player in &mut self.players {
            player.acted_this_round = false;
        }

        match self.phase {
            GamePhase::PreFlop => {
                self.deal_community(3);
                self.phase = GamePhase::Flop;
            }
            GamePhase::Flop => {
                self.deal_community(1);
                self.phase = GamePhase::Turn;
            }
            GamePhase::Turn => {
                self.deal_community(1);
                self.phase = GamePhase::River;
            }
            GamePhase::River => {
                self.phase = GamePhase::Showdown;
            }
            _ => {}
        }

        // 从庄家下一位开始行动
        if let Some(next) = self.next_active_player(self.dealer_index) {
            self.active_index = next;
        }
    }

    /// 评估牌型
    fn evaluate_hand(&self, cards: &[Card]) -> HandEvaluation {
        if cards.len() < 5 {
            return HandEvaluation {
                category: HandRankCategory::HighCard,
                kickers: cards.iter().map(|c| c.rank.value()).collect(),
            };
        }

        // 从7张牌中选择最佳5张组合
        let mut best: Option<HandEvaluation> = None;

        for i in 0..cards.len() {
            for j in (i + 1)..cards.len() {
                let five_cards: Vec<Card> = cards
                    .iter()
                    .enumerate()
                    .filter(|(idx, _)| *idx != i && *idx != j)
                    .map(|(_, c)| *c)
                    .collect();
                let eval = self.evaluate_five_cards(&five_cards);
                if best.is_none() || eval > *best.as_ref().unwrap() {
                    best = Some(eval);
                }
            }
        }

        best.unwrap_or(HandEvaluation {
            category: HandRankCategory::HighCard,
            kickers: vec![0],
        })
    }

    /// 评估5张牌的牌型
    pub fn evaluate_five_cards(&self, cards: &[Card]) -> HandEvaluation {
        let mut ranks: Vec<u8> = cards.iter().map(|c| c.rank.value()).collect();
        ranks.sort_by(|a, b| b.cmp(a));

        let is_flush = cards.windows(2).all(|w| w[0].suit == w[1].suit);
        let is_straight = self.check_straight(&ranks);

        // 统计每个点数出现的次数
        let mut rank_counts: Vec<(u8, u8)> = Vec::new();
        let mut counts: std::collections::HashMap<u8, u8> = std::collections::HashMap::new();
        for &r in &ranks {
            *counts.entry(r).or_insert(0) += 1;
        }
        for (&rank, &count) in &counts {
            rank_counts.push((count, rank));
        }
        rank_counts.sort_by(|a, b| b.0.cmp(&a.0).then(b.1.cmp(&a.1)));

        let category = if is_flush && is_straight {
            if ranks[0] == 14 && ranks[1] == 13 {
                HandRankCategory::RoyalFlush
            } else {
                HandRankCategory::StraightFlush
            }
        } else if rank_counts[0].0 == 4 {
            HandRankCategory::FourOfAKind
        } else if rank_counts[0].0 == 3 && rank_counts.len() > 1 && rank_counts[1].0 >= 2 {
            HandRankCategory::FullHouse
        } else if is_flush {
            HandRankCategory::Flush
        } else if is_straight {
            HandRankCategory::Straight
        } else if rank_counts[0].0 == 3 {
            HandRankCategory::ThreeOfAKind
        } else if rank_counts[0].0 == 2 && rank_counts.len() > 1 && rank_counts[1].0 == 2 {
            HandRankCategory::TwoPair
        } else if rank_counts[0].0 == 2 {
            HandRankCategory::OnePair
        } else {
            HandRankCategory::HighCard
        };

        let kickers: Vec<u8> = rank_counts
            .iter()
            .flat_map(|(count, rank)| std::iter::repeat(*rank).take(*count as usize))
            .collect();

        HandEvaluation { category, kickers }
    }

    /// 检查是否为顺子
    fn check_straight(&self, ranks: &[u8]) -> bool {
        if ranks.len() < 5 {
            return false;
        }
        let mut sorted = ranks.to_vec();
        sorted.sort_by(|a, b| b.cmp(a));
        sorted.dedup();

        if sorted.len() < 5 {
            return false;
        }

        // 检查普通顺子
        if sorted[0] - sorted[4] == 4 {
            return true;
        }

        // 检查 A-2-3-4-5 (轮子)
        if sorted[0] == 14 && sorted[1] == 5 && sorted[2] == 4 && sorted[3] == 3 && sorted[4] == 2 {
            return true;
        }

        false
    }

    /// 摊牌，确定赢家，记录摊牌结果
    fn showdown(&mut self) -> Vec<String> {
        let mut winners = Vec::new();
        let mut best_eval: Option<HandEvaluation> = None;
        let mut results = Vec::new();

        for player in &self.players {
            if player.folded {
                continue;
            }

            let mut all_cards = player.hand.clone();
            all_cards.extend(self.community_cards.iter().cloned());
            let eval = self.evaluate_hand(&all_cards);

            if best_eval.is_none() || eval > *best_eval.as_ref().unwrap() {
                best_eval = Some(eval.clone());
                winners.clear();
                winners.push(player.id.clone());
            } else if Some(&eval) == best_eval.as_ref() {
                winners.push(player.id.clone());
            }

            results.push(ShowdownResult {
                player_id: player.id.clone(),
                hand: player.hand.clone(),
                hand_rank: eval.category,
                is_winner: false, // 临时标记，后面更新
            });
        }

        // 更新赢家标记
        for result in &mut results {
            result.is_winner = winners.contains(&result.player_id);
        }

        self.showdown_results = results;
        winners
    }

    /// 从 action 值中解析玩家动作
    ///
    /// 支持两种格式：
    /// 1. 直接动作: {"action": "fold"} 或 {"action": "raise", "amount": 100}
    /// 2. Tool calls 格式 (AI function calling 响应):
    ///    {"tool_calls": [{"function": {"name": "poker_action", "arguments": "{\"action\":\"fold\"}"}}]}
    fn parse_player_action(&self, action: &serde_json::Value) -> Result<PlayerAction, String> {
        // 尝试从 tool_calls 中提取参数
        let action_value =
            if let Some(tool_calls) = action.get("tool_calls").and_then(|v| v.as_array()) {
                if let Some(first_call) = tool_calls.first() {
                    // 提取 function.arguments
                    let args_str = first_call
                        .get("function")
                        .and_then(|f| f.get("arguments"))
                        .and_then(|a| a.as_str())
                        .ok_or("tool_calls[0].function.arguments 缺失或不是字符串")?;

                    serde_json::from_str::<serde_json::Value>(args_str)
                        .map_err(|e| format!("解析 tool_calls arguments 失败: {e}"))?
                } else {
                    return Err("tool_calls 数组为空".to_string());
                }
            } else {
                // 直接使用 action 本身
                action.clone()
            };

        // 从解析后的值中提取动作
        let action_str = action_value
            .get("action")
            .and_then(|v| v.as_str())
            .ok_or("缺少 action 字段")?;

        match action_str {
            "fold" => Ok(PlayerAction::Fold),
            "check" => Ok(PlayerAction::Check),
            "call" => Ok(PlayerAction::Call),
            "raise" => {
                let amount = action_value
                    .get("amount")
                    .and_then(|v| v.as_u64())
                    .ok_or("raise 需要 amount 字段")? as u32;
                Ok(PlayerAction::Raise(amount))
            }
            "all_in" => Ok(PlayerAction::AllIn),
            _ => Err(format!("未知动作: {}", action_str)),
        }
    }
}

/// 简单的伪随机索引生成（实际项目中应使用 rand crate）
fn random_index(max: usize) -> usize {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    use std::time::{SystemTime, UNIX_EPOCH};

    let mut hasher = DefaultHasher::new();
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos()
        .hash(&mut hasher);
    (hasher.finish() as usize) % max
}

impl GameEngine for TexasHoldemEngine {
    fn game_type(&self) -> &str {
        "texas_holdem"
    }

    fn step(
        &mut self,
        actor_id: &str,
        action: serde_json::Value,
    ) -> Result<Vec<EngineEvent>, String> {
        // 如果游戏还没开始，或者已经结束，检查是否可以开始新牌局
        if self.phase == GamePhase::WaitingForPlayers || self.phase == GamePhase::Finished {
            if self.players.len() >= 2 {
                // 如果是 Finished 阶段，先重置
                if self.phase == GamePhase::Finished {
                    self.finished = false;
                    self.phase = GamePhase::WaitingForPlayers;
                }
                self.start_new_hand();
                let mut events = Vec::new();

                // 为每个玩家发送手牌私密消息
                for player in &self.players {
                    if !player.hand.is_empty() {
                        let hand_msg = serde_json::json!({
                            "type": "your_hand",
                            "hand": player.hand,
                        });
                        events.push(EngineEvent::PrivateMessage {
                            actor_id: player.id.clone(),
                            payload: hand_msg,
                        });
                    }
                }

                // 检查第一个行动的玩家是否是 AI
                if let Some(active_id) = self.current_actor() {
                    if let Some(player) = self.players.iter().find(|p| p.id == active_id) {
                        if player.kind == "Ai" {
                            events.push(EngineEvent::TriggerAi(active_id));
                        }
                    }
                }
                return Ok(events);
            }
            return Ok(vec![]);
        }

        // 摊牌阶段自动处理
        if self.phase == GamePhase::Showdown {
            let winners = self.showdown();
            let winner_share = if winners.is_empty() {
                0
            } else {
                self.pot / winners.len() as u32
            };
            for winner_id in &winners {
                if let Some(player) = self.players.iter_mut().find(|p| p.id == *winner_id) {
                    player.chips += winner_share;
                }
            }
            self.collect_bets();
            self.phase = GamePhase::Finished;
            self.finished = true;
            return Ok(vec![EngineEvent::GameOver]);
        }

        // 验证是否轮到该玩家
        let player_index = self
            .players
            .iter()
            .position(|p| p.id == actor_id)
            .ok_or(format!("未注册的玩家: {actor_id}"))?;

        if player_index != self.active_index {
            return Err(format!(
                "还没轮到 {} 行动，当前应该是 {}",
                actor_id, self.players[self.active_index].id
            ));
        }

        if self.players[player_index].folded {
            return Err(format!("{} 已经弃牌", actor_id));
        }

        // 解析动作（支持 tool_calls 格式和直接格式）
        let player_action = self.parse_player_action(&action)?;

        // 执行动作
        match &player_action {
            PlayerAction::Fold => {
                self.players[player_index].folded = true;
                self.players[player_index].acted_this_round = true;
            }
            PlayerAction::Check => {
                if self.players[player_index].current_bet < self.current_bet {
                    return Err("当前有下注，不能 check".to_string());
                }
                self.players[player_index].acted_this_round = true;
            }
            PlayerAction::Call => {
                let call_amount = self.current_bet - self.players[player_index].current_bet;
                let actual = call_amount.min(self.players[player_index].chips);
                self.players[player_index].chips -= actual;
                self.players[player_index].current_bet += actual;
                self.players[player_index].total_bet += actual;
                if self.players[player_index].chips == 0 {
                    self.players[player_index].all_in = true;
                }
                self.players[player_index].acted_this_round = true;
            }
            PlayerAction::Raise(amount) => {
                if *amount <= self.current_bet {
                    return Err(format!("加注金额必须大于当前下注 {}", self.current_bet));
                }
                let total_needed = *amount - self.players[player_index].current_bet;
                let actual = total_needed.min(self.players[player_index].chips);
                self.players[player_index].chips -= actual;
                self.players[player_index].current_bet += actual;
                self.players[player_index].total_bet += actual;
                self.current_bet = self.players[player_index].current_bet;
                if self.players[player_index].chips == 0 {
                    self.players[player_index].all_in = true;
                }
                self.players[player_index].acted_this_round = true;
                // 加注后，其他玩家需要重新行动
                for (i, p) in self.players.iter_mut().enumerate() {
                    if i != player_index && !p.folded && !p.all_in {
                        p.acted_this_round = false;
                    }
                }
            }
            PlayerAction::AllIn => {
                let all_chips = self.players[player_index].chips;
                self.players[player_index].current_bet += all_chips;
                self.players[player_index].total_bet += all_chips;
                self.players[player_index].chips = 0;
                self.players[player_index].all_in = true;
                self.players[player_index].acted_this_round = true;
                // 如果 all-in 金额超过当前下注，其他玩家需要重新行动
                if self.players[player_index].current_bet > self.current_bet {
                    self.current_bet = self.players[player_index].current_bet;
                    for (i, p) in self.players.iter_mut().enumerate() {
                        if i != player_index && !p.folded && !p.all_in {
                            p.acted_this_round = false;
                        }
                    }
                }
            }
        }

        // 记录历史
        self.history.push(ActionHistory {
            actor_id: actor_id.to_string(),
            action: player_action,
            phase: self.phase,
            chips_after: self.players[player_index].chips,
        });

        let mut events = Vec::new();

        // 检查是否只剩一个未弃牌玩家
        if self.non_folded_count() <= 1 {
            let winner_id = self
                .players
                .iter()
                .find(|p| !p.folded)
                .map(|p| p.id.clone())
                .unwrap_or_default();
            if let Some(winner) = self.players.iter_mut().find(|p| p.id == winner_id) {
                winner.chips += self.pot;
                self.pot = 0;
            }
            self.collect_bets();
            self.phase = GamePhase::Finished;
            self.finished = true;
            events.push(EngineEvent::GameOver);
            return Ok(events);
        }

        // 寻找下一个行动的玩家
        if let Some(next) = self.next_active_player(self.active_index) {
            self.active_index = next;

            // 检查下注轮是否结束
            if self.is_betting_round_complete() {
                // 检查是否只有 all-in 玩家还在（无活跃玩家）
                if self.active_players_count() == 0 {
                    // 直接推进到摊牌
                    while self.phase != GamePhase::Showdown && self.phase != GamePhase::Finished {
                        self.advance_phase();
                    }
                    if self.phase == GamePhase::Showdown {
                        let winners = self.showdown();
                        let winner_share = if winners.is_empty() {
                            0
                        } else {
                            self.pot / winners.len() as u32
                        };
                        for winner_id in &winners {
                            if let Some(player) =
                                self.players.iter_mut().find(|p| p.id == *winner_id)
                            {
                                player.chips += winner_share;
                            }
                        }
                        self.collect_bets();
                        self.phase = GamePhase::Finished;
                        self.finished = true;
                        events.push(EngineEvent::GameOver);
                        return Ok(events);
                    }
                } else {
                    // 推进到下一阶段
                    self.advance_phase();

                    if self.phase == GamePhase::Showdown {
                        let winners = self.showdown();
                        let winner_share = if winners.is_empty() {
                            0
                        } else {
                            self.pot / winners.len() as u32
                        };
                        for winner_id in &winners {
                            if let Some(player) =
                                self.players.iter_mut().find(|p| p.id == *winner_id)
                            {
                                player.chips += winner_share;
                            }
                        }
                        self.collect_bets();
                        self.phase = GamePhase::Finished;
                        self.finished = true;
                        events.push(EngineEvent::GameOver);
                        return Ok(events);
                    }
                }
            }
        } else {
            // 没有下一个可行动的玩家（所有人都 all-in 或弃牌）
            // 检查下注轮是否结束，如果是则推进阶段
            if self.is_betting_round_complete() {
                // 推进到摊牌
                while self.phase != GamePhase::Showdown && self.phase != GamePhase::Finished {
                    self.advance_phase();
                }
                if self.phase == GamePhase::Showdown {
                    let winners = self.showdown();
                    let winner_share = if winners.is_empty() {
                        0
                    } else {
                        self.pot / winners.len() as u32
                    };
                    for winner_id in &winners {
                        if let Some(player) = self.players.iter_mut().find(|p| p.id == *winner_id) {
                            player.chips += winner_share;
                        }
                    }
                    self.collect_bets();
                    self.phase = GamePhase::Finished;
                    self.finished = true;
                    events.push(EngineEvent::GameOver);
                    return Ok(events);
                }
            }
        }

        // 检查下一个玩家是否是 AI
        if let Some(active_id) = self.current_actor() {
            if let Some(player) = self.players.iter().find(|p| p.id == active_id) {
                if player.kind == "Ai" {
                    events.push(EngineEvent::TriggerAi(active_id));
                }
            }
        }

        Ok(events)
    }

    fn to_json(&self) -> serde_json::Value {
        // 计算位置标签
        let positions = self.calculate_positions();

        // 为每个玩家生成视角快照（隐藏其他玩家的手牌）
        let players_json: Vec<serde_json::Value> = self
            .players
            .iter()
            .enumerate()
            .map(|(i, p)| {
                serde_json::json!({
                    "id": p.id,
                    "kind": p.kind,
                    "position": positions.get(i).map(|s| s.as_str()).unwrap_or(""),
                    "chips": p.chips,
                    "current_bet": p.current_bet,
                    "total_bet": p.total_bet,
                    "folded": p.folded,
                    "all_in": p.all_in,
                    "hand_count": p.hand.len(),
                })
            })
            .collect();

        let mut result = serde_json::json!({
            "game_type": self.game_type(),
            "room_id": self.room_id,
            "phase": self.phase,
            "pot": self.pot,
            "current_bet": self.current_bet,
            "community_cards": self.community_cards,
            "players": players_json,
            "active_player": self.current_actor(),
            "dealer_index": self.dealer_index,
            "small_blind": self.small_blind,
            "big_blind": self.big_blind,
            "finished": self.finished,
            "history": self.history,
        });

        // 如果游戏结束且有摊牌结果，添加到输出
        if self.finished && !self.showdown_results.is_empty() {
            result["showdown_results"] = serde_json::json!(self.showdown_results);
        }

        result
    }

    fn to_json_for_player(&self, actor_id: &str) -> serde_json::Value {
        let mut result = self.to_json();

        // 如果是观察者，显示所有玩家的手牌
        if actor_id == "spectator" || actor_id.starts_with("human_spectator") {
            let all_hands: Vec<serde_json::Value> = self
                .players
                .iter()
                .map(|p| {
                    serde_json::json!({
                        "player_id": p.id,
                        "hand": p.hand,
                    })
                })
                .collect();
            result["spectator_hands"] = serde_json::json!(all_hands);
            // 也设置第一个玩家的手牌作为your_hand，方便前端显示
            if let Some(first_player) = self.players.first() {
                if !first_player.hand.is_empty() {
                    result["your_hand"] = serde_json::json!(first_player.hand);
                }
            }
        } else if let Some(player) = self.players.iter().find(|p| p.id == actor_id) {
            // 如果玩家在游戏中且有手牌，为其添加手牌信息
            if !player.hand.is_empty() {
                result["your_hand"] = serde_json::json!(player.hand);
            }
        }

        result
    }

    fn current_actor(&self) -> Option<String> {
        if self.phase == GamePhase::WaitingForPlayers
            || self.phase == GamePhase::Showdown
            || self.phase == GamePhase::Finished
        {
            return None;
        }
        // 如果当前玩家无法行动（all-in 或弃牌），返回 None
        if let Some(player) = self.players.get(self.active_index) {
            if player.folded || player.all_in {
                return None;
            }
            Some(player.id.clone())
        } else {
            None
        }
    }

    fn is_finished(&self) -> bool {
        self.finished
    }

    /// AI 使用的工具定义：扑克动作
    fn tools(&self) -> Option<serde_json::Value> {
        Some(serde_json::json!([
            {
                "type": "function",
                "function": {
                    "name": "poker_action",
                    "description": "执行德州扑克动作",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "action": {
                                "type": "string",
                                "enum": ["fold", "check", "call", "raise", "all_in"],
                                "description": "要执行的动作"
                            },
                            "amount": {
                                "type": "integer",
                                "description": "加注金额（仅 raise 时需要）",
                                "minimum": 0
                            }
                        },
                        "required": ["action"]
                    }
                }
            }
        ]))
    }
}
