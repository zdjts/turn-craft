use platform_core::{
    games::lincoln::{DebatAction, DebatRole, DebatRoomState},
    traits::{Actor, GameAction, GameRole, Playable, RoomState},
};
use tokio::sync::mpsc::{self, Sender};
use tracing::{debug, error, info, warn};

use super::manager::{self, Peer, RoomManager};
pub type LincolnRoom = Room<DebatRole, DebatAction>;

pub enum RoomCommand {
    PlayerAction { actor_id: String, content: String },
    Join(Peer),
    Leave(String),
    Shutdown,
}

pub struct AiTask<A: GameAction> {
    pub room_id: String,
    pub actor_id: String,
    pub history: Vec<A>,
    pub reply_tx: mpsc::Sender<RoomCommand>,
}

pub struct Room<R: GameRole, A: GameAction> {
    pub room_id: String,
    pub engine: Box<dyn Playable<R, A> + Send + Sync>,
    pub state: RoomState<R, A>,
    pub peers: Vec<Peer>,
}
pub fn spawn_lincoln_room(
    room_id: String,
    engine: Box<dyn Playable<DebatRole, DebatAction> + Send + Sync>,
    state: DebatRoomState,
    manager: RoomManager,
    ai_tx: Option<mpsc::Sender<AiTask<DebatAction>>>,
) -> Sender<RoomCommand> {
    let (tx, mut rx) = mpsc::channel::<RoomCommand>(32);
    let room_tx = tx.clone();
    tokio::spawn(async move {
        let room_id_for_cleanup = room_id.clone();

        let ai_actor_ids: Vec<_> = state
            .actors
            .iter()
            .filter(|a| matches!(a.kind, platform_core::traits::ActionKind::Ai))
            .map(|a| a.id.clone())
            .collect();
        let mut room = LincolnRoom {
            room_id: room_id.clone(),
            engine,
            state,
            peers: Vec::new(),
        };
        info!(room_id = %room_id, actors = ?room.state.actors.iter().map(|a| &a.id).collect::<Vec<_>>(), "林肯 task 启动，演员以就绪");

        while let Some(cmd) = rx.recv().await {
            match cmd {
                RoomCommand::PlayerAction { actor_id, content } => {
                    // 拒绝未注册用户
                    if !room.state.actors.iter().any(|a| a.id == actor_id) {
                        warn!(room_id = %room_id, actor_id = %actor_id, "拒绝动作： 未注册的身份");
                        continue;
                    }

                    let action = DebatAction::Speech {
                        action_id: actor_id.clone(),
                        content: content.clone(),
                    };
                    if !room.engine.validata_action(&room.state, &action) {
                        warn!(room_id = %room_id, actor_id = %actor_id, "动作校验失败（非本角色的回合)");
                        if let Some(p) = room.peers.iter().find(|p| p.actor_id == actor_id) {
                            let _ = p.tx.send("invalid_action".to_string());
                        }
                        continue;
                    }
                    let round = room.state.history.len() + 1;
                    info!(room_id = %room_id, actor_id = %actor_id, round = round, "发言被写入历史");
                    // 引擎推动状态机的改变，写入辩论历史
                    room.engine.apply_action(&mut room.state, action);
                    let payload = format!("{actor_id}: {content}");
                    for p in &room.peers {
                        p.tx.send(payload.clone());
                    }
                    match room.engine.get_next_role() {
                        DebatRole::Over => {
                            info!(room_id = %room_id, "本场辩论赛结束");
                            for p in &room.peers {
                                let _ = p.tx.send("game_over".to_string());
                            }
                        }
                        next_role => {
                            let next_actor = room.state.actors.iter().find(|a| a.role == next_role);
                            if let Some(actor) = next_actor {
                                debug!(room_id = %room_id, next_actor_id = %actor.id, "下一回合准备就绪");

                                match actor.kind {
                                    platform_core::traits::ActionKind::Ai => {
                                        if let Some(ref ai_sender) = ai_tx {
                                            info!(room_id = %room_id, ai_actor_id = %actor.id, "自动触发：开始向 AI 总线派发任务");
                                            let task = AiTask::<DebatAction> {
                                                room_id: room_id.clone(),
                                                actor_id: actor_id.clone(),
                                                history: room.state.history.clone(),
                                                reply_tx: room_tx.clone(),
                                            };
                                            let sender_num = ai_sender.send(task).await;
                                            if sender_num.is_err() {
                                                error!(room_id = %room_id, "Ai 调度已关闭")
                                            }
                                        }
                                    }
                                    platform_core::traits::ActionKind::Human => {
                                        debug!(room_id = %room_id, "正在等待人类晚间发言");
                                    }
                                }
                            } else {
                                warn!(room_id = %room_id, next_role = ?next_role, "找不到下一个行动的 actor");
                            }
                        }
                    }
                    room.engine.set_next_role();
                }
                RoomCommand::Join(peer) => {
                    // 拒绝Join方式连接的符合Ai的用户，防止伪造Ai id 通过。
                    let actor_id = peer.actor_id.clone();
                    if ai_actor_ids.contains(&actor_id) {
                        warn!(room_id = %room_id, actor_id = %actor_id, "拒绝加入: 你不是本场的选手");
                        let _ = peer.tx.send("not_a_player_in_this_room".to_string());
                        continue;
                    }
                    // 拒绝不是本场比赛的选手
                    let is_legal_actor = room.state.actors.iter().any(|a| a.id == actor_id);
                    if !is_legal_actor {
                        warn!(room_id = %room_id, actor_id = %actor_id, "拒绝加入:不是本场比赛的选手");
                        let _ = peer.tx.send(("not_a_player_in_this_room".to_string()));
                    }

                    // 清除当前进入的用户，防止短线重联进不去
                    room.peers.retain(|p| p.actor_id != actor_id);
                    room.peers.push(peer);
                    info!(room_id = %room_id, actor_id = %actor_id, "选手网络连接以就绪");

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
                    info!(room_id = %room_id, "收到 Shutdown 命令， 正在清理");
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
// pub fn spawn_lincoln_room(

// pub fn spawn_lincoln_room(
//     room_id: String,
//     engine: Box<dyn Playable<DebatRole, DebatAction> + Send + Sync>,
//     state: DebatRoomState,
//     first_side: DebatRole,
//     manager: RoomManager,
//     ai_tx: Option<mpsc::Sender<AiTask<DebatAction>>>,
// ) -> mpsc::Sender<RoomCommand> {
//     let (tx, mut rx) = mpsc::channel::<RoomCommand>(32);
//     let room_tx = tx.clone();
//
//     tokio::spawn(async move {
//         let room_id_for_cleanup = room_id.clone();
//         let ai_actor_id = ai_tx.as_ref().map(|_| format!("P{room_id}::ai"));
//         let mut room = LincolnRoom {
//             room_id: room_id.clone(),
//             engine,
//             state,
//             peers: Vec::new(),
//         };
//         if let Some(ai_actor_id) = ai_actor_id {
//             room.state.actors.push(Actor {
//                 id: ai_actor_id.clone(),
//                 kind: platform_core::traits::ActionKind::Ai,
//                 role: first_side,
//             });
//             info!(room_id = %room_id, ai_actor_id = %ai_actor_id, "AI角色已注册");
//         }
//         info!(room_id = %room_id, first_side = ?first_side, "房间 task 启动");
//         while let Some(cmd) = rx.recv().await {
//             match cmd {
//                 RoomCommand::Join(peer) => {
//                     let actor_id = peer.actor_id.clone();
//
//                     if let Some(ref ai_actor_id) = ai_actor_id {
//                         if actor_id == *ai_actor_id {
//                             warn!(room_id = %room_id, actor_id = %actor_id, "actor_id 与 AI保留 ID有冲突");
//
//                             let _ = peer.tx.send("reserved_actor_id".to_string());
//                         }
//                     }
//                 }
//                 RoomCommand::PlayerAction { actor_id, content } => todo!(),
//                 RoomCommand::Leave(_) => todo!(),
//                 RoomCommand::Shutdown => todo!(),
//             }
//         }
//     });
//     todo!()
// }
