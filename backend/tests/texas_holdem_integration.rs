//! 德州扑克后端全流程集成测试
//!
//! AI 配置参数：
//!   model:    tp-mimo-pro
//!   base_url: http://127.0.0.1:4000
//!   api_key:  sk-66

use std::collections::HashMap;
use std::sync::Arc;

use dashmap::DashMap;
use platform_core::games::texas_holdem::*;
use platform_core::traits::{ActionKind, EngineEvent, GameEngine};

// ─── AI 配置常量 ───────────────────────────────────────────────

const AI_MODEL: &str = "tp-mimo-pro";
const AI_BASE_URL: &str = "http://127.0.0.1:4000";
const AI_API_KEY: &str = "sk-66";

/// 构造测试用 AiConfig
fn test_ai_config(prompt: &str) -> backend::ai::env::AiConfig {
    backend::ai::env::AiConfig {
        api_key: AI_API_KEY.to_string(),
        base_url: AI_BASE_URL.to_string(),
        model: AI_MODEL.to_string(),
        max_tokens: 2048,
        prompt: prompt.to_string(),
    }
}

/// 构造预填充了全局默认配置的 DashMap
fn global_configs_with_defaults() -> Arc<DashMap<String, backend::ai::env::AiConfig>> {
    let configs: Arc<DashMap<String, backend::ai::env::AiConfig>> = Arc::new(DashMap::new());
    let default_prompt = "你是一位经验丰富的德州扑克 AI 玩家。请根据当前局面做出最优决策（fold/check/call/raise/all_in）。";
    // create_texas_holdem 查找 key 为 "__defaults__/{player_id}" 的全局默认配置
    configs.insert(
        "__defaults__/player2".to_string(),
        test_ai_config(default_prompt),
    );
    configs.insert(
        "__defaults__/p2".to_string(),
        test_ai_config(default_prompt),
    );
    configs.insert(
        "__defaults__/ai1".to_string(),
        test_ai_config(default_prompt),
    );
    configs.insert(
        "__defaults__/ai_player".to_string(),
        test_ai_config(default_prompt),
    );
    configs
}

// ─── 引擎基础测试 ──────────────────────────────────────────────

#[test]
fn test_engine_creation() {
    let engine = TexasHoldemEngine::new("test_room".to_string(), 10, 20);
    assert_eq!(engine.game_type(), "texas_holdem");
    assert_eq!(engine.small_blind, 10);
    assert_eq!(engine.big_blind, 20);
    assert_eq!(engine.phase, GamePhase::WaitingForPlayers);
    assert!(engine.players.is_empty());
    assert!(engine.community_cards.is_empty());
    assert_eq!(engine.pot, 0);
}

#[test]
fn test_add_players() {
    let mut engine = TexasHoldemEngine::new("test_room".to_string(), 10, 20);

    engine.add_player("player1".to_string(), ActionKind::Human, 1000);
    engine.add_player("player2".to_string(), ActionKind::Ai, 1000);

    assert_eq!(engine.players.len(), 2);
    assert_eq!(engine.players[0].id, "player1");
    assert_eq!(engine.players[0].kind, "Human");
    assert_eq!(engine.players[0].chips, 1000);
    assert_eq!(engine.players[1].id, "player2");
    assert_eq!(engine.players[1].kind, "Ai");
    assert_eq!(engine.players[1].chips, 1000);
}

// ─── 游戏流程测试 ──────────────────────────────────────────────

#[test]
fn test_start_new_hand() {
    let mut engine = TexasHoldemEngine::new("test_room".to_string(), 10, 20);
    engine.add_player("player1".to_string(), ActionKind::Human, 1000);
    engine.add_player("player2".to_string(), ActionKind::Ai, 1000);

    // 第一步应该触发游戏开始（WaitingForPlayers → PreFlop）
    let events = engine.step("player1", serde_json::json!({})).unwrap();

    assert_eq!(engine.phase, GamePhase::PreFlop);
    assert_eq!(engine.players.len(), 2);

    // 验证盲注已下，且在 current_bet 中
    assert_eq!(engine.pot, 0, "盲注后 pot 应该为 0，筹码在 current_bet 中");
    assert!(engine.current_bet > 0, "当前有盲注");

    // 验证每个玩家有两张手牌
    for p in &engine.players {
        assert_eq!(p.hand.len(), 2, "每个玩家应有两张手牌");
    }

    // 如果当前行动者是 AI，应该触发 AI 事件
    if let Some(active) = engine.current_actor() {
        let player = engine.players.iter().find(|p| p.id == active).unwrap();
        if player.kind == "Ai" {
            assert!(
                events
                    .iter()
                    .any(|e| matches!(e, EngineEvent::TriggerAi(_))),
                "AI 玩家应触发 TriggerAi 事件"
            );
        }
    }
}

