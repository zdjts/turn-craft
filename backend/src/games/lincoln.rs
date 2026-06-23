use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use platform_core::{
    games::lincoln::{DebateRole, HistoryEntry, LincolnActor, LincolnEngine},
    traits::{ActionKind, GameEngine},
};
use serde_json::Value;

use super::factory::GameFactory;
use crate::ai::config_repo::AiConfigRepository;
use crate::ai::env::AiConfig;
use crate::error::AppError;
use crate::room::model::CreateRoomInput;

/// 首字母大写：将 "judge" 转换为 "Judge"
fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) => format!("{}{}", c.to_uppercase(), chars.as_str()),
        None => String::new(),
    }
}

pub struct LincolnFactory;

#[async_trait]
impl GameFactory for LincolnFactory {
    fn game_type(&self) -> &str {
        "lincoln"
    }

    async fn create(
        &self,
        room_id: &str,
        owner_id: &crate::user::model::UserId,
        input: &CreateRoomInput,
        config_repo: &dyn AiConfigRepository,
    ) -> Result<(Box<dyn GameEngine>, HashMap<String, AiConfig>), AppError> {
        let mut engine = LincolnEngine::new(room_id.to_string(), input.max_round);

        let role_map: HashMap<&str, DebateRole> = HashMap::from([
            ("Judge", DebateRole::Judge),
            ("Pro", DebateRole::Pro),
            ("Con", DebateRole::Con),
        ]);

        let global_system_prompt = "你正在参与一场林肯-道格拉斯辩论。你需要结合当前发给你的环境快照（包含公共历史发言、当前阶段等），严格遵守辩论规则进行发言或裁决。注意：你的真实身份（正方/反方/裁判）和专属要求将放在 user 提示词的最末尾（PRIVATE STATE 中）。";

        let default_prompts: HashMap<&str, &str> = HashMap::from([
            ("Judge", global_system_prompt),
            ("Pro", global_system_prompt),
            ("Con", global_system_prompt),
        ]);

        let global_defaults_key = format!("__defaults_{}__{}", owner_id.0, self.game_type());
        let all_defaults = config_repo
            .get_all_for_room(&global_defaults_key)
            .await
            .unwrap_or_default();
        let mut ai_configs = HashMap::new();

        for (role_name, role_type) in &input.slot_configs {
            let capitalized = capitalize(role_name);
            let debate_role = match role_map.get(capitalized.as_str()) {
                Some(r) => *r,
                None => continue,
            };

            match role_type.as_str() {
                "human" => {
                    let actor_id = if role_name == &input.my_slot {
                        input.my_slot.clone()
                    } else {
                        format!("human_{}", role_name.to_lowercase())
                    };
                    engine.add_actor(actor_id, ActionKind::Human, debate_role);
                }
                "ai" => {
                    let actor_id = format!("ai_{}", role_name.to_lowercase());
                    engine.add_actor(actor_id.clone(), ActionKind::Ai, debate_role);
                    let mut chars = role_name.chars();
                    let capitalized = match chars.next() {
                        Some(c) => format!("{}{}", c.to_uppercase(), chars.as_str()),
                        None => role_name.clone(),
                    };

                    let saved = all_defaults.get(&capitalized);
                    let default_prompt = default_prompts
                        .get(role_name.as_str())
                        .unwrap_or(&"")
                        .to_string();

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
        let engine =
            restore_lincoln(state).map_err(|e| crate::room::error::RoomError::EngineError(e))?;
        Ok(engine)
    }
}

/// 从 JSON 快照恢复 LincolnEngine
///
/// engine_state 来自 to_json() 输出，需包含 cur_role / opening_done / actors / history 等字段。
pub fn restore_lincoln(engine_state: &Value) -> Result<Box<dyn GameEngine>, String> {
    let room_id = engine_state
        .get("room_id")
        .and_then(|v| v.as_str())
        .ok_or("engine_state 缺少 room_id".to_string())?
        .to_string();

    let max_round = engine_state
        .get("max_round")
        .and_then(|v| v.as_u64())
        .ok_or("engine_state 缺少 max_round".to_string())? as usize;

    let round = engine_state
        .get("round")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as usize;

    let finished = engine_state
        .get("finished")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let opening_done = engine_state
        .get("opening_done")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let cur_role: DebateRole = match engine_state.get("cur_role") {
        Some(v) => {
            serde_json::from_value(v.clone()).map_err(|e| format!("解析 cur_role 失败: {e}"))?
        }
        None => return Err("engine_state 缺少 cur_role".to_string()),
    };

    let actors: Vec<LincolnActor> = match engine_state.get("actors") {
        Some(v) => {
            serde_json::from_value(v.clone()).map_err(|e| format!("解析 actors 失败: {e}"))?
        }
        None => return Err("engine_state 缺少 actors".to_string()),
    };

    let history: Vec<HistoryEntry> = engine_state
        .get("history")
        .map(|v| serde_json::from_value(v.clone()).unwrap_or_default())
        .unwrap_or_default();

    let engine = LincolnEngine {
        room_id,
        max_round,
        round,
        cur_role,
        actors,
        history,
        finished,
        opening_done,
    };

    Ok(Box::new(engine))
}
