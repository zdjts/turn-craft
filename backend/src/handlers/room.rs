use axum::{
    Json,
    extract::{Path, Query, State},
};
use serde::{Deserialize};
use serde_json::{Value, json};

use crate::app::AppState;
use crate::auth::middleware::AuthUser;
use crate::error::AppError;
use crate::room::model::CreateRoomInput;

#[derive(Deserialize)]
pub struct PublicRoomsQuery {
    pub game_type: Option<String>,
    pub page: Option<usize>,
    pub per_page: Option<usize>,
}

fn validate_create_room(input: &CreateRoomInput) -> Result<(), AppError> {
    if input.slots.is_empty() {
        return Err(AppError::BadRequest("slots 不能为空".into()));
    }
    if input.my_slot.is_empty() || (!input.slots.contains(&input.my_slot) && input.my_slot != "spectator") {
        return Err(AppError::BadRequest("my_slot 必须在 slots 列表中".into()));
    }
    if input.max_round == 0 || input.max_round > 1000 {
        return Err(AppError::BadRequest("max_round 必须在 1-1000 之间".into()));
    }
    for slot in &input.slots {
        if !input.slot_configs.contains_key(slot) {
            return Err(AppError::BadRequest(format!("slot_configs 缺少槽位 '{}'", slot)));
        }
    }
    Ok(())
}

/// 创建房间处理器 (薄 Handler)
pub async fn create_room(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Json(input): Json<CreateRoomInput>,
) -> Result<Json<Value>, AppError> {
    validate_create_room(&input)?;
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

/// 获取所有公开的房间列表（支持分页和游戏类型过滤）
pub async fn list_public_rooms(
    State(state): State<AppState>,
    Query(params): Query<PublicRoomsQuery>,
) -> Result<Json<Value>, AppError> {
    let page = params.page.unwrap_or(1);
    let per_page = params.per_page.unwrap_or(20);
    let (rooms, total) = state.room_service
        .list_public_rooms(params.game_type.as_deref(), page, per_page)
        .await?;
    Ok(Json(json!({
        "status": "success",
        "rooms": rooms,
        "total": total,
        "page": page,
        "per_page": per_page,
        "has_next": page * per_page < total,
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

/// 生成房间邀请链接
pub async fn create_invite(
    State(state): State<AppState>,
    AuthUser(_user_id): AuthUser,
    Path(room_id): Path<String>,
) -> Result<Json<Value>, AppError> {
    let room_service = state.room_service.clone();
    let rid = room_id;
    let code = room_service.create_invite(&rid).await?;
    Ok(Json(json!({ "status": "success", "invite_code": code, "invite_link": format!("/invite/{}", code) })))
}

/// 通过邀请码查找房间
pub async fn resolve_invite(
    State(state): State<AppState>,
    Path(code): Path<String>,
) -> Result<Json<Value>, AppError> {
    let pool = &state.room_service.pool;
    let room_id: Option<String> = sqlx::query_scalar("SELECT room_id FROM rooms WHERE invite_code = ?")
        .bind(&code)
        .fetch_optional(pool)
        .await
        .map_err(|e| AppError::Internal(e.into()))?
        .flatten();

    match room_id {
        Some(rid) => Ok(Json(json!({ "status": "success", "room_id": rid }))),
        None => Err(AppError::RoomNotFound),
    }
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

#[derive(serde::Deserialize)]
pub struct JoinRoomInput {
    pub slot_name: String,
}

/// 加入房间并占据一个空槽位
pub async fn join_room(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(room_id): Path<String>,
    Json(input): Json<JoinRoomInput>,
) -> Result<Json<Value>, AppError> {
    state
        .room_service
        .join_slot(user_id, &room_id, &input.slot_name)
        .await?;
    Ok(Json(json!({ "status": "success" })))
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use crate::room::model::CreateRoomInput;

    fn make_input(slots: Vec<&str>, my_slot: &str, max_round: usize) -> CreateRoomInput {
        let mut configs = HashMap::new();
        for s in &slots {
            configs.insert(s.to_string(), "ai".to_string());
        }
        CreateRoomInput {
            game_type: "test".into(),
            max_round,
            my_slot: my_slot.into(),
            slots: slots.into_iter().map(|s| s.to_string()).collect(),
            slot_configs: configs,
            game_config: None,
            is_public: false,
        }
    }

    #[test]
    fn test_validate_empty_slots() {
        let input = make_input(vec![], "a", 10);
        assert!(super::validate_create_room(&input).is_err());
    }

    #[test]
    fn test_validate_my_slot_not_in_slots() {
        let input = make_input(vec!["a", "b"], "c", 10);
        assert!(super::validate_create_room(&input).is_err());
    }

    #[test]
    fn test_validate_max_round_zero() {
        let input = make_input(vec!["a", "b"], "a", 0);
        assert!(super::validate_create_room(&input).is_err());
    }

    #[test]
    fn test_validate_max_round_too_large() {
        let input = make_input(vec!["a", "b"], "a", 1001);
        assert!(super::validate_create_room(&input).is_err());
    }

    #[test]
    fn test_validate_valid_input() {
        let input = make_input(vec!["a", "b"], "a", 10);
        assert!(super::validate_create_room(&input).is_ok());
    }
}