#[test]
fn test_blind_bets() {
    let mut engine = TexasHoldemEngine::new("test_room".to_string(), 10, 20);
    engine.add_player("player1".to_string(), ActionKind::Human, 1000);
    engine.add_player("player2".to_string(), ActionKind::Ai, 1000);

    engine.step("player1", serde_json::json!({})).unwrap();

    // 验证盲注金额
    let sb_player = engine
        .players
        .iter()
        .find(|p| p.current_bet > 0 && p.current_bet < engine.big_blind);
    let bb_player = engine
        .players
        .iter()
        .find(|p| p.current_bet == engine.big_blind);

    // 小盲或大盲至少有一个
    assert!(
        sb_player.is_some() || bb_player.is_some(),
        "应该有玩家下了盲注"
    );
    assert_eq!(engine.pot, 0, "小盲和大盲的筹码在 current_bet 中，pot 为 0");
}

#[test]
fn test_call_action() {
    let mut engine = TexasHoldemEngine::new("test_room".to_string(), 10, 20);
    engine.add_player("player1".to_string(), ActionKind::Human, 1000);
    engine.add_player("player2".to_string(), ActionKind::Human, 1000);

    engine.step("player1", serde_json::json!({})).unwrap();

    let active = engine.current_actor().expect("应有活跃玩家");
    let active_idx = engine.players.iter().position(|p| p.id == active).unwrap();
    let chips_before = engine.players[active_idx].chips;

    // 执行 call
    let _events = engine
        .step(&active, serde_json::json!({"action": "call"}))
        .unwrap();

    let chips_after = engine.players[active_idx].chips;
    assert!(chips_after < chips_before, "call 后筹码应减少");
}

#[test]
fn test_raise_action() {
    let mut engine = TexasHoldemEngine::new("test_room".to_string(), 10, 20);
    engine.add_player("player1".to_string(), ActionKind::Human, 1000);
    engine.add_player("player2".to_string(), ActionKind::Human, 1000);

    engine.step("player1", serde_json::json!({})).unwrap();

    let active = engine.current_actor().expect("应有活跃玩家");

    // 执行 raise 到 60
    let _events = engine
        .step(
            &active,
            serde_json::json!({"action": "raise", "amount": 60}),
        )
        .unwrap();

    assert_eq!(engine.current_bet, 60, "raise 后当前下注应为 60");
}

#[test]
fn test_fold_action() {
    let mut engine = TexasHoldemEngine::new("test_room".to_string(), 10, 20);
    engine.add_player("player1".to_string(), ActionKind::Human, 1000);
    engine.add_player("player2".to_string(), ActionKind::Human, 1000);

    engine.step("player1", serde_json::json!({})).unwrap();

    let active = engine.current_actor().expect("应有活跃玩家");

    // 执行 fold
    let events = engine
        .step(&active, serde_json::json!({"action": "fold"}))
        .unwrap();

    let folded_player = engine.players.iter().find(|p| p.id == active).unwrap();
    assert!(folded_player.folded, "fold 后玩家应标记为已弃牌");

    // 只剩一个玩家，游戏应结束
    assert!(
        engine.is_finished() || events.iter().any(|e| matches!(e, EngineEvent::GameOver)),
        "只剩一个玩家时游戏应结束"
    );
}

#[test]
fn test_check_action_when_no_bet() {
    let mut engine = TexasHoldemEngine::new("test_room".to_string(), 10, 20);
    engine.add_player("player1".to_string(), ActionKind::Human, 1000);
    engine.add_player("player2".to_string(), ActionKind::Human, 1000);

    engine.step("player1", serde_json::json!({})).unwrap();

    let active = engine.current_actor().expect("应有活跃玩家");

    // call 大盲，然后轮到下一个人 check
    let _ = engine
        .step(&active, serde_json::json!({"action": "call"}))
        .unwrap();

    if let Some(next) = engine.current_actor() {
        if engine.current_bet
            == engine
                .players
                .iter()
                .find(|p| p.id == next)
                .unwrap()
                .current_bet
        {
            let result = engine.step(&next, serde_json::json!({"action": "check"}));
            assert!(result.is_ok(), "下注相等时 check 应该成功");
        }
    }
}

