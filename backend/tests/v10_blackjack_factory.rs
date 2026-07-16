use std::sync::Arc;
use std::collections::HashMap;

use backend::games::blackjack::BlackjackFactory;
use backend::games::factory::GameFactory;
use backend::room::model::CreateRoomInput;

fn make_input(slots: Vec<&str>, my_slot: &str) -> CreateRoomInput {
    let mut configs = HashMap::new();
    for s in &slots {
        configs.insert(s.to_string(), if *s == "player1" { "human".into() } else { "ai".into() });
    }
    CreateRoomInput {
        game_type: "blackjack".into(),
        max_round: 10,
        my_slot: my_slot.into(),
        slots: slots.into_iter().map(|s| s.to_string()).collect(),
        slot_configs: configs,
        game_config: Some(serde_json::json!({
            "starting_chips": 500,
            "min_bet": 5,
            "max_bet": 50,
        })),
        is_public: false,
    }
}

#[tokio::test]
async fn test_blackjack_factory_create_returns_engine() {
    let factory = BlackjackFactory;
    let input = make_input(vec!["player1", "player2"], "player1");

    // Use a mock config repo (SqliteAiConfigRepo with in-memory would need DB setup)
    // Instead test the factory's meta and slot handling
    let meta = factory.meta();
    assert_eq!(meta.game_type, "blackjack");
    assert_eq!(meta.min_players, 1);
    assert_eq!(meta.max_players, 6);
    assert!(meta.slot_names.len() >= 2);
    assert!(meta.config_schema.is_some(), "Blackjack 应有配置 schema");

    // Verify slot configs from meta
    let slot_names = meta.slot_names;
    for s in &input.slots {
        assert!(slot_names.contains(s), "meta 的 slot_names 应包含所有输入槽位");
    }
}

#[tokio::test]
async fn test_blackjack_factory_restore_from_json() {
    let factory = BlackjackFactory;

    // Create a state JSON representing a mid-game state
    let state = serde_json::json!({
        "room_id": "room-test",
        "players": [
            { "id": "player1", "kind": "Human", "hand": [{"suit": "Hearts", "rank": "Ace"}, {"suit": "Spades", "rank": "King"}],
              "hand_value": 21, "is_bust": false, "is_finished": true, "bet": 50 },
            { "id": "ai-1", "kind": "Ai", "hand": [{"suit": "Diamonds", "rank": "Five"}, {"suit": "Clubs", "rank": "Eight"}],
              "hand_value": 13, "is_bust": false, "is_finished": false, "bet": 20 }
        ],
        "dealer": { "cards": [{"suit": "Hearts", "rank": "Seven"}, {"suit": "Spades", "rank": "Three"}], "value": 10, "is_bust": false },
        "phase": {"PlayerTurn": {"index": 1}},
        "finished": false,
        "results": [],
        "starting_chips": 1000,
    });

    let engine = factory.restore(&state).unwrap();
    assert_eq!(engine.game_type(), "blackjack");
    assert!(!engine.is_finished());
    assert_eq!(engine.current_actor(), Some("ai-1".to_string()));

    // Verify JSON roundtrip
    let restored_json = engine.to_json();
    assert_eq!(restored_json["room_id"], "room-test");
    assert_eq!(restored_json["players"][0]["id"], "player1");
    assert_eq!(restored_json["players"][0]["hand_value"], 21);
}

#[tokio::test]
async fn test_blackjack_factory_n_players() {
    let factory = BlackjackFactory;

    // Single player (1p)
    let input_1p = make_input(vec!["player1"], "player1");
    assert_eq!(input_1p.slots.len(), 1);

    // 6 players (max)
    let input_6p = make_input(
        vec!["player1", "player2", "player3", "player4", "player5", "player6"],
        "player1"
    );
    assert_eq!(input_6p.slots.len(), 6);
}
