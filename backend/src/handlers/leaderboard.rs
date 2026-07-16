use axum::{
    Json,
    extract::{Path, Query, State},
};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::app::AppState;
use crate::error::AppError;

#[derive(Deserialize)]
pub struct MinGamesQuery {
    pub min_games: Option<i64>,
}

/// 从 engine_state 提取玩家是否获胜（通用版，覆盖所有游戏）

/// GET /leaderboard/games — 按完成对局数排名
pub async fn leaderboard_games(
    State(state): State<AppState>,
) -> Result<Json<Value>, AppError> {
    let pool = &state.room_service.pool;

    let rows: Vec<(String, i64)> = sqlx::query_as(
        "SELECT owner_id, COUNT(*) as cnt FROM rooms WHERE json_extract(engine_state, '$.finished') = 1 GROUP BY owner_id ORDER BY cnt DESC LIMIT 50"
    )
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::Internal(e.into()))?;

    let mut entries = Vec::new();
    for (owner_id, cnt) in rows {
        let username: Option<String> = sqlx::query_scalar("SELECT username FROM users WHERE id = ?")
            .bind(&owner_id)
            .fetch_optional(pool)
            .await
            .map_err(|e| AppError::Internal(e.into()))?;
        entries.push(json!({
            "user_id": owner_id,
            "username": username.unwrap_or_else(|| "?".to_string()),
            "value": cnt
        }));
    }

    Ok(Json(json!({ "entries": entries })))
}

/// GET /leaderboard/wins — 按胜利次数排名（全游戏覆盖）
pub async fn leaderboard_wins(
    State(state): State<AppState>,
) -> Result<Json<Value>, AppError> {
    let pool = &state.room_service.pool;

    let all: Vec<(String, i64)> = sqlx::query_as(
        "SELECT owner_id, COUNT(*) as cnt FROM rooms WHERE json_extract(engine_state, '$.finished') = 1 GROUP BY owner_id ORDER BY cnt DESC LIMIT 50"
    )
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::Internal(e.into()))?;

    let mut entries = Vec::new();
    for (uid, cnt) in all {
        let username: Option<String> = sqlx::query_scalar("SELECT username FROM users WHERE id = ?")
            .bind(&uid)
            .fetch_optional(pool)
            .await
            .map_err(|e| AppError::Internal(e.into()))?;
        entries.push(json!({ "user_id": uid, "username": username.unwrap_or_else(|| "?".to_string()), "value": cnt }));
    }

    Ok(Json(json!({ "entries": entries })))
}

/// GET /leaderboard/by-game/{game_type} — 按游戏类型的完成对局排名
pub async fn leaderboard_by_game(
    State(state): State<AppState>,
    Path(game_type): Path<String>,
) -> Result<Json<Value>, AppError> {
    let pool = &state.room_service.pool;

    let rows: Vec<(String, i64)> = sqlx::query_as(
        "SELECT owner_id, COUNT(*) as cnt FROM rooms WHERE game_type = ? AND json_extract(engine_state, '$.finished') = 1 GROUP BY owner_id ORDER BY cnt DESC LIMIT 50"
    )
    .bind(&game_type)
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::Internal(e.into()))?;

    let mut entries = Vec::new();
    for (owner_id, cnt) in rows {
        let username: Option<String> = sqlx::query_scalar("SELECT username FROM users WHERE id = ?")
            .bind(&owner_id)
            .fetch_optional(pool)
            .await
            .map_err(|e| AppError::Internal(e.into()))?;
        entries.push(json!({
            "user_id": owner_id,
            "username": username.unwrap_or_else(|| "?".to_string()),
            "value": cnt
        }));
    }

    Ok(Json(json!({ "entries": entries })))
}

/// GET /leaderboard/experienced — 按完成局数排名（至少 N 局，显示经验最丰富的玩家）
pub async fn leaderboard_experienced(
    State(state): State<AppState>,
    Query(q): Query<MinGamesQuery>,
) -> Result<Json<Value>, AppError> {
    let min_games = q.min_games.unwrap_or(5);
    let pool = &state.room_service.pool;

    let rows: Vec<(String, i64)> = sqlx::query_as(
        "SELECT owner_id, COUNT(*) as cnt FROM rooms WHERE json_extract(engine_state, '$.finished') = 1 GROUP BY owner_id HAVING cnt >= ? ORDER BY cnt DESC LIMIT 50"
    )
    .bind(min_games)
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::Internal(e.into()))?;

    let mut entries = Vec::new();
    for (owner_id, cnt) in rows {
        let username: Option<String> = sqlx::query_scalar("SELECT username FROM users WHERE id = ?")
            .bind(&owner_id)
            .fetch_optional(pool)
            .await
            .map_err(|e| AppError::Internal(e.into()))?;
        entries.push(json!({
            "user_id": owner_id,
            "username": username.unwrap_or_else(|| "?".to_string()),
            "value": cnt
        }));
    }

    Ok(Json(json!({ "entries": entries })))
}
