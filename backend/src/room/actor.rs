use platform_core::traits::{EngineEvent, GameEngine};
use serde_json::Value;
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

use super::model::{Peer, RoomCommand};

pub enum SideEffect {
    TriggerAi {
        actor_id: String,
        snapshot: String,
        tools: Option<Value>,
    },
    SaveEngineState {
        room_id: String,
        engine_state: Value,
    },
    GameOver,
    RoomEmpty,
    PeerJoined,
    StreamChunk {
        actor_id: String,
        content: String,
        is_done: bool,
    },
    AppendEvent {
        #[allow(dead_code)]
        room_id: String,
        event_type: String,
        actor_id: String,
        payload: Value,
    },
    AiFailed {
        actor_id: String,
        error: String,
    },
}

const DEFAULT_TURN_TIMEOUT_SECS: u64 = 120;
const MAX_AI_RETRIES: u32 = 3;

/// AI 任务严格状态机
#[derive(Debug, Clone, PartialEq)]
enum AiState {
    Idle,
    WaitingForAi {
        actor_id: String,
        retries: u32,
    },
    AiFailed {
        actor_id: String,
        error: String,
        retries: u32,
    },
}

impl AiState {
    fn actor_id(&self) -> Option<&str> {
        match self {
            AiState::Idle => None,
            AiState::WaitingForAi { actor_id, .. } => Some(actor_id),
            AiState::AiFailed { actor_id, .. } => Some(actor_id),
        }
    }
}

