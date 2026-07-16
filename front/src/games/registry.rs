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

#[derive(Clone)]
pub struct RoomTemplate {
    pub name: String,
    pub desc: String,
    pub icon: &'static str,
    pub role_config: HashMap<String, String>,
    pub my_role: String,
    pub max_round: usize,
    pub game_config: Option<Value>,
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
    pub help_text: &'static [&'static str],
    pub templates: Vec<RoomTemplate>,
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
                lobby_card: crate::games::lincoln::lincoln_lobby_card,
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
                help_text: &[
                    "🎯 目标：通过辩论说服裁判。正方(Pro)支持辩题，反方(Con)反对辩题。",
                    "👨‍⚖️ 法官(Judge)：开局给出辩题，最后裁决胜负。",
                    "💬 发言顺序：法官开题 → 正方 → 反方 → 正方 → 反方 → 法官总结",
                    "🤖 AI 玩家会自动发言。你发言后等待 AI 回应即可。",
                ],
                templates: vec![
                    RoomTemplate {
                        name: "经典辩论".into(), desc: "法官主办，AI 正反方辩论".into(), icon: "🏛️",
                        role_config: HashMap::from([("Judge".into(),"human".into()),("Pro".into(),"ai".into()),("Con".into(),"ai".into())]),
                        my_role: "Judge".into(), max_round: 8, game_config: None,
                    },
                    RoomTemplate {
                        name: "长辩论".into(), desc: "16 轮深度辩论".into(), icon: "📜",
                        role_config: HashMap::from([("Judge".into(),"human".into()),("Pro".into(),"ai".into()),("Con".into(),"ai".into())]),
                        my_role: "Judge".into(), max_round: 16, game_config: None,
                    },
                ],
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
                lobby_card: crate::games::texas_holdem::texas_holdem_lobby_card,
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
                help_text: &[
                    "🎯 目标：赢取所有筹码。通过下注、加注、弃牌等策略击败对手。",
                    "🃏 每局开始每位玩家获得两张底牌，然后依次发公共牌。",
                    "💰 下注轮次：Pre-Flop → Flop → Turn → River → 摊牌",
                    "🤖 AI 玩家会自动行动。轮到你时，底牌会显示在界面中。",
                ],
                templates: vec![
                    RoomTemplate {
                        name: "标准德州".into(), desc: "6 人桌，你 + 5 个 AI".into(), icon: "🃏",
                        role_config: {
                            let mut m = HashMap::new(); m.insert("player1".into(), "human".into());
                            for i in 2..=6 { m.insert(format!("player{}", i), "ai".into()); } m
                        },
                        my_role: "player1".into(), max_round: 100,
                        game_config: Some(serde_json::json!({"small_blind":10,"big_blind":20,"starting_chips":1000})),
                    },
                    RoomTemplate {
                        name: "快速德州".into(), desc: "3 人对抗赛".into(), icon: "⚡",
                        role_config: {
                            let mut m = HashMap::new(); m.insert("player1".into(), "human".into());
                            for i in 2..=3 { m.insert(format!("player{}", i), "ai".into()); } m
                        },
                        my_role: "player1".into(), max_round: 50,
                        game_config: Some(serde_json::json!({"small_blind":5,"big_blind":10,"starting_chips":500})),
                    },
                ],
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
                help_text: &[
                    "🎯 目标：狼人阵营 vs 好人阵营。狼人隐藏身份，好人找出狼人。",
                    "🌙 夜晚阶段：狼人击杀、预言家查验、女巫救人/毒人。",
                    "☀️ 白天阶段：存活玩家发言讨论，然后投票放逐。",
                    "🤖 AI 玩家会自动行动。请关注私密消息查看你的身份和能力。",
                ],
                templates: vec![
                    RoomTemplate {
                        name: "7 人标准局".into(), desc: "完美推演：2 狼 + 预言家 + 女巫 + 猎人 + 2 村民".into(), icon: "🐺",
                        role_config: {
                            let mut m = HashMap::new(); m.insert("Player1".into(), "human".into());
                            for i in 2..=7 { m.insert(format!("Player{}", i), "ai".into()); } m
                        },
                        my_role: "Player1".into(), max_round: 50, game_config: None,
                    },
                ],
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
