use platform_core::{
    games::werewolf::{Phase, WerewolfEngine, WerewolfRole},
    traits::{ActionKind, GameEngine},
};
use serde_json::json;

fn setup_engine() -> WerewolfEngine {
    let mut engine = WerewolfEngine::new("test_room".to_string());

    engine.add_actor("w1".to_string(), ActionKind::Human, WerewolfRole::Werewolf);
    engine.add_actor("w2".to_string(), ActionKind::Human, WerewolfRole::Werewolf);
    engine.add_actor("seer".to_string(), ActionKind::Human, WerewolfRole::Seer);
    engine.add_actor("witch".to_string(), ActionKind::Human, WerewolfRole::Witch);
    engine.add_actor(
        "hunter".to_string(),
        ActionKind::Human,
        WerewolfRole::Hunter,
    );
    engine.add_actor("v1".to_string(), ActionKind::Human, WerewolfRole::Villager);
    engine.add_actor("v2".to_string(), ActionKind::Human, WerewolfRole::Villager);

    engine.start();
    engine
}

#[test]
fn test_werewolf_night_kill_and_save() {
    let mut engine = setup_engine();

    assert_eq!(engine.phase, Phase::NightWolf);

    // 狼人1杀 v1
    let _ = engine
        .step("w1", json!({"action_type": "kill", "target": "v1"}))
        .unwrap();
    // 狼人2同意杀 v1
    let _ = engine
        .step("w2", json!({"action_type": "kill", "target": "v1"}))
        .unwrap();

    assert_eq!(engine.phase, Phase::NightSeer);

    // 预言家查验 w1
    let _ = engine
        .step("seer", json!({"action_type": "check", "target": "w1"}))
        .unwrap();

    assert_eq!(engine.phase, Phase::NightWitch);

    // 女巫救 v1
    let _ = engine
        .step("witch", json!({"action_type": "save"}))
        .unwrap();

    assert_eq!(engine.phase, Phase::DaySpeech);

    // 检查存活情况，没有人死
    let alive_count = engine.players.iter().filter(|p| p.is_alive).count();
    assert_eq!(alive_count, 7);
}

#[test]
fn test_werewolf_night_kill_and_poison() {
    let mut engine = setup_engine();

    assert_eq!(engine.phase, Phase::NightWolf);

    // 狼人一致杀 v1
    let _ = engine
        .step("w1", json!({"action_type": "kill", "target": "v1"}))
        .unwrap();
    let _ = engine
        .step("w2", json!({"action_type": "kill", "target": "v1"}))
        .unwrap();

    // 预言家查验 v2
    let _ = engine
        .step("seer", json!({"action_type": "check", "target": "v2"}))
        .unwrap();

    // 女巫不救，毒死 w1
    let _ = engine
        .step("witch", json!({"action_type": "poison", "target": "w1"}))
        .unwrap();

    assert_eq!(engine.phase, Phase::DaySpeech);

    let v1 = engine.players.iter().find(|p| p.id == "v1").unwrap();
    let w1 = engine.players.iter().find(|p| p.id == "w1").unwrap();
    assert!(!v1.is_alive);
    assert!(!w1.is_alive);
}

#[test]
fn test_werewolf_day_vote_out() {
    let mut engine = setup_engine();

    // 狼人杀 v1
    let _ = engine
        .step("w1", json!({"action_type": "kill", "target": "v1"}))
        .unwrap();
    let _ = engine
        .step("w2", json!({"action_type": "kill", "target": "v1"}))
        .unwrap();
    let _ = engine
        .step("seer", json!({"action_type": "check", "target": "v2"}))
        .unwrap();
    // 女巫不作为
    let _ = engine
        .step("witch", json!({"action_type": "skip"}))
        .unwrap();

    assert_eq!(engine.phase, Phase::DaySpeech);

    // 发言阶段
    let speakers = engine.speakers.clone();
    for speaker in speakers {
        let _ = engine
            .step(
                &speaker,
                json!({"action_type": "speak", "content": "hello"}),
            )
            .unwrap();
    }

    assert_eq!(engine.phase, Phase::DayVote);

    // 大家集体票死 w1
    for p in &engine.players.clone() {
        if p.is_alive {
            let _ = engine
                .step(&p.id, json!({"action_type": "vote", "target": "w1"}))
                .unwrap();
        }
    }

    assert_eq!(engine.phase, Phase::NightWolf);
    let w1 = engine.players.iter().find(|p| p.id == "w1").unwrap();
    assert!(!w1.is_alive);
}

#[test]
fn test_werewolf_hunter_shoot() {
    let mut engine = setup_engine();

    // 狼人杀 hunter
    let _ = engine
        .step("w1", json!({"action_type": "kill", "target": "hunter"}))
        .unwrap();
    let _ = engine
        .step("w2", json!({"action_type": "kill", "target": "hunter"}))
        .unwrap();
    let _ = engine
        .step("seer", json!({"action_type": "check", "target": "v2"}))
        .unwrap();
    let _ = engine
        .step("witch", json!({"action_type": "skip"}))
        .unwrap();

    // 白天，猎人死了，可以开枪
    assert_eq!(
        engine.phase,
        Phase::DayHunterShoot("hunter".to_string(), "DaySpeech".to_string())
    );

    // 猎人开枪带走 w2
    let _ = engine
        .step("hunter", json!({"action_type": "shoot", "target": "w2"}))
        .unwrap();

    assert_eq!(engine.phase, Phase::DaySpeech);
    let w2 = engine.players.iter().find(|p| p.id == "w2").unwrap();
    assert!(!w2.is_alive);
}

#[test]
fn test_werewolf_win_condition() {
    let mut engine = setup_engine();

    // 狼人杀 v1
    let _ = engine
        .step("w1", json!({"action_type": "kill", "target": "v1"}))
        .unwrap();
    let _ = engine
        .step("w2", json!({"action_type": "kill", "target": "v1"}))
        .unwrap();
    let _ = engine
        .step("seer", json!({"action_type": "check", "target": "w2"}))
        .unwrap();
    // 女巫毒死 w2
    let _ = engine
        .step("witch", json!({"action_type": "poison", "target": "w2"}))
        .unwrap();

    // 存活好人：seer, witch, hunter, v2 (4个)
    // 存活狼人：w1 (1个)
    assert_eq!(engine.phase, Phase::DaySpeech);

    // 发言
    let speakers = engine.speakers.clone();
    for speaker in speakers {
        let _ = engine
            .step(
                &speaker,
                json!({"action_type": "speak", "content": "hello"}),
            )
            .unwrap();
    }

    // 票死 hunter
    for p in &engine.players.clone() {
        if p.is_alive {
            let _ = engine
                .step(&p.id, json!({"action_type": "vote", "target": "hunter"}))
                .unwrap();
        }
    }

    // 猎人出局开枪带走 w1
    assert_eq!(
        engine.phase,
        Phase::DayHunterShoot("hunter".to_string(), "NightWolf".to_string())
    );
    let _ = engine
        .step("hunter", json!({"action_type": "shoot", "target": "w1"}))
        .unwrap();

    // 剩下 w1 死，狼人全死，好人胜利
    assert_eq!(engine.phase, Phase::GameOver("Good".to_string()));
}