#[test]
fn test_all_in_action() {
    let mut engine = TexasHoldemEngine::new("test_room".to_string(), 10, 20);
    engine.add_player("player1".to_string(), ActionKind::Human, 100);
    engine.add_player("player2".to_string(), ActionKind::Human, 1000);

    engine.step("player1", serde_json::json!({})).unwrap();

    let active = engine.current_actor().expect("应有活跃玩家");
    let active_idx = engine.players.iter().position(|p| p.id == active).unwrap();

    let _ = engine
        .step(&active, serde_json::json!({"action": "all_in"}))
        .unwrap();

    assert_eq!(engine.players[active_idx].chips, 0, "all_in 后筹码应为 0");
    assert!(engine.players[active_idx].all_in, "应标记为 all_in");
}

// ─── Tool Calls 格式测试（AI function calling）─────────────────

#[test]
fn test_tool_calls_fold() {
    let mut engine = TexasHoldemEngine::new("test_room".to_string(), 10, 20);
    engine.add_player("player1".to_string(), ActionKind::Human, 1000);
    engine.add_player("ai_player".to_string(), ActionKind::Ai, 1000);

    engine.step("player1", serde_json::json!({})).unwrap();

    // 手动设置轮到 AI
    let ai_idx = engine
        .players
        .iter()
        .position(|p| p.id == "ai_player")
        .unwrap();
    engine.active_index = ai_idx;

    let tool_call = serde_json::json!({
        "tool_calls": [{
            "id": "call_001",
            "type": "function",
            "function": {
                "name": "poker_action",
                "arguments": "{\"action\":\"fold\"}"
            }
        }]
    });

    let events = engine.step("ai_player", tool_call).unwrap();
    assert!(engine.players[ai_idx].folded, "AI fold 后应标记为已弃牌");
}

#[test]
fn test_tool_calls_raise() {
    let mut engine = TexasHoldemEngine::new("test_room".to_string(), 10, 20);
    engine.add_player("player1".to_string(), ActionKind::Human, 1000);
    engine.add_player("ai_player".to_string(), ActionKind::Ai, 1000);

    engine.step("player1", serde_json::json!({})).unwrap();

    let ai_idx = engine
        .players
        .iter()
        .position(|p| p.id == "ai_player")
        .unwrap();

    // 如果轮到 AI
    if engine.current_actor() == Some("ai_player".to_string()) {
        let tool_call = serde_json::json!({
            "tool_calls": [{
                "id": "call_002",
                "type": "function",
                "function": {
                    "name": "poker_action",
                    "arguments": "{\"action\":\"raise\",\"amount\":100}"
                }
            }]
        });

        let _ = engine.step("ai_player", tool_call).unwrap();
        assert_eq!(engine.current_bet, 100, "AI raise 后下注应为 100");
    }
}

#[test]
fn test_tool_calls_call() {
    let mut engine = TexasHoldemEngine::new("test_room".to_string(), 10, 20);
    engine.add_player("player1".to_string(), ActionKind::Human, 1000);
    engine.add_player("ai_player".to_string(), ActionKind::Ai, 1000);

    engine.step("player1", serde_json::json!({})).unwrap();

    let ai_idx = engine
        .players
        .iter()
        .position(|p| p.id == "ai_player")
        .unwrap();

    if engine.current_actor() == Some("ai_player".to_string()) {
        let chips_before = engine.players[ai_idx].chips;

        let tool_call = serde_json::json!({
            "tool_calls": [{
                "id": "call_003",
                "type": "function",
                "function": {
                    "name": "poker_action",
                    "arguments": "{\"action\":\"call\"}"
                }
            }]
        });

        let _ = engine.step("ai_player", tool_call).unwrap();
        assert!(
            engine.players[ai_idx].chips < chips_before,
            "AI call 后筹码应减少"
        );
    }
}

// ─── 序列化 / 快照测试 ─────────────────────────────────────────

#[test]
fn test_to_json_snapshot() {
    let mut engine = TexasHoldemEngine::new("test_room".to_string(), 10, 20);
    engine.add_player("player1".to_string(), ActionKind::Human, 1000);
    engine.add_player("player2".to_string(), ActionKind::Ai, 1000);

    engine.step("player1", serde_json::json!({})).unwrap();

    let snapshot = engine.to_json();

    assert_eq!(snapshot["game_type"], "texas_holdem");
    assert_eq!(snapshot["room_id"], "test_room");
    assert_eq!(snapshot["small_blind"], 10);
    assert_eq!(snapshot["big_blind"], 20);
    assert!(snapshot["players"].is_array());
    assert!(snapshot["phase"].is_string());
    assert!(snapshot["pot"].is_number());

    // 验证玩家手牌不在公共快照中
    for p in snapshot["players"].as_array().unwrap() {
        assert!(p.get("hand").is_none(), "公共快照不应包含手牌");
    }
}

