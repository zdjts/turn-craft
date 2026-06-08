use crate::network::manager::RoomHandle;
use crate::network::room::{RoomCommand, spawn_game_room};
use crate::{AppState, games::lincoln::create_lincoln};
use axum::extract::Path;
use axum::{Json, extract::State};
use serde::Deserialize; // 导入上一步编写的林肯辩论工厂函数

#[derive(Deserialize)]
pub struct CreateRoomInput {
    pub game_type: String, // 游戏类型标识，例如 "lincoln"
    pub max_round: usize,  // 辩论的最大轮数
    pub player_id: String, // 真人裁判的玩家 ID
}

pub async fn create_room(
    State(state): State<AppState>,
    Json(input): Json<CreateRoomInput>,
) -> Json<serde_json::Value> {
    // 1. 生成唯一的房间 ID
    let room_id = format!(
        "room_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
    );

    tracing::info!(room_id = %room_id, game_type = %input.game_type, "接收到创建房间请求，开始路由工厂...");

    // 2. 【多态路由】根据前端传来的 game_type，动态调用对应的游戏工厂
    let (engine_box, room_state, ai_configs) = match input.game_type.as_str() {
        "lincoln" => create_lincoln(&room_id, &input.player_id, input.max_round),
        _ => {
            tracing::error!(game_type = %input.game_type, "创建房间失败：未知的游戏类型");
            return Json(serde_json::json!({
                "status": "error",
                "message": format!("不支持的游戏类型: {}", input.game_type)
            }));
        }
    };

    // ✨ 3. 【清爽激活】直接调用签名函数。
    // 扔进去引擎、状态和 AI 配置，顺手接过它内部自建通道并吐出来的真正的控制端发送键（room_tx）
    let room_tx = spawn_game_room(
        room_id.clone(),
        engine_box,
        room_state,
        Some(state.ai_tx.clone()),
        ai_configs,
    );

    // 4. 【注册托管】把真正有用的 room_tx 塞进全局管理器
    state.room_manager.rooms.insert(
        room_id.clone(),
        RoomHandle {
            room_id: room_id.clone(),
            tx: room_tx,
        },
    );

    tracing::info!(room_id = %room_id, "房间创建并注册完毕，基础设施层完美闭环");

    Json(serde_json::json!({
        "status": "success",
        "room_id": room_id,
    }))
}

pub async fn delete_room(
    State(state): State<AppState>,
    Path(room_id): Path<String>, // 👈 直接通过 Path 提取器捕获 URL 中的房号
) -> Json<serde_json::Value> {
    tracing::info!(room_id = %room_id, "收到销毁房间请求，开始执行剔除...");

    // 1. 从 DashMap 中原子移除
    if let Some((_, handle)) = state.room_manager.rooms.remove(&room_id) {
        // 2. 向后台常驻的异步大循环投递 Shutdown 指令
        if let Err(e) = handle.tx.send(RoomCommand::Shutdown).await {
            tracing::warn!(room_id = %room_id, error = ?e, "房间协程可能已经提前自行销毁，通道投递失败");
        }

        tracing::info!(room_id = %room_id, "房间已从全局管理器中剥离，Shutdown 信号投递成功");

        Json(serde_json::json!({
            "status": "success",
            "message": format!("房间 {} 销毁流程已成功激活", room_id)
        }))
    } else {
        tracing::warn!(room_id = %room_id, "销毁失败：全局管理器中未找到对应的房间句柄");

        Json(serde_json::json!({
            "status": "error",
            "message": format!("未找到指定的房间 ID: {}", room_id)
        }))
    }
}
