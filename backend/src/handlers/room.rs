use axum::{Json, extract::{Path, State}};
use serde_json::{json, Value};

use crate::app::AppState;
use crate::error::AppError;
use crate::auth::middleware::AuthUser;
use crate::room::model::CreateRoomInput;

/// 创建房间处理器 (薄 Handler)
pub async fn create_room(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Json(input): Json<CreateRoomInput>,
) -> Result<Json<Value>, AppError> {
    tracing::info!(
        user_id = %user_id.0,
        game_type = %input.game_type,
        "接收到创建房间请求"
    );
    let out = state.room_service.create_room(user_id, input).await?;
    Ok(Json(json!({
        "status": "success",
        "room_id": out.room_id,
        "actor_id": out.assigned_slot
    })))
}

/// 删除房间处理器 (薄 Handler)
pub async fn delete_room(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(room_id): Path<String>,
) -> Result<Json<Value>, AppError> {
    tracing::info!(
        user_id = %user_id.0,
        room_id = %room_id,
        "接收到销毁房间请求"
    );
    state.room_service.delete_room(user_id, &room_id).await?;
    Ok(Json(json!({ "status": "success" })))
}
