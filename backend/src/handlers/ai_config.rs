use axum::Json;
use axum::extract::{Path, State};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::app::AppState;
use crate::error::AppError;

/// GET /rooms/{room_id}/ai-config (薄 Handler - 读 SQLite)
pub async fn get_ai_config(
    State(state): State<AppState>,
    Path(room_id): Path<String>,
) -> Result<Json<Value>, AppError> {
    let configs = state
        .ai_service
        .config_repo
        .get_all_for_room(&room_id)
        .await
        .map_err(AppError::Ai)?;

    let mut result = serde_json::Map::new();
    for (actor_id, cfg) in configs {
        result.insert(
            actor_id,
            json!({
                "api_key": cfg.api_key,
                "base_url": cfg.base_url,
                "model": cfg.model,
                "max_tokens": cfg.max_tokens,
                "prompt": cfg.prompt,
            }),
        );
    }

    Ok(Json(json!({
        "status": "success",
        "configs": result,
    })))
}

/// 更新 AI 配置请求体
#[derive(Deserialize)]
pub struct UpdateAiConfigInput {
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub model: Option<String>,
    pub max_tokens: Option<u32>,
    pub prompt: Option<String>,
}

/// PUT /rooms/{room_id}/ai-config/{actor_id} (薄 Handler - 写 SQLite)
pub async fn update_ai_config(
    State(state): State<AppState>,
    Path((room_id, actor_id)): Path<(String, String)>,
    Json(input): Json<UpdateAiConfigInput>,
) -> Result<Json<Value>, AppError> {
    // 1) 读取当前配置
    let mut config = state
        .ai_service
        .config_repo
        .get(&room_id, &actor_id)
        .await
        .map_err(AppError::Ai)?;

    // 2) 更新参数
    if let Some(v) = input.api_key {
        config.api_key = v;
    }
    if let Some(v) = input.base_url {
        config.base_url = v;
    }
    if let Some(v) = input.model {
        config.model = v;
    }
    if let Some(v) = input.max_tokens {
        config.max_tokens = v;
    }
    if let Some(v) = input.prompt {
        config.prompt = v;
    }

    // 3) 保存更新后的配置
    state
        .ai_service
        .config_repo
        .set(&room_id, &actor_id, &config)
        .await
        .map_err(AppError::Ai)?;

    // 4) 提取角色名并更新全局默认配置
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

    state
        .ai_service
        .config_repo
        .set("__defaults__", &role_name, &config)
        .await
        .map_err(AppError::Ai)?;

    tracing::info!(room_id = %room_id, actor_id = %actor_id, "AI 配置已于 SQLite 更新（含全局默认）");

    Ok(Json(json!({
        "status": "success",
        "config": {
            "api_key": config.api_key,
            "base_url": config.base_url,
            "model": config.model,
            "max_tokens": config.max_tokens,
            "prompt": config.prompt,
        }
    })))
}