pub fn spawn_game_room(
    room_id: String,
    engine: Box<dyn GameEngine>,
    effect_tx: mpsc::Sender<SideEffect>,
) -> mpsc::Sender<RoomCommand> {
    let (tx, mut rx) = mpsc::channel::<RoomCommand>(32);

    tokio::spawn(async move {
        let mut engine = engine;
        let mut peers: Vec<Peer> = Vec::new();
        let mut ai_state = AiState::Idle;

        info!(room_id = %room_id, game_type = %engine.game_type(), "房间 actor 启动");
        let timeout_dur = std::time::Duration::from_secs(DEFAULT_TURN_TIMEOUT_SECS);

        loop {
            // 推进状态机：无待办且引擎有下一个 actor → 进入等待
            if ai_state == AiState::Idle && !engine.is_finished() {
                if let Some(next) = engine.current_actor() {
                    ai_state = AiState::WaitingForAi {
                        actor_id: next,
                        retries: 0,
                    };
                }
            }

            let cmd = if let Some(ref actor_id) = ai_state.actor_id() {
                match tokio::time::timeout(timeout_dur, rx.recv()).await {
                    Ok(Some(cmd)) => cmd,
                    Ok(None) => break,
                    Err(_elapsed) => {
                        warn!(room_id = %room_id, actor_id = %actor_id, "回合超时，自动 skip");
                        let timeout_action = serde_json::json!({"action": "timeout"});
                        match engine.step(actor_id, timeout_action) {
                            Ok(events) => {
                                broadcast_and_handle_events(
                                    &engine, &peers, &room_id, &effect_tx, events,
                                ).await;
                            }
                            Err(e) => {
                                warn!(room_id = %room_id, error = %e, "超时 action 执行失败");
                            }
                        }
                        ai_state = AiState::Idle;
                        continue;
                    }
                }
            } else {
                match rx.recv().await {
                    Some(cmd) => cmd,
                    None => break,
                }
            };

            match cmd {
                RoomCommand::Shutdown => break,
                RoomCommand::PlayerAction { actor_id, action, feedback_tx } => {
                    let is_expected = ai_state.actor_id() == Some(&actor_id);
                    match engine.step(&actor_id, action.clone()) {
                        Ok(events) => {
                            if let Some(tx) = feedback_tx { let _ = tx.send(Ok(())); }
                            ai_state = AiState::Idle;
                            let _ = effect_tx.send(SideEffect::AppendEvent {
                                room_id: room_id.clone(),
                                event_type: "action".to_string(),
                                actor_id: actor_id.clone(),
                                payload: action,
                            }).await;
                            broadcast_and_handle_events(&engine, &peers, &room_id, &effect_tx, events).await;
                        }
                        Err(e) => {
                            warn!(room_id = %room_id, actor_id = %actor_id, error = %e, "动作执行失败");
                            if let Some(tx) = feedback_tx {
                                let _ = tx.send(Err(e.to_string()));
                            } else if let Some(p) = peers.iter().find(|p| p.actor_id == actor_id) {
                                let _ = p.tx.send(serde_json::json!({
                                    "type": "action_error",
                                    "actor_id": actor_id,
                                    "error": e.to_string(),
                                    "can_retry": is_expected,
                                }).to_string()).await;
                            }
                            // 状态转移：WaitingForAi → AiFailed
                            let retries = match &ai_state {
                                AiState::WaitingForAi { retries, .. } => *retries + 1,
                                _ => 1,
                            };
            if is_expected && retries <= MAX_AI_RETRIES {
                ai_state = AiState::AiFailed {
                    actor_id: actor_id.clone(),
                    error: e.to_string(),
                    retries,
                };
                let _ = effect_tx.send(SideEffect::AppendEvent {
                    room_id: room_id.clone(),
                    event_type: "ai_failed".to_string(),
                    actor_id: actor_id.clone(),
                    payload: serde_json::json!({ "error": e.to_string(), "retries": retries }),
                }).await;
                let _ = effect_tx.send(SideEffect::AiFailed {
                    actor_id: actor_id.clone(),
                    error: e.to_string(),
                }).await;
            } else {
                // 非预期的 actor 或重试用尽 → 强制 idle 防止死锁
                ai_state = AiState::Idle;
            }
                        }
                    }
                }
                RoomCommand::RetryAi { actor_id } => {
                    info!(room_id = %room_id, actor_id = %actor_id, "手动/自动重试 AI");
                    // AiFailed → trigger new AI
                    match &ai_state {
                        AiState::AiFailed { actor_id: aid, retries, .. } if aid == &actor_id => {
                            let retries = *retries;
                            let prompt = engine.to_ai_prompt(&actor_id);
                            let tools = engine.tools();
                            let _ = effect_tx.send(SideEffect::TriggerAi {
                                snapshot: prompt,
                                actor_id: actor_id.clone(),
                                tools,
                            }).await;
                            ai_state = AiState::WaitingForAi { actor_id, retries };
                        }
                        _ => {
                            // 不在失败态，忽略
                            warn!(room_id = %room_id, actor_id = %actor_id, "无法重试：当前状态不是 AiFailed");
                        }
                    }
                }
                RoomCommand::SkipAiTurn { actor_id } => {
                    info!(room_id = %room_id, actor_id = %actor_id, "跳过 AI 回合");
                    let skip_action = serde_json::json!({"action": "skip", "content": "[回合跳过]"});
                    match engine.step(&actor_id, skip_action) {
                        Ok(events) => {
                            ai_state = AiState::Idle;
                            broadcast_and_handle_events(&engine, &peers, &room_id, &effect_tx, events).await;
                        }
                        Err(e) => {
                            warn!(room_id = %room_id, actor_id = %actor_id, error = %e, "跳过动作执行失败");
                            ai_state = AiState::Idle;
                        }
                    }
                }
                RoomCommand::Join(peer) => {
                    let actor_id = peer.actor_id.clone();
                    let was_empty = peers.is_empty();
                    peers.retain(|p| p.actor_id != actor_id);
                    peers.push(peer);
                    info!(room_id = %room_id, actor_id = %actor_id, "选手已连接");
                    if was_empty { let _ = effect_tx.send(SideEffect::PeerJoined).await; }
                    let _ = effect_tx.send(SideEffect::AppendEvent {
                        room_id: room_id.clone(),
                        event_type: "player_joined".to_string(),
                        actor_id: actor_id.clone(),
                        payload: Value::Null,
                    }).await;
                    if let Some(p) = peers.iter().find(|p| p.actor_id == actor_id) {
                        let _ = p.tx.send(engine.to_json_for_player(&actor_id).to_string()).await;
                    }
                }
                RoomCommand::Leave(actor_id) => {
                    peers.retain(|p| p.actor_id != actor_id);
                    info!(room_id = %room_id, actor_id = %actor_id, "选手离开房间");
                    let _ = effect_tx.send(SideEffect::AppendEvent {
                        room_id: room_id.clone(),
                        event_type: "player_left".to_string(),
                        actor_id: actor_id.clone(),
                        payload: Value::Null,
                    }).await;
                    if peers.is_empty() { let _ = effect_tx.send(SideEffect::RoomEmpty).await; }
                }
                RoomCommand::BroadcastStreamChunk { actor_id, content, is_done } => {
                    let msg = if is_done {
                        serde_json::json!({ "type": "stream_done", "actor_id": actor_id })
                    } else {
                        serde_json::json!({ "type": "stream_chunk", "actor_id": actor_id, "content": content })
                    };
                    let msg_str = msg.to_string();
                    for p in peers.iter() {
                        let _ = p.tx.send(msg_str.clone()).await;
                    }
                }
                RoomCommand::SlotOccupied { slot_name, user_id: _ } => {
                    info!(room_id = %room_id, slot_name = %slot_name, "槽位被用户占据");
                }
                RoomCommand::SlotReleased { slot_name } => {
                    info!(room_id = %room_id, slot_name = %slot_name, "槽位被释放");
                }
            }
        }

        info!(room_id = %room_id, "房间 actor 已退出");
    });

    tx
}

