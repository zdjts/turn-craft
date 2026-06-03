use std::{fmt::Debug, marker::PhantomData};

use platform_core::{
    // games::lincoln::{self, DebatAction, DebatRole, DebatRoomState, LincolnPayload},
    traits::{GameAction, GameEvent, GameRole, Payload, Playable, RoomState},
};
use tokio::sync::mpsc::{self};
use tracing::{error, info, warn};

use super::manager::Peer;

pub enum RoomCommand {
    PlayerAction { actor_id: String, action: String },
    Join(Peer),
    Leave(String),
    Shutdown,
}

pub struct AiTask {
    pub room_id: String,
    pub actor_id: String,
    pub snapshot: String,
    pub reply_tx: mpsc::Sender<RoomCommand>,
}

pub struct Room<R: GameRole, A: GameAction, P: Payload, E: Debug> {
    pub room_id: String,
    pub engine: Box<dyn Playable<R, A, P, E>>,
    pub state: RoomState<R, A>,
    pub peers: Vec<Peer>,
    pub _marker: PhantomData<E>,
}

pub fn spawn_game_room<R, A, P, E>(
    room_id: String,
    engine: Box<dyn Playable<R, A, P, E>>,
    state: RoomState<R, A>,
    ai_tx: Option<mpsc::Sender<AiTask>>,
) -> mpsc::Sender<RoomCommand>
where
    R: GameRole + 'static,
    A: GameAction + 'static,
    P: Payload + 'static,
    E: Debug + 'static + Send,
{
    let (tx, mut rx) = mpsc::channel::<RoomCommand>(32);
    let room_tx = tx.clone();

    info!(room_id = %room_id, "创建房间成功");

    tokio::spawn(async move {
        let mut room = Room {
            room_id: room_id.clone(),
            engine,
            state,
            peers: Vec::new(),
            _marker: PhantomData,
        };

        // 预先收集 AI actor IDs，用于 Join 时验证
        let ai_actor_ids: Vec<_> = room
            .state
            .actors
            .iter()
            .filter(|a| matches!(a.kind, platform_core::traits::ActionKind::Ai))
            .map(|a| a.id.clone())
            .collect();

        info!(
            room_id = %room_id,
            actors = ?room.state.actors.iter().map(|a| &a.id).collect::<Vec<_>>(),
            "房间 task 启动，演员已就绪"
        );

        while let Some(cmd) = rx.recv().await {
            match cmd {
                RoomCommand::PlayerAction { actor_id, action } => {
                    // 拒绝未注册用户
                    if !room.state.actors.iter().any(|a| a.id == actor_id) {
                        warn!(room_id = %room_id, actor_id = %actor_id, "拒绝动作：未注册的身份");
                        continue;
                    }

                    // 解析动作
                    let parsed_action = match room.engine.parse_action(&action) {
                        Ok(a) => a,
                        Err(e) => {
                            warn!(room_id = %room_id, actor_id = %actor_id, error = ?e, "动作解析失败");
                            if let Some(p) = room.peers.iter().find(|p| p.actor_id == actor_id) {
                                let _ = p.tx.send("invalid_action".to_string());
                            }
                            continue;
                        }
                    };

                    // 执行动作并获取事件
                    let events = match room.engine.step(&mut room.state, parsed_action) {
                        Ok(events) => events,
                        Err(e) => {
                            warn!(room_id = %room_id, actor_id = %actor_id, error = ?e, "动作执行失败");
                            if let Some(p) = room.peers.iter().find(|p| p.actor_id == actor_id) {
                                let _ = p.tx.send("action_failed".to_string());
                            }
                            continue;
                        }
                    };

                    let round = room.state.history.len();
                    info!(room_id = %room_id, actor_id = %actor_id, round = round, "发言被写入历史");

                    // 处理所有事件
                    for event in events {
                        match event {
                            GameEvent::Broadcast(msg) => {
                                for p in &room.peers {
                                    let _ = p.tx.send(msg.clone());
                                }
                            }
                            GameEvent::TriggerAi(next_actor_id) => {
                                if let Some(ref ai_sender) = ai_tx {
                                    // 获取当前状态的快照
                                    let next_role = room
                                        .state
                                        .actors
                                        .iter()
                                        .find(|a| a.id == next_actor_id)
                                        .map(|a| a.role.clone());

                                    let role = next_role.unwrap_or_else(|| {
                                        // 如果找不到角色，使用默认值（需要 R: Default 或者其他方式）
                                        // 这里暂时用 panic 或者处理不了的情况
                                        panic!("Cannot determine role for AI actor");
                                    });

                                    let snapshot = room.engine.get_snapshot(&room.state, &role);

                                    info!(room_id = %room_id, ai_actor_id = %next_actor_id, "自动触发：开始向 AI 总线派发任务");

                                    let task = AiTask {
                                        room_id: room_id.clone(),
                                        actor_id: next_actor_id,
                                        snapshot,
                                        reply_tx: room_tx.clone(),
                                    };

                                    if let Err(e) = ai_sender.send(task).await {
                                        error!(room_id = %room_id, "AI 调度失败: {:?}", e);
                                    }
                                }
                            }
                            GameEvent::GameOver => {
                                info!(room_id = %room_id, "游戏结束");
                                for p in &room.peers {
                                    let _ = p.tx.send("game_over".to_string());
                                }
                            }
                            GameEvent::Custom(_) => {}
                            GameEvent::NotifyRole { role, payload } => {
                                // 通知特定角色
                                for p in &room.peers {
                                    if let Some(actor) =
                                        room.state.actors.iter().find(|a| a.id == p.actor_id)
                                    {
                                        if actor.role == role {
                                            let _ = p.tx.send(payload.clone());
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                RoomCommand::Join(peer) => {
                    let actor_id = peer.actor_id.clone();

                    // 拒绝 AI actor 通过 Join 方式加入
                    if ai_actor_ids.contains(&actor_id) {
                        warn!(room_id = %room_id, actor_id = %actor_id, "拒绝加入：AI 角色不能通过此方式加入");
                        let _ = peer.tx.send("not_a_player_in_this_room".to_string());
                        continue;
                    }

                    // 拒绝不是本场比赛的选手
                    let is_legal_actor = room.state.actors.iter().any(|a| a.id == actor_id);
                    if !is_legal_actor {
                        warn!(room_id = %room_id, actor_id = %actor_id, "拒绝加入：不是本场比赛的选手");
                        let _ = peer.tx.send("not_a_player_in_this_room".to_string());
                        continue;
                    }

                    // 清除当前连接的用户，防止断线重连失败
                    room.peers.retain(|p| p.actor_id != actor_id);
                    room.peers.push(peer);

                    info!(room_id = %room_id, actor_id = %actor_id, "选手网络连接已就绪");

                    for p in &room.peers {
                        let _ = p.tx.send(format!("joined:{actor_id}"));
                    }
                }
                RoomCommand::Leave(actor_id) => {
                    room.peers.retain(|p| p.actor_id != actor_id);
                    room.state.actors.retain(|a| a.id != actor_id);

                    info!(room_id = %room_id, actor_id = %actor_id, "选手离开房间");

                    for p in &room.peers {
                        let _ = p.tx.send(format!("left:{actor_id}"));
                    }

                    if room.peers.is_empty() {
                        info!(room_id = %room_id, "所有玩家断开，房间自动销毁");
                        break;
                    }
                }
                RoomCommand::Shutdown => {
                    info!(room_id = %room_id, "收到 Shutdown 命令，正在清理");
                    for p in &room.peers {
                        let _ = p.tx.send("room_closed".to_string());
                    }
                    break;
                }
            }
        }
    });

    tx
}
