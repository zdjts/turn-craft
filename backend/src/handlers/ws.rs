use std::sync::Arc;

use axum::{
    extract::{
        Path, State, WebSocketUpgrade, Query,
        ws::{Message, WebSocket},
    },
    response::IntoResponse,
};
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use tokio::sync::mpsc;

use crate::{
    app::AppState,
    room::model::RoomCommand,
    user::model::UserId,
    error::AppError,
};

#[derive(Deserialize)]
pub struct WsQuery {
    pub token: String,
}

#[derive(Deserialize)]
pub struct ConnectParams {
    pub room_id: String,
    pub actor_id: String,
}

/// WebSocket 升级处理器 (带 Token 验证)
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    Path(params): Path<ConnectParams>,
    Query(query): Query<WsQuery>,
) -> Result<impl IntoResponse, AppError> {
    let user_id = state.auth_service.verify_token(&query.token).await?;
    Ok(ws.on_upgrade(move |socket| {
        handle_socket(socket, state.room_service.clone(), params.room_id, user_id, params.actor_id)
    }))
}

/// 处理 WebSocket 连接：双向数据转发
async fn handle_socket(
    socket: WebSocket,
    room_service: Arc<crate::room::RoomService>,
    room_id: String,
    user_id: UserId,
    actor_id: String,
) {
    let (peer_tx, mut peer_rx) = mpsc::channel::<String>(64);

    // 1. 调用 room_service.connect() 进行鉴权和加入
    if let Err(e) = room_service.connect(user_id, &room_id, &actor_id, peer_tx).await {
        tracing::warn!(room_id = %room_id, actor_id = %actor_id, error = ?e, "连接拒绝：room_service.connect 失败");
        return;
    }

    // 2. 获取 room_tx 用于后续发送上行动作
    let room_tx = match room_service.get_room_tx(&room_id) {
        Some(tx) => tx,
        None => {
            tracing::error!(room_id = %room_id, "找不到房间的通道");
            return;
        }
    };

    let (mut ws_sender, mut ws_receiver) = socket.split();
    tracing::info!(room_id = %room_id, actor_id = %actor_id, "网关与房间会话绑定成功");

    let room_tx_ingress = room_tx.clone();
    let actor_id_ingress = actor_id.clone();
    let room_id_ingress = room_id.clone();

    let mut ingress_task = tokio::spawn(async move {
        while let Some(meg_res) = ws_receiver.next().await {
            match meg_res {
                Ok(Message::Text(text)) => {
                    let action = match serde_json::from_str::<serde_json::Value>(&text) {
                        Ok(val) => val,
                        Err(_) => serde_json::json!({ "content": text.to_string() }),
                    };
                    let cmd = RoomCommand::PlayerAction {
                        actor_id: actor_id_ingress.clone(),
                        action,
                    };
                    if let Err(e) = room_tx_ingress.send(cmd).await {
                        tracing::error!(room_id = %room_id_ingress, actor_id = %actor_id_ingress, error = ?e, "上行数据转发失败，房间已销毁");
                        break;
                    }
                }
                Ok(Message::Close(_)) => {
                    tracing::info!(room_id = %room_id_ingress, actor_id = %actor_id_ingress, "收到客户端主动关闭帧");
                    break;
                }
                Err(e) => {
                    tracing::error!(room_id = %room_id_ingress, actor_id = %actor_id_ingress, error = ?e, "读取网络字节流发生异常");
                    break;
                }
                _ => {}
            }
        }
    });

    let actor_id_egress = actor_id.clone();
    let room_id_egress = room_id.clone();

    let mut egress_task = tokio::spawn(async move {
        while let Some(msg_str) = peer_rx.recv().await {
            if let Err(e) = ws_sender.send(Message::Text(msg_str.into())).await {
                tracing::error!(room_id = %room_id_egress, actor_id = %actor_id_egress, error = ?e, "下发游戏事件失败，连接已断开");
                break;
            }
        }
    });

    tokio::select! {
        _ = &mut ingress_task => {
            tracing::info!(room_id = %room_id, actor_id = %actor_id, "上行链路断开，开始熔断下行任务");
            egress_task.abort();
        }
        _ = &mut egress_task => {
            tracing::info!(room_id = %room_id, actor_id = %actor_id, "下行链路断开，开始熔断上行任务");
            ingress_task.abort();
        }
    }

    tracing::info!(room_id = %room_id, actor_id = %actor_id, "连接彻底断开，向房间发送 Leave 善后命令");
    if let Err(e) = room_tx.send(RoomCommand::Leave(actor_id.clone())).await {
        tracing::error!(room_id = %room_id, actor_id = %actor_id, error = ?e, "善后清理失败，房间可能已提前注销");
    }
}
