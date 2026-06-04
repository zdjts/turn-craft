use backend::network::room::*;
use platform_core::games::lincoln::*;
use platform_core::traits::*;
use tokio::sync::mpsc;
use tokio::time::{timeout, Duration};

#[derive(Debug)]
struct MockEngine {
    cur_role: DebatRole,
    round: usize,
    max_round: usize,
}

impl Playable<DebatRole, DebatAction, LincolnPayload, LincolnErr> for MockEngine {
    fn parse_action(&self, raw_content: &str) -> Result<DebatAction, LincolnErr> {
        if raw_content == "valid" {
            Ok(DebatAction::Speech {
                action_id: "actor1".to_string(),
                content: "hello".to_string(),
            })
        } else {
            Err(LincolnErr::InvalidProtocol)
        }
    }

    fn step(
        &mut self,
        state: &mut RoomState<DebatRole, DebatAction>,
        action: DebatAction,
    ) -> Result<Vec<GameEvent<DebatRole, LincolnPayload>>, LincolnErr> {
        let DebatAction::Speech { action_id, content } = action.clone();
        let Some(actor) = state.find_actor(&action_id) else {
            return Err(LincolnErr::NotActor);
        };
        if actor.role != self.cur_role {
            return Err(LincolnErr::NotYourTurn);
        }
        state.history.push(action);
        match self.cur_role {
            DebatRole::Pro | DebatRole::Con => {
                self.round += 1;
                if self.round >= self.max_round {
                    self.cur_role = DebatRole::Judge;
                } else {
                    self.cur_role = match self.cur_role {
                        DebatRole::Pro => DebatRole::Con,
                        _ => DebatRole::Pro,
                    };
                }
            }
            DebatRole::Judge => {
                self.cur_role = DebatRole::Over;
            }
            _ => {}
        }
        let mut events = Vec::new();
        events.push(GameEvent::Broadcast(format!("{}: {}", action_id, content)));
        if let Some(next_actor) = state.actors.iter().find(|a| a.role == self.cur_role) {
            if matches!(next_actor.kind, ActionKind::Ai) {
                events.push(GameEvent::TriggerAi(next_actor.id.clone()));
            }
        }
        if self.cur_role == DebatRole::Over {
            events.push(GameEvent::GameOver);
        }
        Ok(events)
    }

    fn get_snapshot(
        &self,
        _state: &RoomState<DebatRole, DebatAction>,
        _role: &DebatRole,
    ) -> String {
        "snapshot".to_string()
    }
}

fn make_actor(id: &str, role: DebatRole, kind: ActionKind) -> Actor<DebatRole> {
    Actor {
        id: id.to_string(),
        kind,
        role,
    }
}

fn make_state(actors: Vec<Actor<DebatRole>>) -> RoomState<DebatRole, DebatAction> {
    let mut state = RoomState::new("test_room".to_string(), "lincoln".to_string());
    state.actors = actors;
    state
}

fn spawn_test_room(
    actors: Vec<Actor<DebatRole>>,
    max_round: usize,
) -> (mpsc::Sender<RoomCommand>, mpsc::Receiver<AiTask>) {
    let state = make_state(actors);
    let engine = Box::new(MockEngine {
        cur_role: DebatRole::Pro,
        round: 0,
        max_round,
    });
    let (ai_tx, ai_rx) = mpsc::channel::<AiTask>(16);
    let room_tx = spawn_game_room("test_room".to_string(), engine, state, Some(ai_tx));
    (room_tx, ai_rx)
}

fn make_peer(actor_id: &str, tx: mpsc::Sender<String>) -> Peer {
    Peer {
        actor_id: actor_id.to_string(),
        tx,
    }
}

#[tokio::test]
async fn test_join_valid_human() {
    let actors = vec![
        make_actor("actor1", DebatRole::Pro, ActionKind::Human),
        make_actor("actor2", DebatRole::Con, ActionKind::Human),
    ];
    let (room_tx, _ai_rx) = spawn_test_room(actors, 2);
    let (peer_tx, mut peer_rx) = mpsc::channel::<String>(32);
    let peer = make_peer("actor1", peer_tx);
    room_tx.send(RoomCommand::Join(peer)).await.unwrap();
    let msg = timeout(Duration::from_secs(1), peer_rx.recv())
        .await
        .unwrap()
        .unwrap();
    assert!(msg.contains("joined:actor1"));
}