#[test]
fn test_to_json_for_player() {
    let mut engine = TexasHoldemEngine::new("test_room".to_string(), 10, 20);
    engine.add_player("player1".to_string(), ActionKind::Human, 1000);
    engine.add_player("player2".to_string(), ActionKind::Ai, 1000);

    engine.step("player1", serde_json::json!({})).unwrap();

    let p1_snapshot = engine.to_json_for_player("player1");
    let p2_snapshot = engine.to_json_for_player("player2");

    // 每个玩家应该能看到自己的手牌
    if let Some(hand) = p1_snapshot.get("your_hand") {
        assert!(hand.is_array());
        assert_eq!(hand.as_array().unwrap().len(), 2);
    }

    if let Some(hand) = p2_snapshot.get("your_hand") {
        assert!(hand.is_array());
        assert_eq!(hand.as_array().unwrap().len(), 2);
    }

    // player1 不应该看到 player2 的手牌
    let players = p1_snapshot["players"].as_array().unwrap();
    for p in players {
        assert!(p.get("hand").is_none(), "不应暴露其他玩家的手牌");
    }
}

// ─── Tools 定义测试 ─────────────────────────────────────────────

#[test]
fn test_tools_definition() {
    let engine = TexasHoldemEngine::new("test_room".to_string(), 10, 20);

    let tools = engine.tools().expect("德州扑克应提供 tools 定义");
    let tools_arr = tools.as_array().expect("tools 应为数组");
    assert_eq!(tools_arr.len(), 1);

    let func = &tools_arr[0]["function"];
    assert_eq!(func["name"], "poker_action");

    let params = &func["parameters"]["properties"];
    assert!(params["action"].is_object(), "应有 action 参数");
    assert!(params["amount"].is_object(), "应有 amount 参数");

    let action_enum = params["action"]["enum"].as_array().unwrap();
    assert!(action_enum.contains(&serde_json::json!("fold")));
    assert!(action_enum.contains(&serde_json::json!("check")));
    assert!(action_enum.contains(&serde_json::json!("call")));
    assert!(action_enum.contains(&serde_json::json!("raise")));
    assert!(action_enum.contains(&serde_json::json!("all_in")));
}

// ─── 完整对局模拟 ──────────────────────────────────────────────

#[test]
fn test_full_game_simulation() {
    let mut engine = TexasHoldemEngine::new("sim_room".to_string(), 10, 20);
    engine.add_player("human1".to_string(), ActionKind::Human, 500);
    engine.add_player("ai1".to_string(), ActionKind::Ai, 500);

    // 1. 开始游戏
    let _ = engine.step("human1", serde_json::json!({})).unwrap();
    assert_eq!(engine.phase, GamePhase::PreFlop, "应进入 PreFlop");

    // 2. 模拟几轮下注直到进入 Flop
    let mut max_steps = 20;
    while engine.phase == GamePhase::PreFlop && max_steps > 0 {
        max_steps -= 1;
        if let Some(active) = engine.current_actor() {
            let _ = engine
                .step(&active, serde_json::json!({"action": "call"}))
                .unwrap();
        } else {
            break;
        }
    }

    // 3. 验证进入了下一个阶段或游戏结束
    assert!(
        engine.phase == GamePhase::Flop
            || engine.phase == GamePhase::Turn
            || engine.phase == GamePhase::River
            || engine.phase == GamePhase::Showdown
            || engine.phase == GamePhase::Finished
            || engine.is_finished(),
        "游戏应推进到下一阶段，当前: {:?}",
        engine.phase
    );

    // 4. 验证快照可序列化
    let snapshot = engine.to_json();
    let json_str = serde_json::to_string(&snapshot).expect("快照应可序列化为 JSON");
    assert!(!json_str.is_empty());
}

