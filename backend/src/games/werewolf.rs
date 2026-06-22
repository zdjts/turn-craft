use std::collections::HashMap;

use async_trait::async_trait;
use serde_json::Value;

use platform_core::{
    games::werewolf::{HistoryEvent, Phase, WerewolfEngine, WerewolfPlayer, WerewolfRole},
    traits::{ActionKind, GameEngine},
};

use super::factory::GameFactory;
use crate::ai::config_repo::AiConfigRepository;
use crate::ai::env::AiConfig;
use crate::error::AppError;
use crate::room::model::CreateRoomInput;

pub struct WerewolfFactory;

#[async_trait]
impl GameFactory for WerewolfFactory {
    fn game_type(&self) -> &str {
        "werewolf"
    }

    async fn create(
        &self,
        room_id: &str,
        input: &CreateRoomInput,
        config_repo: &dyn AiConfigRepository,
    ) -> Result<(Box<dyn GameEngine>, HashMap<String, AiConfig>), AppError> {
        let mut engine = WerewolfEngine::new(room_id.to_string());

        let default_prompts: HashMap<WerewolfRole, &str> = HashMap::from([
            (
                WerewolfRole::Werewolf,
                "你是一只狼人。每晚你需要和另一只狼人队友统一意见杀人。白天如果局势不利，你可以选择'自爆'（直接结束白天进入黑夜）。请隐藏好自己的身份，发言时伪装成好人。",
            ),
            (
                WerewolfRole::Seer,
                "你是预言家。每晚你可以查验一名玩家的身份（好人或狼人）。白天你需要通过发言带领好人阵营投票出狼人。",
            ),
            (
                WerewolfRole::Witch,
                "你是女巫。你有一瓶解药和一瓶毒药，解药可救活今晚被狼杀的人，毒药可毒杀任意一人。每晚你只能使用其中一瓶救。",
            ),
            (
                WerewolfRole::Hunter,
                "你是猎人。如果你被狼人杀害或白天被投票出局，你可以开枪带走任意一名存活玩家。但如果是被女巫毒死，你将无法开枪。",
            ),
            (
                WerewolfRole::Villager,
                "你是一个平民。你没有任何夜间技能，只能在白天认真听取大家发言，分辨谁是狼人并投票将其出局。",
            ),
        ]);

        let global_defaults_key = "__defaults__";
        let mut ai_configs = HashMap::new();

        // Standard 7 players mapping for Werewolf:
        // By default UI should send slot names mapped to specific roles, or we just assign roles blindly.
        // Wait, UI will generate slots like "player1", "player2".
        // Let's hardcode a role distribution for 7 slots, or parse from config.
        // Assuming slots are just player names, we assign roles randomly or deterministically.
        // To be safe, we assign them deterministically if exactly 7 slots.

        let mut roles_pool = vec![
            WerewolfRole::Werewolf,
            WerewolfRole::Werewolf,
            WerewolfRole::Seer,
            WerewolfRole::Witch,
            WerewolfRole::Hunter,
            WerewolfRole::Villager,
            WerewolfRole::Villager,
        ];

        for (slot_name, role_type) in &input.slot_configs {
            let role = roles_pool.pop().unwrap_or(WerewolfRole::Villager);

            match role_type.as_str() {
                "human" => {
                    let actor_id = if slot_name == &input.my_slot {
                        input.my_slot.clone()
                    } else {
                        format!("human_{}", slot_name)
                    };
                    engine.add_actor(actor_id, ActionKind::Human, role);
                }
                "ai" => {
                    let actor_id = format!("ai_{}", slot_name);
                    engine.add_actor(actor_id.clone(), ActionKind::Ai, role);

                    let role_str = match role {
                        WerewolfRole::Werewolf => "Werewolf",
                        WerewolfRole::Seer => "Seer",
                        WerewolfRole::Witch => "Witch",
                        WerewolfRole::Hunter => "Hunter",
                        WerewolfRole::Villager => "Villager",
                    };
                    let saved = config_repo.get(global_defaults_key, role_str).await.ok();
                    let default_prompt = default_prompts.get(&role).unwrap_or(&"").to_string();

                    let config = match saved {
                        Some(s) => AiConfig {
                            api_key: s.api_key,
                            base_url: s.base_url,
                            model: s.model,
                            max_tokens: s.max_tokens,
                            prompt: if s.prompt.is_empty() {
                                default_prompt
                            } else {
                                s.prompt
                            },
                        },
                        None => AiConfig {
                            api_key: crate::config::CONFIG.default_ai_api_key.clone(),
                            base_url: crate::config::CONFIG.default_ai_base_url.clone(),
                            model: crate::config::CONFIG.default_ai_model.clone(),
                            prompt: default_prompt,
                            max_tokens: crate::config::CONFIG.default_ai_max_tokens,
                        },
                    };
                    ai_configs.insert(actor_id, config);
                }
                _ => {}
            }
        }
        Ok((Box::new(engine), ai_configs))
    }

    fn restore(&self, state: &Value) -> Result<Box<dyn GameEngine>, AppError> {
        let engine: WerewolfEngine = serde_json::from_value(state.clone()).map_err(|e| {
            crate::room::error::RoomError::EngineError(format!("恢复狼人杀失败: {}", e))
        })?;
        Ok(Box::new(engine))
    }
}