#[tokio::test]
async fn test_join_ai_rejected() {
    let actors = vec![make_actor("ai_actor", DebatRole::Judge, ActionKind::Ai)];
    let (room_tx, _ai_rx) = spawn_test_room(actors, 1);
    let (peer_tx, mut peer_rx) = mpsc::channel::<String>(32);
    let peer = make_peer("ai_actor", peer_tx);
    room_tx.send(RoomCommand::Join(peer)).await.unwrap();
    let msg = timeout(Duration::from_secs(1), peer_rx.recv())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(msg, "not_a_player_in_this_room");
}

#[tokio::test]
async fn test_join_invalid_actor() {
    let actors = vec![make_actor("actor1", DebatRole::Pro, ActionKind::Human)];
    let (room_tx, _ai_rx) = spawn_test_room(actors, 2);
    let (peer_tx, mut peer_rx) = mpsc::channel::<String>(32);
    let peer = make_peer("unknown", peer_tx);
    room_tx.send(RoomCommand::Join(peer)).await.unwrap();
    let msg = timeout(Duration::from_secs(1), peer_rx.recv())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(msg, "not_a_player_in_this_room");
}

#[tokio::test]
async fn test_player_action_valid() {
    let actors = vec![
        make_actor("actor1", DebatRole::Pro, ActionKind::Human),
        make_actor("actor2", DebatRole::Con, ActionKind::Human),
    ];
    let (room_tx, _ai_rx) = spawn_test_room(actors, 1);
    let (peer_tx, mut peer_rx) = mpsc::channel::<String>(32);
    let peer = make_peer("actor1", peer_tx);
    room_tx.send(RoomCommand::Join(peer)).await.unwrap();
    // consume join message
    let _ = timeout(Duration::from_secs(1), peer_rx.recv())
        .await
        .unwrap()
        .unwrap();
    room_tx
        .send(RoomCommand::PlayerAction {
            actor_id: "actor1".to_string(),
            action: "valid".to_string(),
        })
        .await
        .unwrap();
    let msg = timeout(Duration::from_secs(1), peer_rx.recv())
        .await
        .unwrap()
        .unwrap();
    assert!(msg.contains("actor1: hello"));
}

#[tokio::test]
async fn test_player_action_invalid() {
    let actors = vec![make_actor("actor1", DebatRole::Pro, ActionKind::Human)];
    let (room_tx, _ai_rx) = spawn_test_room(actors, 1);
    let (peer_tx, mut peer_rx) = mpsc::channel::<String>(32);
    let peer = make_peer("actor1", peer_tx);
    room_tx.send(RoomCommand::Join(peer)).await.unwrap();
    let _ = timeout(Duration::from_secs(1), peer_rx.recv())
        .await
        .unwrap()
        .unwrap();
    room_tx
        .send(RoomCommand::PlayerAction {
            actor_id: "actor1".to_string(),
            action: "bad".to_string(),
        })
        .await
        .unwrap();
    let msg = timeout(Duration::from_secs(1), peer_rx.recv())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(msg, "invalid_action");
}

#[tokio::test]
async fn test_player_action_wrong_turn() {
    let actors = vec![
        make_actor("actor1", DebatRole::Pro, ActionKind::Human),
        make_actor("actor2", DebatRole::Con, ActionKind::Human),
    ];
    let (room_tx, _ai_rx) = spawn_test_room(actors, 1);
    let (peer_tx, mut peer_rx) = mpsc::channel::<String>(32);
    let peer = make_peer("actor2", peer_tx);
    room_tx.send(RoomCommand::Join(peer)).await.unwrap();
    let _ = timeout(Duration::from_secs(1), peer_rx.recv())
        .await
        .unwrap()
        .unwrap();
    room_tx
        .send(RoomCommand::PlayerAction {
            actor_id: "actor2".to_string(),
            action: "valid".to_string(),
        })
        .await
        .unwrap();
    let msg = timeout(Duration::from_secs(1), peer_rx.recv())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(msg, "action_failed");
}

