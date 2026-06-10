use std::collections::HashMap;
use std::sync::Arc;

use dashmap::DashMap;
use platform_core::{
    games::lincoln::{DebateRole, HistoryEntry, LincolnActor, LincolnEngine},
    traits::{ActionKind, GameEngine},
};
use serde_json::Value;

use crate::ai::env::AiConfig;

/// 创建林肯辩论引擎：初始化角色和 AI 配置
pub fn create_lincoln(
    room_id: &str,
    my_role: &str,
    role_config: &HashMap<String, String>,
    max_round: usize,
    global_ai_configs: Option<&Arc<DashMap<String, AiConfig>>>,
) -> (Box<dyn GameEngine>, HashMap<String, AiConfig>) {
    let mut engine = LincolnEngine::new(room_id.to_string(), max_round);

    let role_map: HashMap<&str, DebateRole> = HashMap::from([
        ("Judge", DebateRole::Judge),
        ("Pro", DebateRole::Pro),
        ("Con", DebateRole::Con),
    ]);

    let default_prompts: HashMap<&str, &str> = HashMap::from([
        ("Judge", "你是一位公正的辩论裁判。请给出辩题，听取双方论点后做出最终裁决。字数控制在300字以内。"),
        ("Pro", "你现在是激进的立论家。请作为正方，针对裁判给出的辩题，发表具有说服力的论点。字数控制在200字以内。"),
        ("Con", "你现在是沉稳的驳论家。请作为反方，严密审视正方的发言，并进行针锋相对的反驳。字数控制在200字以内。"),
    ]);

    // 上一次保存的全局默认配置 key
    let global_defaults_key = "__defaults__";

    let mut ai_configs = HashMap::new();

    for (role_name, role_type) in role_config {
        let debate_role = match role_map.get(role_name.as_str()) {
            Some(r) => *r,
            None => continue,
        };

        match role_type.as_str() {
            "human" => {
                let actor_id = if role_name == my_role {
                    my_role.to_string()
                } else {
                    format!("human_{}", role_name.to_lowercase())
                };
                engine.add_actor(actor_id, ActionKind::Human, debate_role);
            }
            "ai" => {
                let actor_id = format!("ai_{}", role_name.to_lowercase());
                engine.add_actor(actor_id.clone(), ActionKind::Ai, debate_role);

                // 优先从全局默认配置读取（用户上次在 Settings 保存的值）
                let saved = global_ai_configs.and_then(|gc| {
                    gc.get(&format!("{}/{}", global_defaults_key, role_name))
                        .map(|r| r.clone())
                });

                let default_prompt = default_prompts
                    .get(role_name.as_str())
                    .unwrap_or(&"")
                    .to_string();

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
                        api_key: std::env::var("DEEPSEEK_API_KEY").unwrap_or_default(),
                        base_url: "https://api.deepseek.com/v1".to_string(),
                        model: "deepseek-chat".to_string(),
                        prompt: default_prompt,
                        max_tokens: 2048,
                    },
                };

                ai_configs.insert(actor_id, config);
            }
            _ => {}
        }
    }

    (Box::new(engine), ai_configs)
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
        Some(v) => serde_json::from_value(v.clone())
            .map_err(|e| format!("解析 cur_role 失败: {e}"))?,
        None => return Err("engine_state 缺少 cur_role".to_string()),
    };

    let actors: Vec<LincolnActor> = match engine_state.get("actors") {
        Some(v) => serde_json::from_value(v.clone())
            .map_err(|e| format!("解析 actors 失败: {e}"))?,
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
