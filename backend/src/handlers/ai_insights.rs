use axum::{
    Json,
    extract::{Path, State},
};
use reqwest::Client;
use serde_json::{Value, json};
use tracing::warn;

use crate::app::AppState;
use crate::error::AppError;
use crate::auth::middleware::AuthUser;
use crate::ai::insights::generate_insights;

/// AI 策略评价 — LLM 生成 + DB 缓存
pub async fn get_ai_insights(
    _user: AuthUser,
    State(state): State<AppState>,
    Path(room_id): Path<String>,
) -> Result<Json<Value>, AppError> {
    let snapshot = state.room_service.get_room_snapshot(&room_id).await?
        .ok_or(AppError::RoomNotFound)?;

    let engine = &snapshot.engine_state;
    let finished = engine.get("finished").and_then(|v| v.as_bool()).unwrap_or(false);
    if !finished {
        return Ok(Json(json!({ "insights": [] })));
    }

    // 检查 DB 缓存
    let pool = &state.room_service.pool;
    let cached: Result<Option<String>, _> = sqlx::query_scalar(
        "SELECT ai_insights FROM rooms WHERE room_id = ? AND ai_insights IS NOT NULL"
    )
    .bind(&room_id)
    .fetch_optional(pool)
    .await;

    if let Ok(Some(json_str)) = cached {
        if let Ok(val) = serde_json::from_str::<Value>(&json_str) {
            return Ok(Json(val));
        }
    }

    // 取第一个 AI 的配置作为 LLM 调用配置
    let ai_config = {
        let configs = &snapshot.ai_configs;
        match configs.values().next() {
            Some(cfg) => cfg.clone(),
            None => {
                warn!("无 AI 配置可用，返回空 insights");
                return Ok(Json(json!({ "insights": [] })));
            }
        }
    };

    let http = Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| AppError::Internal(e.into()))?;

    let insights = generate_insights(&http, &ai_config, engine).await;

    let result = json!({ "insights": insights });
    let result_str = serde_json::to_string(&result).unwrap_or_default();
    let _ = sqlx::query("UPDATE rooms SET ai_insights = ? WHERE room_id = ?")
        .bind(&result_str)
        .bind(&room_id)
        .execute(pool)
        .await;

    Ok(Json(result))
}
