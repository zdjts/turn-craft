use platform_core::games::lincoln::*;
use platform_core::traits::*;

fn make_state(actors: Vec<Actor<DebatRole>>) -> RoomState<DebatRole, DebatAction> {
    let mut state = RoomState::new("test_room".to_string(), "lincoln".to_string());
    state.actors = actors;
    state
}

fn make_actor(id: &str, role: DebatRole, kind: ActionKind) -> Actor<DebatRole> {
    Actor {
        id: id.to_string(),
        kind,
        role,
    }
}

#[test]
fn test_parse_action_valid() {
    let game = LincolnGame {
        max_round: 2,
        round: 0,
        cur_role: DebatRole::Pro,
    };
    let json = r#"{"Speech":{"action_id":"actor1","content":"hello"}}"#;
    let action = game.parse_action(json).unwrap();
    match action {
        DebatAction::Speech { action_id, content } => {
            assert_eq!(action_id, "actor1");
            assert_eq!(content, "hello");
        }
    }
}

#[test]
fn test_parse_action_invalid() {
    let game = LincolnGame {
        max_round: 2,
        round: 0,
        cur_role: DebatRole::Pro,
    };
    let json = "not json";
    assert!(game.parse_action(json).is_err());
}

#[test]
fn test_step_correct_turn() {
    let mut game = LincolnGame {
        max_round: 2,
        round: 0,
        cur_role: DebatRole::Pro,
    };
    let mut state = make_state(vec![
        make_actor("actor1", DebatRole::Pro, ActionKind::Human),
        make_actor("actor2", DebatRole::Con, ActionKind::Human),
    ]);
    let action = DebatAction::Speech {
        action_id: "actor1".to_string(),
        content: "hello".to_string(),
    };
    let events = game.step(&mut state, action).unwrap();
    assert_eq!(state.history.len(), 1);
    assert_eq!(game.round, 1);
    assert_eq!(game.cur_role, DebatRole::Con);
    assert!(events.iter().any(|e| matches!(e, GameEvent::Broadcast(_))));
}

#[test]
fn test_step_wrong_turn() {
    let mut game = LincolnGame {
        max_round: 2,
        round: 0,
        cur_role: DebatRole::Pro,
    };
    let mut state = make_state(vec![
        make_actor("actor1", DebatRole::Pro, ActionKind::Human),
        make_actor("actor2", DebatRole::Con, ActionKind::Human),
    ]);
    let action = DebatAction::Speech {
        action_id: "actor2".to_string(),
        content: "hello".to_string(),
    };
    let err = game.step(&mut state, action).unwrap_err();
    match err {
        LincolnErr::NotYourTurn => {}
        _ => panic!("expected NotYourTurn"),
    }
}

#[test]
fn test_step_not_actor() {
    let mut game = LincolnGame {
        max_round: 2,
        round: 0,
        cur_role: DebatRole::Pro,
    };
    let mut state = make_state(vec![make_actor(
        "actor1",
        DebatRole::Pro,
        ActionKind::Human,
    )]);
    let action = DebatAction::Speech {
        action_id: "unknown".to_string(),
        content: "hello".to_string(),
    };
    let err = game.step(&mut state, action).unwrap_err();
    match err {
        LincolnErr::NotActor => {}
        _ => panic!("expected NotActor"),
    }
}

#[test]
fn test_step_progression_to_judge() {
    let mut game = LincolnGame {
        max_round: 1,
        round: 0,
        cur_role: DebatRole::Pro,
    };
    let mut state = make_state(vec![
        make_actor("actor1", DebatRole::Pro, ActionKind::Human),
        make_actor("actor2", DebatRole::Con, ActionKind::Human),
        make_actor("judge", DebatRole::Judge, ActionKind::Human),
    ]);
    let action = DebatAction::Speech {
        action_id: "actor1".to_string(),
        content: "hello".to_string(),
    };
    let events = game.step(&mut state, action).unwrap();
    assert_eq!(game.round, 1);
    assert_eq!(game.cur_role, DebatRole::Judge);
    // no GameOver yet
    assert!(!events.iter().any(|e| matches!(e, GameEvent::GameOver)));
}

#[test]
fn test_step_judge_to_over() {
    let mut game = LincolnGame {
        max_round: 1,
        round: 1,
        cur_role: DebatRole::Judge,
    };
    let mut state = make_state(vec![make_actor(
        "judge",
        DebatRole::Judge,
        ActionKind::Human,
    )]);
    let action = DebatAction::Speech {
        action_id: "judge".to_string(),
        content: "verdict".to_string(),
    };
    let events = game.step(&mut state, action).unwrap();
    assert_eq!(game.cur_role, DebatRole::Over);
    assert!(events.iter().any(|e| matches!(e, GameEvent::GameOver)));
}

#[test]
fn test_get_snapshot() {
    let game = LincolnGame {
        max_round: 2,
        round: 1,
        cur_role: DebatRole::Con,
    };
    let mut state = make_state(vec![make_actor(
        "actor1",
        DebatRole::Pro,
        ActionKind::Human,
    )]);
    state.history.push(DebatAction::Speech {
        action_id: "actor1".to_string(),
        content: "hello".to_string(),
    });
    let snapshot_str = game.get_snapshot(&state, &DebatRole::Pro);
    let snapshot: LincolnSnapshot = serde_json::from_str(&snapshot_str).unwrap();
    assert_eq!(snapshot.cur_role, DebatRole::Con);
    assert_eq!(snapshot.round, 1);
    assert_eq!(snapshot.max_round, 2);
    assert_eq!(snapshot.history_logs, vec!["actor1: hello"]);
}
