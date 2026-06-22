use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use platform_core::{
    games::texas_holdem::{
        ActionHistory, GamePhase, PokerPlayer, ShowdownResult, TexasHoldemEngine,
    },
    traits::{ActionKind, GameEngine},
};
use serde_json::Value;

use super::factory::GameFactory;
use crate::ai::config_repo::AiConfigRepository;
use crate::ai::env::AiConfig;
use crate::error::AppError;
use crate::room::model::CreateRoomInput;

pub struct TexasHoldemFactory;

#[async_trait]
impl GameFactory for TexasHoldemFactory {
    fn game_type(&self) -> &str {
        "texas_holdem"
    }

    async fn create(
        &self,
        room_id: &str,
        input: &CreateRoomInput,
        config_repo: &dyn AiConfigRepository,
    ) -> Result<(Box<dyn GameEngine>, HashMap<String, AiConfig>), AppError> {
        let gc = input.game_config.as_ref();
        let small_blind = gc
            .and_then(|v| v.get("small_blind"))
            .and_then(|v| v.as_u64())
            .unwrap_or(10) as u32;
        let big_blind = gc
            .and_then(|v| v.get("big_blind"))
            .and_then(|v| v.as_u64())
            .unwrap_or(20) as u32;
        let starting_chips = gc
            .and_then(|v| v.get("starting_chips"))
            .and_then(|v| v.as_u64())
            .unwrap_or(1000) as u32;

        let mut engine = TexasHoldemEngine::new(room_id.to_string(), small_blind, big_blind);
        let default_prompt = "你是一位经验丰富的德州扑克 AI 玩家。请根据当前局面做出最优决策（fold/check/call/raise/all_in）。";
        let global_defaults_key = "__defaults__";
        let mut ai_configs = HashMap::new();

        for (player_id, player_type) in &input.slot_configs {
            match player_type.as_str() {
                "human" => {
                    engine.add_player(player_id.clone(), ActionKind::Human, starting_chips);
                }
                "ai" => {
                    engine.add_player(player_id.clone(), ActionKind::Ai, starting_chips);

                    // 优先从 SQLite 配置仓储获取默认全局配置
                    let saved = config_repo.get(global_defaults_key, player_id).await.ok();

                    let config = match saved {
                        Some(s) => AiConfig {
                            api_key: s.api_key,
                            base_url: s.base_url,
                            model: s.model,
                            max_tokens: s.max_tokens,
                            prompt: if s.prompt.is_empty() {
                                default_prompt.to_string()
                            } else {
                                s.prompt
                            },
                        },
                        None => AiConfig {
                            api_key: crate::config::CONFIG.default_ai_api_key.clone(),
                            base_url: crate::config::CONFIG.default_ai_base_url.clone(),
                            model: crate::config::CONFIG.default_ai_model.clone(),
                            prompt: default_prompt.to_string(),
                            max_tokens: crate::config::CONFIG.default_ai_max_tokens,
                        },
                    };

                    ai_configs.insert(player_id.clone(), config);
                }
                _ => {}
            }
        }

        Ok((Box::new(engine), ai_configs))
    }

    fn restore(&self, state: &Value) -> Result<Box<dyn GameEngine>, AppError> {
        let engine = restore_texas_holdem(state)
            .map_err(|e| crate::room::error::RoomError::EngineError(e))?;
        Ok(engine)
    }
}

/// 从 JSON 快照恢复 TexasHoldemEngine
///
/// engine_state 来自 to_json() 输出。
/// 注意：玩家手牌在快照中被隐藏，恢复后需要重新发牌。
pub fn restore_texas_holdem(engine_state: &Value) -> Result<Box<dyn GameEngine>, String> {
    let room_id = engine_state
        .get("room_id")
        .and_then(|v| v.as_str())
        .ok_or("engine_state 缺少 room_id".to_string())?
        .to_string();

    let small_blind = engine_state
        .get("small_blind")
        .and_then(|v| v.as_u64())
        .ok_or("engine_state 缺少 small_blind".to_string())? as u32;

    let big_blind = engine_state
        .get("big_blind")
        .and_then(|v| v.as_u64())
        .ok_or("engine_state 缺少 big_blind".to_string())? as u32;

    let pot = engine_state
        .get("pot")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u32;

    let current_bet = engine_state
        .get("current_bet")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u32;

    let dealer_index = engine_state
        .get("dealer_index")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as usize;

    let finished = engine_state
        .get("finished")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let phase: GamePhase = match engine_state.get("phase") {
        Some(v) => {
            serde_json::from_value(v.clone()).map_err(|e| format!("解析 phase 失败: {e}"))?
        }
        None => return Err("engine_state 缺少 phase".to_string()),
    };

    let community_cards = engine_state
        .get("community_cards")
        .map(|v| serde_json::from_value(v.clone()).unwrap_or_default())
        .unwrap_or_default();

    let players_json = engine_state
        .get("players")
        .and_then(|v| v.as_array())
        .ok_or("engine_state 缺少 players".to_string())?;

    let mut players = Vec::new();
    for p in players_json {
        let id = p
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or("player 缺少 id")?
            .to_string();
        let kind = p
            .get("kind")
            .and_then(|v| v.as_str())
            .unwrap_or("Human")
            .to_string();
        let chips = p.get("chips").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
        let current_bet = p.get("current_bet").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
        let total_bet = p.get("total_bet").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
        let folded = p.get("folded").and_then(|v| v.as_bool()).unwrap_or(false);
        let all_in = p.get("all_in").and_then(|v| v.as_bool()).unwrap_or(false);

        players.push(PokerPlayer {
            id,
            kind,
            chips,
            hand: Vec::new(), // 手牌在快照中被隐藏，无法恢复
            current_bet,
            total_bet,
            folded,
            all_in,
            acted_this_round: false,
        });
    }

    let history: Vec<ActionHistory> = engine_state
        .get("history")
        .map(|v| serde_json::from_value(v.clone()).unwrap_or_default())
        .unwrap_or_default();

    let showdown_results: Vec<ShowdownResult> = engine_state
        .get("showdown_results")
        .map(|v| serde_json::from_value(v.clone()).unwrap_or_default())
        .unwrap_or_default();

    // 从 active_player 推导 active_index
    let active_player_id = engine_state.get("active_player").and_then(|v| v.as_str());

    let active_index = active_player_id
        .and_then(|id| players.iter().position(|p| p.id == id))
        .unwrap_or(dealer_index);

    let engine = TexasHoldemEngine {
        room_id,
        players,
        deck: Vec::new(),
        community_cards,
        pot,
        current_bet,
        phase,
        dealer_index,
        active_index,
        small_blind,
        big_blind,
        history,
        finished,
        round_reset_bets: false,
        showdown_results,
    };

    Ok(Box::new(engine))
}