#[test]
fn test_full_game_to_showdown() {
    let mut engine = TexasHoldemEngine::new("showdown_room".to_string(), 10, 20);
    engine.add_player("p1".to_string(), ActionKind::Human, 200);
    engine.add_player("p2".to_string(), ActionKind::Human, 200);

    // 开始
    let _ = engine.step("p1", serde_json::json!({})).unwrap();

    // 一直 call 直到游戏结束
    let mut max_steps = 100;
    while !engine.is_finished() && max_steps > 0 {
        max_steps -= 1;
        if let Some(active) = engine.current_actor() {
            println!("Before action {}: p1: {} (bet {}), p2: {} (bet {}), pot: {}, current_bet: {}", active, engine.players[0].chips, engine.players[0].current_bet, engine.players[1].chips, engine.players[1].current_bet, engine.pot, engine.current_bet);
            match engine.step(&active, serde_json::json!({"action": "call"})) {
                Ok(_) => {}
                Err(e) => {
                    println!("Call failed: {:?}", e);
                    // 如果 call 失败，尝试 check
                    let _ = engine.step(&active, serde_json::json!({"action": "check"}));
                }
            }
            println!("After action {}: p1: {} (bet {}), p2: {} (bet {}), pot: {}, current_bet: {}, phase: {:?}", active, engine.players[0].chips, engine.players[0].current_bet, engine.players[1].chips, engine.players[1].current_bet, engine.pot, engine.current_bet, engine.phase);
        } else {
            break;
        }
    }

    // 验证游戏结束
    assert!(engine.is_finished(), "游戏应该已经结束");

    // 验证有赢家分配筹码
    let total_chips: u32 = engine.players.iter().map(|p| p.chips).sum();
    println!("Total chips: {}", total_chips);
    assert_eq!(total_chips, 400, "筹码总量应守恒（初始 200×2=400）");
}

// ─── 持久化恢复测试 ────────────────────────────────────────────

#[test]
fn test_snapshot_and_restore_flow() {
    let mut engine = TexasHoldemEngine::new("restore_room".to_string(), 5, 10);
    engine.add_player("p1".to_string(), ActionKind::Human, 500);
    engine.add_player("p2".to_string(), ActionKind::Ai, 500);

    // 开始游戏
    let _ = engine.step("p1", serde_json::json!({})).unwrap();

    // 生成快照
    let snapshot = engine.to_json();
    let snapshot_str = serde_json::to_string(&snapshot).unwrap();

    // 模拟从 JSON 恢复
    let restored_value: serde_json::Value = serde_json::from_str(&snapshot_str).unwrap();

    // 验证关键字段完整
    assert_eq!(restored_value["game_type"], "texas_holdem");
    assert_eq!(restored_value["room_id"], "restore_room");
    assert_eq!(restored_value["small_blind"], 5);
    assert_eq!(restored_value["big_blind"], 10);
    assert_eq!(restored_value["pot"].as_u64().unwrap(), 0);
}

// ─── 边界条件测试 ──────────────────────────────────────────────

#[test]
fn test_invalid_actor() {
    let mut engine = TexasHoldemEngine::new("test_room".to_string(), 10, 20);
    engine.add_player("p1".to_string(), ActionKind::Human, 1000);
    engine.add_player("p2".to_string(), ActionKind::Human, 1000);

    engine.step("p1", serde_json::json!({})).unwrap();

    // 使用不存在的玩家 ID
    let result = engine.step("ghost_player", serde_json::json!({"action": "call"}));
    assert!(result.is_err(), "不存在的玩家应返回错误");
}

#[test]
fn test_out_of_turn() {
    let mut engine = TexasHoldemEngine::new("test_room".to_string(), 10, 20);
    engine.add_player("p1".to_string(), ActionKind::Human, 1000);
    engine.add_player("p2".to_string(), ActionKind::Human, 1000);

    engine.step("p1", serde_json::json!({})).unwrap();

    let active = engine.current_actor().expect("应有活跃玩家");
    let inactive = if active == "p1" { "p2" } else { "p1" };

    // 不是自己的回合
    let result = engine.step(inactive, serde_json::json!({"action": "call"}));
    assert!(result.is_err(), "不是自己的回合应返回错误");
}

#[test]
fn test_raise_below_current_bet() {
    let mut engine = TexasHoldemEngine::new("test_room".to_string(), 10, 20);
    engine.add_player("p1".to_string(), ActionKind::Human, 1000);
    engine.add_player("p2".to_string(), ActionKind::Human, 1000);

    engine.step("p1", serde_json::json!({})).unwrap();

    let active = engine.current_actor().expect("应有活跃玩家");

    // 尝试 raise 低于当前下注
    let result = engine.step(&active, serde_json::json!({"action": "raise", "amount": 5}));
    assert!(result.is_err(), "raise 低于当前下注应失败");
}

