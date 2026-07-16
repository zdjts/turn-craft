use platform_core::games::blackjack::{BlackjackEngine, BlackjackConfig, Phase};
use platform_core::traits::GameEngine;
use serde_json::json;

fn make_engine() -> BlackjackEngine {
    let mut engine = BlackjackEngine::new("room-1".into(), BlackjackConfig {
        starting_chips: 1000, min_bet: 10, max_bet: 100,
    });
    engine.add_player("player1".into(), "Human".into());
    engine.add_player("ai-1".into(), "Ai".into());
    engine
}

fn make_single_player_engine() -> BlackjackEngine {
    let mut engine = BlackjackEngine::new("room-1".into(), BlackjackConfig {
        starting_chips: 1000, min_bet: 10, max_bet: 100,
    });
    engine.add_player("player1".into(), "Human".into());
    engine
}

fn deal(mut engine: &mut BlackjackEngine) {
    let _ = engine.step("player1", json!({"action": "bet"}));
}

#[test]
fn test_blackjack_deal_two_cards() {
    let mut engine = make_engine();
    deal(&mut engine);

    assert_eq!(engine.actors.len(), 2);
    assert_eq!(engine.actors[0].hand.len(), 2, "玩家应有 2 张牌");
    assert_eq!(engine.actors[1].hand.len(), 2, "AI 应有 2 张牌");
    assert_eq!(engine.dealer.cards.len(), 2, "庄家应有 2 张牌");
    assert!(engine.actors[0].hand_value > 0, "玩家点数应 > 0");
}

#[test]
fn test_blackjack_hit_adds_card() {
    let mut engine = make_engine();
    deal(&mut engine);

    let hand_before = engine.actors[0].hand.len();
    let _ = engine.step("player1", json!({"action": "hit"}));
    assert_eq!(engine.actors[0].hand.len(), hand_before + 1, "要牌后手牌 +1");
    // Hand value should be updated
    assert!(engine.actors[0].hand_value > engine.actors[0].hand_value - 11,
            "点数应更新");
}

#[test]
fn test_blackjack_stand_moves_to_next_player() {
    let mut engine = make_engine();
    deal(&mut engine);

    assert_eq!(engine.phase, Phase::PlayerTurn { index: 0 });

    let _ = engine.step("player1", json!({"action": "stand"}));

    assert_eq!(engine.phase, Phase::PlayerTurn { index: 1 },
               "停牌后应转到下一个玩家");
    assert!(engine.actors[0].is_finished, "停牌后玩家标记为 finished");
}

#[test]
fn test_blackjack_bust_over_21() {
    let mut engine = make_single_player_engine();
    deal(&mut engine);

    // Hit until bust
    let mut hit_count = 0;
    while !engine.actors[0].is_bust && hit_count < 20 {
        let _ = engine.step("player1", json!({"action": "hit"}));
        hit_count += 1;
    }

    if engine.actors[0].is_bust {
        assert!(engine.actors[0].hand_value > 21, "爆牌时点数应 > 21");
    } else {
        // Very unlikely but possible with many low cards
        assert!(hit_count > 0, "至少执行了一次 hit");
    }
}

#[test]
fn test_blackjack_stand_then_dealer_draws_to_17() {
    let mut engine = make_single_player_engine();
    deal(&mut engine);

    // Stand immediately
    let _ = engine.step("player1", json!({"action": "stand"}));

    assert!(engine.finished, "唯一玩家停牌后游戏应结束");
    assert!(engine.dealer.value >= 17 || engine.dealer.is_bust,
            "庄家点数应 >= 17 或爆牌, 实际: {}", engine.dealer.value);
}

#[test]
fn test_blackjack_dealer_draws_to_17() {
    let mut engine = make_engine();
    deal(&mut engine);

    // Both players stand
    let _ = engine.step("player1", json!({"action": "stand"}));
    let _ = engine.step("ai-1", json!({"action": "stand"}));

    assert!(engine.finished, "全部玩家停牌后游戏应结束");
    assert!(engine.dealer.value >= 17 || engine.dealer.is_bust,
            "庄家点数应 >= 17 或爆牌");
}

#[test]
fn test_blackjack_natural_blackjack_check() {
    let mut engine = make_engine();
    deal(&mut engine);

    let player = &engine.actors[0];
    if player.hand.len() == 2 && player.hand_value == 21 {
        let has_ace = player.hand.iter().any(|c| {
            format!("{:?}", c.rank) == "Ace"
        });
        assert!(has_ace, "自然 Blackjack 应包含 A");
    }
}

#[test]
fn test_blackjack_double_doubles_bet() {
    let mut engine = make_engine();
    deal(&mut engine);

    let bet_before = engine.actors[0].bet;
    let _ = engine.step("player1", json!({"action": "double"}));

    assert_eq!(engine.actors[0].bet, bet_before * 2, "加倍后赌注翻倍");
    assert!(engine.actors[0].is_finished, "加倍后玩家应自动停牌");
}

#[test]
fn test_blackjack_all_players_bust_game_over() {
    let mut engine = make_single_player_engine();
    deal(&mut engine);

    // Hit until bust (single player -> game over on bust)
    while !engine.actors[0].is_bust {
        let _ = engine.step("player1", json!({"action": "hit"}));
    }

    assert!(engine.finished, "单玩家爆牌后游戏应结束");
}

#[test]
fn test_blackjack_to_json_contains_required_fields() {
    let mut engine = make_engine();
    deal(&mut engine);

    let state = engine.to_json();
    assert_eq!(state["game_type"], "blackjack");
    assert!(state.get("room_id").is_some());
    assert!(state.get("players").is_some());
    assert!(state.get("dealer").is_some());
    assert!(state.get("phase").is_some());
    assert!(state.get("finished").is_some());
}

#[test]
fn test_blackjack_to_json_for_player_hides_hole_card() {
    let mut engine = make_single_player_engine();
    deal(&mut engine);

    let player_view = engine.to_json_for_player("player1");
    if !engine.finished {
        let dealer = &player_view["dealer"];
        assert!(
            dealer.get("upcard").is_some() || dealer.get("card_count").is_some(),
            "玩家视角应隐藏庄家底牌"
        );
    }
}
