use platform_core::traits::{EngineEvent, GameEngine};
use serde_json::Value;
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

use super::model::{Peer, RoomCommand, RoomSnapshot};
use crate::user::model::UserId;

/// 副作用事件 — Room Actor 不自己处理，发射到外部
pub enum SideEffect {
    TriggerAi {
        actor_id: String,
        snapshot: String,
        tools: Option<Value>,
    },
    PersistSnapshot(RoomSnapshot),
    GameOver,
    /// 所有玩家已离开，开始保活计时
    RoomEmpty,
    /// 有玩家加入（可能为重连）
    PeerJoined,
    /// AI 流式输出的增量片段 — 由 AiWorker 发送
    StreamChunk {
        actor_id: String,
        content: String,
        is_done: bool,
    },
}

/// 启动房间 Actor — 纯游戏循环
///
/// 只做三件事：
/// 1. 等命令
/// 2. 调 engine.step()
/// 3. 广播 + 发射 SideEffect
///
/// 保活、清理、AI 配置查找、持久化全部由外部负责。
pub fn spawn_game_room(
    room_id: String,
    engine: Box<dyn GameEngine>,
    effect_tx: mpsc::Sender<SideEffect>,
) -> mpsc::Sender<RoomCommand> {
    let (tx, mut rx) = mpsc::channel::<RoomCommand>(32);

    tokio::spawn(async move {
        let mut engine = engine;
        let mut peers: Vec<Peer> = Vec::new();

        info!(room_id = %room_id, game_type = %engine.game_type(), "房间 actor 启动");

        while let Some(cmd) = rx.recv().await {
            match cmd {
                RoomCommand::PlayerAction {
                    actor_id,
                    action,
                    feedback_tx,
                } => {
                    let events = match engine.step(&actor_id, action) {
                        Ok(events) => {
                            if let Some(tx) = feedback_tx {
                                let _ = tx.send(Ok(()));
                            }
                            events
                        }
                        Err(e) => {
                            warn!(room_id = %room_id, actor_id = %actor_id, error = %e, "动作执行失败");
                            if let Some(tx) = feedback_tx {
                                let _ = tx.send(Err(e.to_string()));
                            } else if let Some(p) = peers.iter().find(|p| p.actor_id == actor_id) {
                                let _ = p
                                    .tx
                                    .send(serde_json::json!({"error": e.to_string()}).to_string())
                                    .await;
                            }
                            continue;
                        }
                    };

                    // 持久化快照
                    let snapshot = RoomSnapshot {
                        room_id: room_id.clone(),
                        owner_id: UserId(room_id.clone()),
                        game_type: engine.game_type().to_string(),
                        engine_state: engine.to_json(),
                        actor_slots: Vec::new(),
                        ai_configs: Default::default(),
                        max_round: 0,
                        created_at: chrono::Utc::now().naive_utc(),
                        is_public: false,
                    };
                    let _ = effect_tx.send(SideEffect::PersistSnapshot(snapshot)).await;

                    // 广播
                    for p in &peers {
                        let s = engine.to_json_for_player(&p.actor_id).to_string();
                        let _ = p.tx.send(s).await;
                    }

                    // 处理引擎事件
                    for event in events {
                        match event {
                            EngineEvent::TriggerAi(id) => {
                                let _ = effect_tx
                                    .send(SideEffect::TriggerAi {
                                        snapshot: engine.to_ai_prompt(&id),
                                        actor_id: id,
                                        tools: engine.tools(),
                                    })
                                    .await;
                            }
                            EngineEvent::GameOver => {
                                info!(room_id = %room_id, "游戏结束");
                                let _ = effect_tx.send(SideEffect::GameOver).await;
                            }
                            EngineEvent::PrivateMessage { actor_id, payload } => {
                                if let Some(p) = peers.iter().find(|p| p.actor_id == actor_id) {
                                    let _ = p.tx.send(payload.to_string()).await;
                                    debug!(room_id = %room_id, actor_id = %actor_id, "私密消息已发送");
                                }
                            }
                        }
                    }
                }
                RoomCommand::Join(peer) => {
                    let actor_id = peer.actor_id.clone();
                    let was_empty = peers.is_empty();
                    peers.retain(|p| p.actor_id != actor_id);
                    peers.push(peer);
                    info!(room_id = %room_id, actor_id = %actor_id, "选手已连接");

                    if was_empty {
                        let _ = effect_tx.send(SideEffect::PeerJoined).await;
                    }

                    if let Some(p) = peers.iter().find(|p| p.actor_id == actor_id) {
                        let _ =
                            p.tx.send(engine.to_json_for_player(&actor_id).to_string())
                                .await;
                    }
                }
                RoomCommand::Leave(actor_id) => {
                    peers.retain(|p| p.actor_id != actor_id);
                    info!(room_id = %room_id, actor_id = %actor_id, "选手离开房间");

                    if peers.is_empty() {
                        let _ = effect_tx.send(SideEffect::RoomEmpty).await;
                    }
                }
                RoomCommand::Shutdown => {
                    tracing::info!(room_id = %room_id, "房间已主动销毁 (空闲超时)");
                    break;
                }
                RoomCommand::BroadcastStreamChunk { actor_id, content, is_done } => {
                    let msg = if is_done {
                        serde_json::json!({
                            "type": "stream_done",
                            "actor_id": actor_id,
                        })
                    } else {
                        serde_json::json!({
                            "type": "stream_chunk",
                            "actor_id": actor_id,
                            "content": content,
                        })
                    };
                    let msg_str = msg.to_string();
                    for p in &peers {
                        let _ = p.tx.send(msg_str.clone()).await;
                    }
                }
            }
        }

        info!(room_id = %room_id, "房间 actor 已退出");
    });

    tx
}
