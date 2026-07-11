use dioxus::prelude::*;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::LazyLock;

use super::GamePluginProps;

#[derive(Props, Clone, PartialEq)]
pub struct GameConfigProps {
    pub role_config: Signal<HashMap<String, String>>,
    pub my_role: Signal<String>,
    pub max_round: Signal<usize>,
    pub game_config: Signal<Option<Value>>,
}

pub struct DefaultGameConfig {
    pub role_config: HashMap<String, String>,
    pub my_role: String,
    pub max_round: usize,
    pub game_config: Option<Value>,
}

#[derive(Clone)]
pub struct GameUIDefinition {
    pub game_type: &'static str,
    pub name: &'static str,
    pub icon: &'static str,
    pub description: &'static str,
    pub min_players: usize,
    pub max_players: usize,
    pub lobby_card: fn(props: GameConfigProps) -> Element,
    pub game_component: fn(props: GamePluginProps) -> Element,
    pub default_config: fn() -> DefaultGameConfig,
    pub generate_slots: fn(configs: &HashMap<String, String>) -> Vec<String>,
}

pub struct GameUIRegistry {
    games: HashMap<&'static str, GameUIDefinition>,
}

impl GameUIRegistry {
    pub fn new() -> Self {
        let mut games = HashMap::new();

        // 1. Lincoln
        games.insert(
            "lincoln",
            GameUIDefinition {
                game_type: "lincoln",
                name: "林肯辩论",
                icon: "🏛️",
                description: "经典英式辩论 · 法官裁判 · 正反方交锋",
                min_players: 3,
                max_players: 3,
                lobby_card: crate::games::lincoln::LincolnLobbyCard,
                game_component: crate::games::lincoln::LincolnGame,
                default_config: || DefaultGameConfig {
                    role_config: HashMap::from([
                        ("Judge".to_string(), "human".to_string()),
                        ("Pro".to_string(), "ai".to_string()),
                        ("Con".to_string(), "ai".to_string()),
                    ]),
                    my_role: "Judge".to_string(),
                    max_round: 16,
                    game_config: None,
                },
                generate_slots: |_| vec!["Judge".to_string(), "Pro".to_string(), "Con".to_string()],
            },
        );

        // 2. Texas Hold'em
        games.insert(
            "texas_holdem",
            GameUIDefinition {
                game_type: "texas_holdem",
                name: "德州扑克",
                icon: "🃏",
                description: "2-6 人经典德扑 · 盲注博弈 · 心理对抗",
                min_players: 2,
                max_players: 6,
                lobby_card: crate::games::texas_holdem::TexasHoldemLobbyCard,
                game_component: crate::games::texas_holdem::TexasHoldemGame,
                default_config: || {
                    let mut modes = HashMap::new();
                    modes.insert("player1".to_string(), "human".to_string());
                    for i in 2..=6 {
                        modes.insert(format!("player{}", i), "ai".to_string());
                    }
                    DefaultGameConfig {
                        role_config: modes,
                        my_role: "player1".to_string(),
                        max_round: 100,
                        game_config: Some(serde_json::json!({
                            "small_blind": 10,
                            "big_blind": 20,
                            "starting_chips": 1000,
                        })),
                    }
                },
                generate_slots: |configs| {
                    (1..=configs.len())
                        .map(|i| format!("player{}", i))
                        .collect()
                },
            },
        );

        // 3. Werewolf
        games.insert(
            "werewolf",
            GameUIDefinition {
                game_type: "werewolf",
                name: "狼人杀",
                icon: "🐺",
                description: "7 人社交推理 · 狼人暗杀 · 好人投票",
                min_players: 7,
                max_players: 7,
                lobby_card: crate::games::werewolf::WerewolfLobbyCard,
                game_component: crate::games::werewolf::WerewolfGame,
                default_config: || {
                    let mut modes = HashMap::new();
                    modes.insert("Player1".to_string(), "human".to_string());
                    for i in 2..=7 {
                        modes.insert(format!("Player{}", i), "ai".to_string());
                    }
                    DefaultGameConfig {
                        role_config: modes,
                        my_role: "Player1".to_string(),
                        max_round: 50,
                        game_config: None,
                    }
                },
                generate_slots: |_| (1..=7).map(|i| format!("Player{}", i)).collect(),
            },
        );

        Self { games }
    }

    pub fn get(&self, game_type: &str) -> Option<&GameUIDefinition> {
        self.games.get(game_type)
    }

    pub fn all_games(&self) -> Vec<&GameUIDefinition> {
        let mut list: Vec<&GameUIDefinition> = self.games.values().collect();
        list.sort_by_key(|g| g.game_type);
        list
    }
}

pub static REGISTRY: LazyLock<GameUIRegistry> = LazyLock::new(GameUIRegistry::new);
