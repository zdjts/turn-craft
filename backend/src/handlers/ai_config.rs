use axum::Json;
use axum::extract::{Path, State};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::app::AppState;
use crate::auth::middleware::AuthUser;
use crate::error::AppError;

fn mask_api_key(key: &str) -> String {
    if key.len() <= 8 {
        return "****".to_string();
    }
    format!("{}****{}", &key[..2], &key[key.len()-4..])
}

/// GET /rooms/{room_id}/ai-config (薄 Handler - 读 SQLite, API Key 掩码)
pub async fn get_ai_config(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(room_id): Path<String>,
) -> Result<Json<Value>, AppError> {
    let snapshot = state.room_service.get_room_snapshot(&room_id).await
        .map_err(|_| AppError::RoomNotFound)?;

    let snapshot = snapshot.ok_or(AppError::RoomNotFound)?;

    if snapshot.owner_id != user_id {
        return Err(AppError::Forbidden);
    }

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
                "api_key": mask_api_key(&cfg.api_key),
                "base_url": cfg.base_url,
                "model": cfg.model,
                "max_tokens": cfg.max_tokens,
                "prompt": cfg.prompt,
                "style": cfg.style.as_str(),
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
    pub style: Option<String>,
}

/// PUT /rooms/{room_id}/ai-config/{actor_id} (薄 Handler - 写 SQLite)
pub async fn update_ai_config(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
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

    // 2) 更新参数 - 若客户端传的是掩码值则忽略
    if let Some(v) = input.api_key {
        if !v.contains("****") {
            config.api_key = v;
        }
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
    if let Some(v) = input.style {
        config.style = crate::ai::env::AiStyle::from_str(&v);
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

    if let Ok(Some(snapshot)) = state.room_service.get_room_snapshot(&room_id).await {
        let defaults_room_id = format!("__defaults_{}__{}", user_id.0, snapshot.game_type);
        state
            .ai_service
            .config_repo
            .set(&defaults_room_id, &role_name, &config)
            .await
            .map_err(AppError::Ai)?;
    }

    tracing::info!(room_id = %room_id, actor_id = %actor_id, "AI 配置已于 SQLite 更新（含全局默认）");

    Ok(Json(json!({
        "status": "success",
        "config": {
            "api_key": config.api_key,
            "base_url": config.base_url,
            "model": config.model,
            "max_tokens": config.max_tokens,
            "prompt": config.prompt,
            "style": config.style.as_str(),
        }
    })))
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_mask_short_key() {
        assert_eq!(super::mask_api_key(""), "****");
        assert_eq!(super::mask_api_key("abc"), "****");
        assert_eq!(super::mask_api_key("12345678"), "****");
    }

    #[test]
    fn test_mask_normal_key() {
        let result = super::mask_api_key("sk-1234567890abcdef");
        assert!(result.starts_with("sk"), "should start with 'sk'");
        assert!(result.contains("****"), "should contain mask");
        assert!(result.ends_with("cdef"), "should end with last 4 chars");
    }

    #[test]
    fn test_mask_long_key() {
        let key = "sk-abcdefghijklmnopqrstuvwxyz1234";
        let result = super::mask_api_key(key);
        assert_eq!(result, "sk****1234");
    }
}
