use axum::{
    Json,
    extract::{Path, State},
};
use serde_json::{Value, json};

use crate::app::AppState;
use crate::auth::middleware::AuthUser;
use crate::error::AppError;
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

/// 获取所有公开的房间列表
pub async fn list_public_rooms(
    State(state): State<AppState>,
) -> Result<Json<Value>, AppError> {
    let rooms = state.room_service.list_public_rooms().await?;
    Ok(Json(json!({
        "status": "success",
        "rooms": rooms
    })))
}

/// 获取当前用户的历史房间列表
pub async fn list_history_rooms(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
) -> Result<Json<Value>, AppError> {
    let rooms = state.room_service.list_history_rooms(user_id).await?;
    Ok(Json(json!({
        "status": "success",
        "rooms": rooms
    })))
}

#[derive(serde::Deserialize)]
pub struct SetRoomPublicInput {
    pub is_public: bool,
}

/// 设置房间是否公开
pub async fn set_room_public(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(room_id): Path<String>,
    Json(input): Json<SetRoomPublicInput>,
) -> Result<Json<Value>, AppError> {
    state
        .room_service
        .set_room_public(user_id, &room_id, input.is_public)
        .await?;
    Ok(Json(json!({ "status": "success" })))
}

/// 获取单个房间详情
pub async fn get_room(
    State(state): State<AppState>,
    Path(room_id): Path<String>,
) -> Result<Json<Value>, AppError> {
    let room = state
        .room_service
        .get_room_snapshot(&room_id)
        .await?
        .ok_or(AppError::RoomNotFound)?;
    Ok(Json(json!({
        "status": "success",
        "room": room
    })))
}
