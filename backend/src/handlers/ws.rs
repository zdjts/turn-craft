use std::sync::Arc;

use axum::{
    extract::{
        Path, State, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    response::IntoResponse,
};
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use tokio::sync::mpsc;

use crate::{
    app::AppState,
    network::{
        manager::{Peer, RoomManager},
        room::RoomCommand,
    },
};

/// WebSocket 连接参数
#[derive(Deserialize)]
pub struct ConnectParams {
    pub room_id: String,
    pub actor_id: String,
}

/// WebSocket 升级处理器
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(app): State<AppState>,
    Path(params): Path<ConnectParams>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| {
        handle_socket(socket, app.room_manager, params.room_id, params.actor_id)
    })
}

/// 处理 WebSocket 连接：双向数据转发
async fn handle_socket(
    socket: WebSocket,
    room_manager: Arc<RoomManager>,
    room_id: String,
    actor_id: String,
) {
    let room_tx = {
        if let Some(handle) = room_manager.rooms.get(&room_id) {
            handle.tx.clone()
        } else {
            tracing::warn!(room_id = %room_id, actor_id = %actor_id, "拒绝连接：目标房间不存在");
            return;
        }
    };

    let (mut ws_sender, mut ws_receiver) = socket.split();
    let (peer_tx, mut peer_rx) = mpsc::channel::<String>(64);
    let peer = Peer {
        actor_id: actor_id.clone(),
        tx: peer_tx,
    };
    if let Err(e) = room_tx.send(RoomCommand::Join(peer)).await {
        tracing::error!(room_id = %room_id, actor_id = %actor_id, error = ?e, "加入房间失败，控制总线已关闭");
        return;
    };
    tracing::info!(room_id = %room_id, actor_id = %actor_id, "网关与房间会话绑定成功");

    let room_tx_ingress = room_tx.clone();
    let actor_id_ingress = actor_id.clone();
    let room_id_ingress = room_id.clone();

    let mut ingress_task = tokio::spawn(async move {
        while let Some(meg_res) = ws_receiver.next().await {
            match meg_res {
                Ok(Message::Text(text)) => {
                    // 尝试解析为 JSON；如果不是合法 JSON，则包装为 {"content": text}
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