#[tokio::test]
async fn test_ai_trigger() {
    let actors = vec![
        make_actor("actor1", DebatRole::Pro, ActionKind::Human),
        make_actor("actor2", DebatRole::Con, ActionKind::Human),
        make_actor("ai_judge", DebatRole::Judge, ActionKind::Ai),
    ];
    let (room_tx, mut ai_rx) = spawn_test_room(actors, 1);
    let (peer_tx, mut peer_rx) = mpsc::channel::<String>(32);
    let peer = make_peer("actor1", peer_tx);
    room_tx.send(RoomCommand::Join(peer)).await.unwrap();
    let _ = timeout(Duration::from_secs(1), peer_rx.recv())
        .await
        .unwrap()
        .unwrap();
    room_tx
        .send(RoomCommand::PlayerAction {
            actor_id: "actor1".to_string(),
            action: "valid".to_string(),
        })
        .await
        .unwrap();
    let msg = timeout(Duration::from_secs(1), peer_rx.recv())
        .await
        .unwrap()
        .unwrap();
    assert!(msg.contains("actor1: hello"));
    let ai_task = timeout(Duration::from_secs(1), ai_rx.recv())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(ai_task.actor_id, "ai_judge");
    assert_eq!(ai_task.snapshot, "snapshot");
}

#[tokio::test]
async fn test_game_over() {
    let actors = vec![
        make_actor("actor1", DebatRole::Pro, ActionKind::Human),
        make_actor("judge", DebatRole::Judge, ActionKind::Human),
    ];
    let (room_tx, _ai_rx) = spawn_test_room(actors, 1);
    let (peer_tx, mut peer_rx) = mpsc::channel::<String>(32);
    let peer = make_peer("actor1", peer_tx);
    room_tx.send(RoomCommand::Join(peer)).await.unwrap();
    let _ = timeout(Duration::from_secs(1), peer_rx.recv())
        .await
        .unwrap()
        .unwrap();
    room_tx
        .send(RoomCommand::PlayerAction {
            actor_id: "actor1".to_string(),
            action: "valid".to_string(),
        })
        .await
        .unwrap();
    // consume broadcast
    let _ = timeout(Duration::from_secs(1), peer_rx.recv())
        .await
        .unwrap()
        .unwrap();
    // join judge
    let (peer_tx2, mut peer_rx2) = mpsc::channel::<String>(32);
    let peer2 = make_peer("judge", peer_tx2);
    room_tx.send(RoomCommand::Join(peer2)).await.unwrap();
    let _ = timeout(Duration::from_secs(1), peer_rx2.recv())
        .await
        .unwrap()
        .unwrap();
    room_tx
        .send(RoomCommand::PlayerAction {
            actor_id: "judge".to_string(),
            action: "valid".to_string(),
        })
        .await
        .unwrap();
    let msg = timeout(Duration::from_secs(1), peer_rx2.recv())
        .await
        .unwrap()
        .unwrap();
    assert!(msg.contains("judge: hello"));
    let msg2 = timeout(Duration::from_secs(1), peer_rx2.recv())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(msg2, "game_over");
}

#[tokio::test]
async fn test_leave_and_shutdown() {
    let actors = vec![make_actor("actor1", DebatRole::Pro, ActionKind::Human)];
    let (room_tx, _ai_rx) = spawn_test_room(actors, 1);
    let (peer_tx, mut peer_rx) = mpsc::channel::<String>(32);
    let peer = make_peer("actor1", peer_tx);
    room_tx.send(RoomCommand::Join(peer)).await.unwrap();
    let _ = timeout(Duration::from_secs(1), peer_rx.recv())
        .await
        .unwrap()
        .unwrap();
    room_tx
        .send(RoomCommand::Leave("actor1".to_string()))
        .await
        .unwrap();
    let msg = timeout(Duration::from_secs(1), peer_rx.recv())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(msg, "left:actor1");
    // After leave the room shuts down because no peers remain.
    // No further assertions needed.
}
