use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use dashmap::DashMap;
use platform_core::traits::{EngineEvent, GameEngine};
use serde_json::Value;
use tokio::sync::mpsc::{self};
use tracing::{error, info, warn};

use crate::ai::env::AiConfig;

use super::manager::Peer;

/// 断线保活时长：最后一个玩家离开后，房间保留 10 分钟等待重连
const RECONNECT_TIMEOUT: Duration = Duration::from_secs(600);

pub enum RoomCommand {
    PlayerAction { actor_id: String, action: Value },
    Join(Peer),
    Leave(String),
    Shutdown,
}

pub struct AiTask {
    pub room_id: String,
    pub actor_id: String,
    pub snapshot: String,
    pub reply_tx: mpsc::Sender<RoomCommand>,
    pub ai_config: AiConfig,
}

pub fn spawn_game_room(
    room_id: String,
    engine: Box<dyn GameEngine>,
    ai_tx: Option<mpsc::Sender<AiTask>>,
    ai_configs: HashMap<String, AiConfig>,
    global_ai_configs: Option<Arc<DashMap<String, AiConfig>>>,
    rooms_map: Arc<DashMap<String, super::manager::RoomHandle>>,
) -> mpsc::Sender<RoomCommand> {
    let (tx, mut rx) = mpsc::channel::<RoomCommand>(32);
    let room_tx = tx.clone();

    info!(room_id = %room_id, game_type = %engine.game_type(), "创建房间成功");

    tokio::spawn(async move {
        let mut engine = engine;
        let mut peers: Vec<Peer> = Vec::new();
        let mut local_ai_configs = ai_configs;
        // None = 有玩家在线，Some = 所有人断开的时间点
        let mut empty_since: Option<Instant> = None;

        info!(room_id = %room_id, "房间 task 启动，引擎就绪");

        loop {
            // 计算超时等待时间
            let recv_timeout = if let Some(since) = empty_since {
                let elapsed = since.elapsed();
                if elapsed >= RECONNECT_TIMEOUT {
                    info!(room_id = %room_id, "房间空闲超时（{}秒），自动销毁", elapsed.as_secs());
                    break;
                }
                // 保活期间每 60 秒输出一次心跳日志
                let remaining = RECONNECT_TIMEOUT - elapsed;
                let check_interval = Duration::from_secs(60).min(remaining);
                check_interval
            } else {
                // 有玩家在线，无限等待
                Duration::from_secs(3600) // 1小时兜底，防止 recv 永远阻塞
            };

            let cmd = tokio::select! {
                cmd = rx.recv() => {
                    match cmd {
                        Some(c) => c,
                        None => {
                            info!(room_id = %room_id, "所有发送端已关闭，房间退出");
                            break;
                        }
                    }
                }
                _ = tokio::time::sleep(recv_timeout) => {
                    // 超时检查
                    if let Some(since) = empty_since {
                        let elapsed = since.elapsed();
                        if elapsed >= RECONNECT_TIMEOUT {
                            info!(room_id = %room_id, "房间空闲超时，自动销毁");
                            break;
                        }
                        let remaining = RECONNECT_TIMEOUT - elapsed;
                        info!(room_id = %room_id, remaining_secs = remaining.as_secs(), "房间保活中，等待重连...");
                    }
                    continue;
                }
            };

            match cmd {
                RoomCommand::PlayerAction { actor_id, action } => {
                    let events = match engine.step(&actor_id, action) {
                        Ok(events) => events,
                        Err(e) => {
                            warn!(room_id = %room_id, actor_id = %actor_id, error = %e, "动作执行失败");
                            if let Some(p) = peers.iter().find(|p| p.actor_id == actor_id) {
                                let err_msg = serde_json::json!({"error": e}).to_string();
                                let _ = p.tx.send(err_msg).await;
                            }
                            continue;
                        }
                    };

                    info!(room_id = %room_id, actor_id = %actor_id, round = engine.to_json().get("round").and_then(|v| v.as_u64()).unwrap_or(0), "动作已处理");

                    let snapshot = engine.to_json().to_string();
                    for p in &peers {
                        let _ = p.tx.send(snapshot.clone()).await;
                    }

                    for event in events {
                        match event {
                            EngineEvent::TriggerAi(next_actor_id) => {
                                if let Some(ref ai_sender) = ai_tx {
                                    let global_key = format!("{}/{}", room_id, next_actor_id);
                                    let ai_config = if let Some(ref g) = global_ai_configs {
                                        if let Some(cfg) = g.get(&global_key) {
                                            cfg.clone()
                                        } else {
                                            local_ai_configs
                                                .get(&next_actor_id)
                                                .cloned()
                                                .unwrap_or_else(|| AiConfig::new())
                                        }
                                    } else {
                                        local_ai_configs
                                            .get(&next_actor_id)
                                            .cloned()
                                            .unwrap_or_else(|| AiConfig::new())
                                    };
                                    local_ai_configs
                                        .insert(next_actor_id.clone(), ai_config.clone());

                                    let task = AiTask {
                                        room_id: room_id.clone(),
                                        actor_id: next_actor_id,
                                        snapshot: engine.to_json().to_string(),
                                        reply_tx: room_tx.clone(),
                                        ai_config,
                                    };

                                    if let Err(e) = ai_sender.send(task).await {
                                        error!(room_id = %room_id, "AI 调度失败: {:?}", e);
                                    }
                                }
                            }
                            EngineEvent::GameOver => {
                                info!(room_id = %room_id, "游戏结束");
                            }
                        }
                    }
                }
                RoomCommand::Join(peer) => {
                    let actor_id = peer.actor_id.clone();

                    // 有人重连，取消空闲计时
                    if empty_since.take().is_some() {
                        info!(room_id = %room_id, actor_id = %actor_id, "玩家重连，取消房间销毁计时");
                    }

                    peers.retain(|p| p.actor_id != actor_id);
                    peers.push(peer);

                    info!(room_id = %room_id, actor_id = %actor_id, "选手已连接");

                    if let Some(p) = peers.iter().find(|p| p.actor_id == actor_id) {
                        let snapshot = engine.to_json().to_string();
                        let _ = p.tx.send(snapshot).await;
                    }
                }
                RoomCommand::Leave(actor_id) => {
                    peers.retain(|p| p.actor_id != actor_id);
                    info!(room_id = %room_id, actor_id = %actor_id, "选手离开房间");

                    if peers.is_empty() {
                        let now = Instant::now();
                        empty_since = Some(now);
                        info!(
                            room_id = %room_id,
                            timeout_secs = RECONNECT_TIMEOUT.as_secs(),
                            "所有玩家断开，房间进入保活等待（{}秒后自动销毁）",
                            RECONNECT_TIMEOUT.as_secs()
                        );
                    }
                }
                RoomCommand::Shutdown => {
                    info!(room_id = %room_id, "收到 Shutdown 命令，正在清理");
                    for p in &peers {
                        let _ = p
                            .tx
                            .send(r#"{"event":"room_closed"}"#.to_string())
                            .await;
                    }
                    break;
                }
            }
        }

        // 从 RoomManager 中移除自身，避免残留句柄
        rooms_map.remove(&room_id);
        info!(room_id = %room_id, "房间 task 已退出，已从 RoomManager 清除");
    });

    tx
}
