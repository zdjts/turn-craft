use std::collections::HashMap;

use async_trait::async_trait;
use platform_core::games::blackjack::{BlackjackConfig, BlackjackEngine};
use platform_core::traits::{GameEngine, GameMeta};
use serde_json::Value;

use super::factory::GameFactory;
use crate::ai::config_repo::AiConfigRepository;
use crate::ai::env::AiConfig;
use crate::error::AppError;
use crate::room::model::CreateRoomInput;

pub struct BlackjackFactory;

#[async_trait]
impl GameFactory for BlackjackFactory {
    fn game_type(&self) -> &str { "blackjack" }

    fn meta(&self) -> GameMeta {
        GameMeta {
            game_type: "blackjack".into(),
            name: "二十一点".into(),
            description: "经典 Blackjack · 庄家对赌 · 策略博弈".into(),
            min_players: 1,
            max_players: 6,
            slot_names: (1..=6).map(|i| format!("player{}", i)).collect(),
            config_schema: Some(serde_json::json!({
                "starting_chips": { "type": "integer", "default": 1000 },
                "min_bet": { "type": "integer", "default": 10 },
                "max_bet": { "type": "integer", "default": 100 },
            })),
        }
    }

    async fn create(
        &self,
        room_id: &str,
        _owner_id: &crate::user::model::UserId,
        input: &CreateRoomInput,
        _config_repo: &dyn AiConfigRepository,
    ) -> Result<(Box<dyn GameEngine>, HashMap<String, AiConfig>), AppError> {
        let player_count = input.slots.len();
        let gc = input.game_config.as_ref().and_then(|v| v.as_object());
        let starting_chips = gc.and_then(|o| o.get("starting_chips")).and_then(|v| v.as_u64()).unwrap_or(1000) as u32;
        let min_bet = gc.and_then(|o| o.get("min_bet")).and_then(|v| v.as_u64()).unwrap_or(10) as u32;
        let max_bet = gc.and_then(|o| o.get("max_bet")).and_then(|v| v.as_u64()).unwrap_or(100) as u32;

        let config = BlackjackConfig { starting_chips, min_bet, max_bet };
        let mut engine = BlackjackEngine::new(room_id.to_string(), config);

        let mut ai_configs = HashMap::new();
        for i in 1..=player_count {
            let slot = format!("player{}", i);
            let role_type = input.slot_configs.get(&slot).map(|s| s.as_str()).unwrap_or("ai");
            match role_type {
                "human" => engine.add_player(slot, "Human".into()),
                "ai" => {
                    engine.add_player(slot.clone(), "Ai".into());
                    let config = AiConfig {
                        api_key: crate::config::CONFIG.default_ai_api_key.clone(),
                        base_url: crate::config::CONFIG.default_ai_base_url.clone(),
                        model: crate::config::CONFIG.default_ai_model.clone(),
                        max_tokens: crate::config::CONFIG.default_ai_max_tokens,
                        prompt: "你正在玩二十一点。根据你的风格决定要牌或停牌。\
                                 保守型：12点以上停牌。激进型：15点还继续要牌。\
                                 注意庄家明牌，计算爆牌概率。".into(),
                        style: crate::ai::env::AiStyle::Default,
                    };
                    ai_configs.insert(slot, config);
                }
                _ => {}
            }
        }

        Ok((Box::new(engine), ai_configs))
    }

    fn restore(&self, state: &Value) -> Result<Box<dyn GameEngine>, AppError> {
        use platform_core::games::blackjack::{BlackjackActor, DealerHand, Phase, BlackjackResult, BlackjackConfig};
        let room_id = state.get("room_id").and_then(|v| v.as_str()).ok_or(
            AppError::BadRequest("缺少 room_id".into())
        )?.to_string();

        let actors: Vec<BlackjackActor> = serde_json::from_value(
            state.get("players").cloned().unwrap_or_default()
        ).unwrap_or_default();

        let dealer: DealerHand = serde_json::from_value(
            state.get("dealer").cloned().unwrap_or_default()
        ).unwrap_or(DealerHand { cards: Vec::new(), value: 0, is_bust: false });

        let phase: Phase = serde_json::from_value(
            state.get("phase").cloned().unwrap_or_default()
        ).unwrap_or(Phase::Betting);

        let results: Vec<BlackjackResult> = serde_json::from_value(
            state.get("results").cloned().unwrap_or_default()
        ).unwrap_or_default();

        let starting_chips = state.get("starting_chips").and_then(|v| v.as_u64()).unwrap_or(1000) as u32;
        let config = BlackjackConfig { starting_chips, min_bet: 10, max_bet: 100 };
        let mut engine = BlackjackEngine::new(room_id, config);
        engine.actors = actors;
        engine.dealer = dealer;
        engine.phase = phase;
        engine.results = results;
        engine.finished = state.get("finished").and_then(|v| v.as_bool()).unwrap_or(false);
        Ok(Box::new(engine))
    }
}
