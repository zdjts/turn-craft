use std::collections::HashMap;

use crate::network::manager::RoomHandle;
use crate::network::room::{RoomCommand, spawn_game_room};
use crate::persistence::{self, RoomSnapshot};
use crate::{AppState, games::lincoln::create_lincoln};
use axum::extract::Path;
use axum::{Json, extract::State};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct CreateRoomInput {
    pub game_type: String,
    pub max_round: usize,
    pub my_role: String,
    pub role_config: HashMap<String, String>,
}

pub async fn create_room(
    State(state): State<AppState>,
    Json(input): Json<CreateRoomInput>,
) -> Json<serde_json::Value> {
    let room_id = format!(
        "room_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
    );

    tracing::info!(
        room_id = %room_id,
        game_type = %input.game_type,
        my_role = %input.my_role,
        role_config = ?input.role_config,
        "接收到创建房间请求，开始路由工厂..."
    );

    let (engine_box, ai_configs) = match input.game_type.as_str() {
        "lincoln" => create_lincoln(
            &room_id,
            &input.my_role,
            &input.role_config,
            input.max_round,
            Some(&state.ai_configs),
        ),
        _ => {
            tracing::error!(game_type = %input.game_type, "创建房间失败：未知的游戏类型");
            return Json(serde_json::json!({
                "status": "error",
                "message": format!("不支持的游戏类型: {}", input.game_type)
            }));
        }
    };

    // 注册 AI 配置到全局存储（DashMap 自身并发安全，无需额外锁）
    for (actor_id, cfg) in &ai_configs {
        state
            .ai_configs
            .insert(format!("{}/{}", room_id, actor_id), cfg.clone());
    }

    // 持久化到文件（新房间的 AI 配置也会保存）
    crate::save_configs_to_file(&state.ai_configs);

    // 保存初始房间快照
    let initial_snapshot = RoomSnapshot {
        room_id: room_id.clone(),
        game_type: engine_box.game_type().to_string(),
        engine_state: engine_box.to_json(),
        role_config: input.role_config.clone(),
        ai_configs: ai_configs.clone(),
        max_round: input.max_round,
    };
    persistence::save_room_snapshot(&state.snapshots, &room_id, initial_snapshot);

    let room_tx = spawn_game_room(
        room_id.clone(),
        engine_box,
        Some(state.ai_tx.clone()),
        ai_configs,
        Some(state.ai_configs.clone()),
        state.room_manager.rooms.clone(),
        state.snapshots.clone(),
        input.role_config.clone(),
        false,
    );

    state.room_manager.rooms.insert(
        room_id.clone(),
        RoomHandle {
            room_id: room_id.clone(),
            tx: room_tx,
        },
    );

    let creator_actor_id = determine_actor_id(&input.my_role, &input.role_config);

    tracing::info!(room_id = %room_id, creator_actor_id = %creator_actor_id, "房间创建并注册完毕");

    Json(serde_json::json!({
        "status": "success",
        "room_id": room_id,
        "actor_id": creator_actor_id,
    }))
}

fn determine_actor_id(my_role: &str, role_config: &HashMap<String, String>) -> String {
    match role_config.get(my_role).map(|s| s.as_str()) {
        Some("human") => my_role.to_string(),
        _ => format!("human_{}", my_role.to_lowercase()),
    }
}

pub async fn delete_room(
    State(state): State<AppState>,
    Path(room_id): Path<String>,
) -> Json<serde_json::Value> {
    tracing::info!(room_id = %room_id, "收到销毁房间请求");

    // 清理 AI 配置（DashMap 自身并发安全）
    let prefix = format!("{}/", room_id);
    state
        .ai_configs
        .retain(|key, _| !key.starts_with(&prefix));

    if let Some((_, handle)) = state.room_manager.rooms.remove(&room_id) {
        if let Err(e) = handle.tx.send(RoomCommand::Shutdown).await {
            tracing::warn!(room_id = %room_id, error = ?e, "房间协程可能已提前销毁");
        }
        tracing::info!(room_id = %room_id, "房间已销毁");
        Json(serde_json::json!({
            "status": "success",
            "message": format!("房间 {} 销毁成功", room_id)
        }))
    } else {
        Json(serde_json::json!({
            "status": "error",
            "message": format!("未找到房间: {}", room_id)
        }))
    }
}