#[test]
fn test_check_when_bet_exists() {
    let mut engine = TexasHoldemEngine::new("test_room".to_string(), 10, 20);
    engine.add_player("p1".to_string(), ActionKind::Human, 1000);
    engine.add_player("p2".to_string(), ActionKind::Human, 1000);

    engine.step("p1", serde_json::json!({})).unwrap();

    let active = engine.current_actor().expect("应有活跃玩家");

    // 如果当前有未匹配的下注，check 应该失败
    let active_player = engine.players.iter().find(|p| p.id == active).unwrap();
    if active_player.current_bet < engine.current_bet {
        let result = engine.step(&active, serde_json::json!({"action": "check"}));
        assert!(result.is_err(), "有未匹配下注时 check 应失败");
    }
}

// ─── 牌型评估测试 ──────────────────────────────────────────────

#[test]
fn test_hand_rankings() {
    let engine = TexasHoldemEngine::new("test".to_string(), 10, 20);

    // 皇家同花顺 > 四条
    let royal = vec![
        Card::new(Suit::Spades, Rank::Ace),
        Card::new(Suit::Spades, Rank::King),
        Card::new(Suit::Spades, Rank::Queen),
        Card::new(Suit::Spades, Rank::Jack),
        Card::new(Suit::Spades, Rank::Ten),
    ];
    let royal_eval = engine.evaluate_five_cards(&royal);

    let four_kind = vec![
        Card::new(Suit::Hearts, Rank::Ace),
        Card::new(Suit::Diamonds, Rank::Ace),
        Card::new(Suit::Clubs, Rank::Ace),
        Card::new(Suit::Spades, Rank::Ace),
        Card::new(Suit::Hearts, Rank::King),
    ];
    let four_eval = engine.evaluate_five_cards(&four_kind);

    assert!(royal_eval > four_eval, "皇家同花顺应大于四条");

    // 同花 > 顺子
    let flush = vec![
        Card::new(Suit::Hearts, Rank::Two),
        Card::new(Suit::Hearts, Rank::Five),
        Card::new(Suit::Hearts, Rank::Seven),
        Card::new(Suit::Hearts, Rank::Nine),
        Card::new(Suit::Hearts, Rank::Jack),
    ];
    let flush_eval = engine.evaluate_five_cards(&flush);

    let straight = vec![
        Card::new(Suit::Hearts, Rank::Five),
        Card::new(Suit::Diamonds, Rank::Six),
        Card::new(Suit::Clubs, Rank::Seven),
        Card::new(Suit::Spades, Rank::Eight),
        Card::new(Suit::Hearts, Rank::Nine),
    ];
    let straight_eval = engine.evaluate_five_cards(&straight);

    assert!(flush_eval > straight_eval, "同花应大于顺子");
}

#[test]
fn test_wheel_straight() {
    let engine = TexasHoldemEngine::new("test".to_string(), 10, 20);

    // A-2-3-4-5 (最小的顺子)
    let wheel = vec![
        Card::new(Suit::Hearts, Rank::Ace),
        Card::new(Suit::Diamonds, Rank::Two),
        Card::new(Suit::Clubs, Rank::Three),
        Card::new(Suit::Spades, Rank::Four),
        Card::new(Suit::Hearts, Rank::Five),
    ];
    let eval = engine.evaluate_five_cards(&wheel);
    assert_eq!(
        eval.category,
        HandRankCategory::Straight,
        "A-2-3-4-5 应为顺子"
    );
}

// ─── 历史记录测试 ──────────────────────────────────────────────

#[test]
fn test_action_history() {
    let mut engine = TexasHoldemEngine::new("test_room".to_string(), 10, 20);
    engine.add_player("p1".to_string(), ActionKind::Human, 1000);
    engine.add_player("p2".to_string(), ActionKind::Human, 1000);

    engine.step("p1", serde_json::json!({})).unwrap();

    let initial_history_len = engine.history.len();

    if let Some(active) = engine.current_actor() {
        let _ = engine
            .step(&active, serde_json::json!({"action": "call"}))
            .unwrap();
        assert!(
            engine.history.len() > initial_history_len,
            "执行动作后历史记录应增加"
        );

        let last = engine.history.last().unwrap();
        assert_eq!(last.actor_id, active);
        assert_eq!(last.action, PlayerAction::Call);
    }
}

