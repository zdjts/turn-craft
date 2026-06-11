use axum::Json;
use axum::extract::{Path, State};
use serde::Deserialize;

use crate::app::AppState;

/// GET /rooms/{room_id}/ai-config
pub async fn get_ai_config(
    State(state): State<AppState>,
    Path(room_id): Path<String>,
) -> Json<serde_json::Value> {
    let prefix = format!("{}/", room_id);

    let mut result = serde_json::Map::new();
    for entry in state.ai_configs.iter() {
        if entry.key().starts_with(&prefix) {
            let actor_id = entry.key().strip_prefix(&prefix).unwrap_or(entry.key());
            result.insert(
                actor_id.to_string(),
                serde_json::json!({
                    "api_key": entry.value().api_key,
                    "base_url": entry.value().base_url,
                    "model": entry.value().model,
                    "max_tokens": entry.value().max_tokens,
                    "prompt": entry.value().prompt,
                }),
            );
        }
    }

    Json(serde_json::json!({
        "status": "success",
        "configs": result,
    }))
}

/// 更新 AI 配置请求体（所有字段可选）
#[derive(Deserialize)]
pub struct UpdateAiConfigInput {
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub model: Option<String>,
    pub max_tokens: Option<u32>,
    pub prompt: Option<String>,
}

/// PUT /rooms/{room_id}/ai-config/{actor_id}
pub async fn update_ai_config(
    State(state): State<AppState>,
    Path((room_id, actor_id)): Path<(String, String)>,
    Json(input): Json<UpdateAiConfigInput>,
) -> Json<serde_json::Value> {
    let key = format!("{}/{}", room_id, actor_id);

    // 1) 读取当前配置并修改，clone 后立即 drop RefMut 释放写锁
    let updated = {
        let Some(mut cfg) = state.ai_configs.get_mut(&key) else {
            return Json(serde_json::json!({
                "status": "error",
                "message": format!("未找到 AI 配置: {}/{}", room_id, actor_id)
            }));
        };
        if let Some(v) = input.api_key {
            cfg.api_key = v;
        }
        if let Some(v) = input.base_url {
            cfg.base_url = v;
        }
        if let Some(v) = input.model {
            cfg.model = v;
        }
        if let Some(v) = input.max_tokens {
            cfg.max_tokens = v;
        }
        if let Some(v) = input.prompt {
            cfg.prompt = v;
        }
        cfg.clone()
        // cfg (RefMut) 在此 drop，写锁释放
    };

    // 2) 更新全局默认配置（无锁冲突）
    let role_name = actor_id
        .strip_prefix("ai_")
        .map(|r| {
            let mut chars = r.chars();
            match chars.next() {
                Some(c) => format!("{}{}", c.to_uppercase(), chars.as_str()),
                None => r.to_string(),
            }
        })
        .unwrap_or_else(|| actor_id.clone());
    let defaults_key = format!("__defaults__/{}", role_name);
    state.ai_configs.insert(defaults_key, updated.clone());

    // 3) 持久化到文件（此时无写锁，iter() 安全）
    crate::persistence::save_configs_to_file(&state.ai_configs);

    tracing::info!(room_id = %room_id, actor_id = %actor_id, "AI 配置已更新（含全局默认）");

    Json(serde_json::json!({
        "status": "success",
        "config": {
            "api_key": updated.api_key,
            "base_url": updated.base_url,
            "model": updated.model,
            "max_tokens": updated.max_tokens,
            "prompt": updated.prompt,
        }
    }))
}
