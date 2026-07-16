use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::traits::{EngineEvent, GameEngine};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum Suit { Hearts, Diamonds, Clubs, Spades }

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum Rank { Two, Three, Four, Five, Six, Seven, Eight, Nine, Ten, Jack, Queen, King, Ace }

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Card { pub suit: Suit, pub rank: Rank }

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum Phase {
    Betting,
    Dealing,
    PlayerTurn { index: usize },
    DealerTurn,
    Settlement,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum ActionType { Hit, Stand, Double }

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BlackjackActor {
    pub id: String,
    pub kind: String,
    pub hand: Vec<Card>,
    pub hand_value: u32,
    pub is_bust: bool,
    pub is_finished: bool,
    pub bet: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DealerHand {
    pub cards: Vec<Card>,
    pub value: u32,
    pub is_bust: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BlackjackEngine {
    pub room_id: String,
    pub actors: Vec<BlackjackActor>,
    pub dealer: DealerHand,
    pub phase: Phase,
    pub deck: Vec<Card>,
    pub finished: bool,
    pub results: Vec<BlackjackResult>,
    pub starting_chips: u32,
    pub config: BlackjackConfig,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BlackjackConfig {
    pub starting_chips: u32,
    pub min_bet: u32,
    pub max_bet: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BlackjackResult {
    pub actor_id: String,
    pub player_value: u32,
    pub dealer_value: u32,
    pub outcome: String,
    pub payout: i32,
}

// ── Helpers ──

fn card_value(card: &Card) -> u32 {
    match card.rank {
        Rank::Two => 2, Rank::Three => 3, Rank::Four => 4, Rank::Five => 5,
        Rank::Six => 6, Rank::Seven => 7, Rank::Eight => 8, Rank::Nine => 9, Rank::Ten => 10,
        Rank::Jack | Rank::Queen | Rank::King => 10, Rank::Ace => 11,
    }
}

fn hand_total(cards: &[Card]) -> u32 {
    let total: u32 = cards.iter().map(card_value).sum();
    let ace_count = cards.iter().filter(|c| c.rank == Rank::Ace).count();
    if total > 21 && ace_count > 0 {
        total - 10 * (ace_count as u32)
    } else {
        total
    }
}

fn new_deck() -> Vec<Card> {
    let suits = [Suit::Hearts, Suit::Diamonds, Suit::Clubs, Suit::Spades];
    let ranks = [Rank::Two, Rank::Three, Rank::Four, Rank::Five, Rank::Six, Rank::Seven,
                 Rank::Eight, Rank::Nine, Rank::Ten, Rank::Jack, Rank::Queen, Rank::King, Rank::Ace];
    let mut deck = Vec::with_capacity(52);
    for suit in &suits {
        for rank in &ranks {
            deck.push(Card { suit: suit.clone(), rank: rank.clone() });
        }
    }
    deck
}

fn shuffle(mut deck: Vec<Card>) -> Vec<Card> {
    use rand::seq::SliceRandom;
    let mut rng = rand::thread_rng();
    deck.shuffle(&mut rng);
    deck
}

fn deal_card(deck: &mut Vec<Card>) -> Card {
    deck.pop().expect("deck empty")
}

impl BlackjackEngine {
    pub fn new(room_id: String, config: BlackjackConfig) -> Self {
        let deck = shuffle(new_deck());
        Self {
            room_id,
            actors: Vec::new(),
            dealer: DealerHand { cards: Vec::new(), value: 0, is_bust: false },
            phase: Phase::Betting,
            deck,
            finished: false,
            results: Vec::new(),
            starting_chips: config.starting_chips,
            config,
        }
    }

    pub fn add_player(&mut self, id: String, kind: String) {
        self.actors.push(BlackjackActor {
            id, kind, hand: Vec::new(), hand_value: 0,
            is_bust: false, is_finished: false, bet: 0,
        });
    }
}

impl GameEngine for BlackjackEngine {
    fn game_type(&self) -> &str { "blackjack" }

    fn step(&mut self, _actor_id: &str, action: Value) -> Result<Vec<EngineEvent>, crate::error::EngineError> {
        match &self.phase {
            Phase::Betting => {
                // All players auto-bet min_bet
                for player in &mut self.actors {
                    player.bet = self.config.min_bet;
                }
                self.phase = Phase::Dealing;
                // Deal 2 cards to each player and dealer
                for _ in 0..2 {
                    for player in &mut self.actors {
                        player.hand.push(deal_card(&mut self.deck));
                        player.hand_value = hand_total(&player.hand);
                    }
                    self.dealer.cards.push(deal_card(&mut self.deck));
                }
                self.dealer.value = hand_total(&self.dealer.cards);
                // After deal, start player turns
                self.phase = Phase::PlayerTurn { index: 0 };
                let idx = 0;
                if idx < self.actors.len() {
                    let aid = self.actors[idx].id.clone();
                    return Ok(vec![EngineEvent::TriggerAi(aid)]);
                }
                Ok(vec![])
            }

            Phase::PlayerTurn { index } => {
                let idx = *index;
                if idx >= self.actors.len() {
                    self.phase = Phase::DealerTurn;
                    // Dealer draws until 17
                    while self.dealer.value < 17 {
                        self.dealer.cards.push(deal_card(&mut self.deck));
                        self.dealer.value = hand_total(&self.dealer.cards);
                        if self.dealer.value > 21 { self.dealer.is_bust = true; break; }
                    }
                    self.phase = Phase::Settlement;
                    self.finished = true;
                    return Ok(vec![EngineEvent::GameOver]);
                }

                let act_type = action.get("action").and_then(|v| v.as_str()).unwrap_or("");
                let player = &mut self.actors[idx];

                match act_type {
                    "hit" => {
                        player.hand.push(deal_card(&mut self.deck));
                        player.hand_value = hand_total(&player.hand);
                        if player.hand_value > 21 {
                            player.is_bust = true;
                            player.is_finished = true;
                            self.phase = Phase::PlayerTurn { index: idx + 1 };
                        }
                    }
                    "stand" => {
                        player.is_finished = true;
                        self.phase = Phase::PlayerTurn { index: idx + 1 };
                    }
                    "double" => {
                        player.bet *= 2;
                        player.hand.push(deal_card(&mut self.deck));
                        player.hand_value = hand_total(&player.hand);
                        if player.hand_value > 21 { player.is_bust = true; }
                        player.is_finished = true;
                        self.phase = Phase::PlayerTurn { index: idx + 1 };
                    }
                    _ => return Err(crate::error::EngineError("未知 action".into())),
                }

                let new_idx = idx + 1;
                if new_idx < self.actors.len() {
                    let aid = self.actors[new_idx].id.clone();
                    Ok(vec![EngineEvent::TriggerAi(aid)])
                } else {
                    // All players done → dealer turn
                    while self.dealer.value < 17 {
                        self.dealer.cards.push(deal_card(&mut self.deck));
                        self.dealer.value = hand_total(&self.dealer.cards);
                        if self.dealer.value > 21 { self.dealer.is_bust = true; break; }
                    }
                    self.phase = Phase::Settlement;
                    self.finished = true;
                    Ok(vec![EngineEvent::GameOver])
                }
            }

            Phase::DealerTurn | Phase::Settlement => {
                self.finished = true;
                Ok(vec![EngineEvent::GameOver])
            }

            Phase::Dealing => Ok(vec![]),
        }
    }

    fn to_json(&self) -> Value {
        json!({
            "game_type": "blackjack",
            "room_id": self.room_id,
            "players": self.actors,
            "dealer": self.dealer,
            "phase": self.phase,
            "finished": self.finished,
            "results": self.results,
            "starting_chips": self.starting_chips,
        })
    }

    fn to_json_for_player(&self, _actor_id: &str) -> Value {
        let mut state = self.to_json();
        // Hide dealer's hole card (first card if still hidden)
        if self.phase != Phase::Settlement && !self.finished {
            let visible_dealer = json!({
                "upcard": self.dealer.cards.first(),
                "card_count": self.dealer.cards.len(),
            });
            state["dealer"] = visible_dealer;
        }
        state
    }

    fn to_ai_prompt(&self, actor_id: &str) -> String {
        let player = self.actors.iter().find(|a| a.id == actor_id);
        let hand_info = match player {
            Some(p) => format!("你的手牌: {:?}, 点数: {}, 赌注: {}", p.hand.len(), p.hand_value, p.bet),
            None => "".to_string(),
        };
        let dealer_up = self.dealer.cards.first().map(|c| format!("{:?}", c)).unwrap_or("?".to_string());
        format!(
            "当前阶段: {:?}\n你的手牌: {}\n庄家明牌: {}\n可选行动: hit(要牌), stand(停牌), double(加倍)\n\n\
             根据你的风格决定行动。保守风格在12点以上停牌，激进风格在15点还继续要牌。",
            self.phase, hand_info, dealer_up
        )
    }

    fn current_actor(&self) -> Option<String> {
        match &self.phase {
            Phase::PlayerTurn { index } => {
                if *index < self.actors.len() {
                    let player = &self.actors[*index];
                    if player.is_finished { None } else { Some(player.id.clone()) }
                } else { None }
            }
            _ => None,
        }
    }

    fn is_finished(&self) -> bool { self.finished }
}