async fn broadcast_and_handle_events(
    engine: &Box<dyn GameEngine>,
    peers: &[Peer],
    room_id: &str,
    effect_tx: &mpsc::Sender<SideEffect>,
    events: Vec<EngineEvent>,
) {
    let engine_state = engine.to_json();
    let player_states: Vec<(String, String)> = peers
        .iter()
        .map(|p| (p.actor_id.clone(), engine.to_json_for_player(&p.actor_id).to_string()))
        .collect();
    let tools = engine.tools();

    let _ = effect_tx.send(SideEffect::SaveEngineState {
        room_id: room_id.to_string(),
        engine_state: engine_state.clone(),
    }).await;

    let _ = effect_tx.send(SideEffect::AppendEvent {
        room_id: room_id.to_string(),
        event_type: "state_change".to_string(),
        actor_id: String::new(),
        payload: engine_state,
    }).await;

    for (actor_id, state_str) in &player_states {
        if let Some(p) = peers.iter().find(|p| &p.actor_id == actor_id) {
            let _ = p.tx.send(state_str.clone()).await;
        }
    }

    for event in events {
        match event {
            EngineEvent::TriggerAi(id) => {
                let prompt = engine.to_ai_prompt(&id);
                let _ = effect_tx.send(SideEffect::AppendEvent {
                    room_id: room_id.to_string(),
                    event_type: "ai_trigger".to_string(),
                    actor_id: id.clone(),
                    payload: Value::Null,
                }).await;
                let _ = effect_tx
                    .send(SideEffect::TriggerAi {
                        snapshot: prompt,
                        actor_id: id,
                        tools: tools.clone(),
                    })
                    .await;
            }
            EngineEvent::GameOver => {
                info!(room_id = %room_id, "游戏结束");
                let _ = effect_tx.send(SideEffect::AppendEvent {
                    room_id: room_id.to_string(),
                    event_type: "game_over".to_string(),
                    actor_id: String::new(),
                    payload: Value::Null,
                }).await;
                let _ = effect_tx.send(SideEffect::GameOver).await;
            }
            EngineEvent::PrivateMessage { actor_id, payload } => {
                if let Some(p) = peers.iter().find(|p| p.actor_id == actor_id) {
                    let _ = p.tx.send(payload.to_string()).await;
                    debug!(room_id = %room_id, actor_id = %actor_id, "私密消息已发送");
                }
            }
            EngineEvent::PlayerJoined(_) | EngineEvent::PlayerLeft(_) => {}
        }
    }
}
