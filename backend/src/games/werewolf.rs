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
        owner_id: &crate::user::model::UserId,
        input: &CreateRoomInput,
        config_repo: &dyn AiConfigRepository,
    ) -> Result<(Box<dyn GameEngine>, HashMap<String, AiConfig>), AppError> {
        let mut engine = WerewolfEngine::new(room_id.to_string());

        let global_system_prompt = "你正在参与一场 7 人局狼人杀游戏。游戏包含狼人、预言家、女巫、猎人、平民。你需要结合当前发给你的环境快照（包含公共历史、私有历史、个人身份与存活状态），严格遵守狼人杀规则进行逻辑推理、对话博弈和动作决策。注意：你的真实身份和专属能力说明将放在 user 提示词的最末尾（PRIVATE STATE 中）。";

        let default_prompts: HashMap<WerewolfRole, &str> = HashMap::from([
            (WerewolfRole::Werewolf, global_system_prompt),
            (WerewolfRole::Seer, global_system_prompt),
            (WerewolfRole::Witch, global_system_prompt),
            (WerewolfRole::Hunter, global_system_prompt),
            (WerewolfRole::Villager, global_system_prompt),
        ]);

        let global_defaults_key = format!("__defaults_{}__{}", owner_id.0, self.game_type());
        let all_defaults = config_repo
            .get_all_for_room(&global_defaults_key)
            .await
            .unwrap_or_default();
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

        // Shuffle the roles so they are randomly assigned
        use rand::seq::SliceRandom;
        let mut rng = rand::thread_rng();
        roles_pool.shuffle(&mut rng);

        // Iterate over `input.slots` so players are added to `engine.players` in the correct seat order
        for slot_name in &input.slots {
            let role_type = input.slot_configs.get(slot_name).map(|s| s.as_str()).unwrap_or("ai");
            let role = roles_pool.pop().unwrap_or(WerewolfRole::Villager);

            match role_type {
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

                    let mut chars = slot_name.chars();
                    let capitalized_slot = match chars.next() {
                        Some(c) => format!("{}{}", c.to_uppercase(), chars.as_str()),
                        None => slot_name.clone(),
                    };

                    let saved = all_defaults.get(&capitalized_slot);
                    let default_prompt = default_prompts.get(&role).unwrap_or(&"").to_string();

                    let config = match saved {
                        Some(s) => AiConfig {
                            api_key: s.api_key.clone(),
                            base_url: s.base_url.clone(),
                            model: s.model.clone(),
                            max_tokens: s.max_tokens,
                            prompt: if s.prompt.is_empty() {
                                default_prompt
                            } else {
                                s.prompt.clone()
                            },
                        },
                        None => {
                            let fallback = all_defaults.values().next();
                            AiConfig {
                                api_key: fallback.map(|f| f.api_key.clone()).unwrap_or_else(|| {
                                    crate::config::CONFIG.default_ai_api_key.clone()
                                }),
                                base_url: fallback.map(|f| f.base_url.clone()).unwrap_or_else(
                                    || crate::config::CONFIG.default_ai_base_url.clone(),
                                ),
                                model: fallback.map(|f| f.model.clone()).unwrap_or_else(|| {
                                    crate::config::CONFIG.default_ai_model.clone()
                                }),
                                prompt: default_prompt,
                                max_tokens: fallback
                                    .map(|f| f.max_tokens)
                                    .unwrap_or(crate::config::CONFIG.default_ai_max_tokens),
                            }
                        }
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
