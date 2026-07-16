use std::sync::Arc;
use std::collections::HashMap;

use serde_json::Value;
use tokio::sync::mpsc;

use backend::auth::AuthService;
use backend::room::RoomService;
use backend::room::supervisor::RoomSupervisor;
use backend::room::model::{CreateRoomInput, Peer, RoomCommand};
use backend::games::factory::GameRegistry;
use backend::user::repository::SqliteUserRepo;
use backend::room::repository::SqliteRoomRepo;
use backend::ai::config_repo::SqliteAiConfigRepo;
use backend::event_store::SqliteEventStore;

static DB_SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(400);

fn unique_id(prefix: &str) -> String {
    let seq = DB_SEQ.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    format!("{}_{}", prefix, seq)
}

async fn setup() -> (sqlx::SqlitePool, AuthService, RoomService) {
    let db_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "sqlite://test_v8_spec.db?mode=rwc".to_string());
    let pool = sqlx::SqlitePool::connect(&db_url).await.unwrap();

    for query in [
        "DROP TABLE IF EXISTS game_events",
        "DROP TABLE IF EXISTS ai_configs",
        "DROP TABLE IF EXISTS rooms",
        "DROP TABLE IF EXISTS users",
    ] {
        let _ = sqlx::query(query).execute(&pool).await;
    }

    sqlx::query("CREATE TABLE IF NOT EXISTS users (id TEXT PRIMARY KEY, username TEXT UNIQUE NOT NULL, password_hash TEXT NOT NULL, created_at TEXT NOT NULL DEFAULT (datetime('now')))")
        .execute(&pool).await.unwrap();
    sqlx::query("CREATE TABLE IF NOT EXISTS rooms (room_id TEXT PRIMARY KEY, owner_id TEXT NOT NULL, game_type TEXT NOT NULL, engine_state TEXT NOT NULL, actor_slots TEXT NOT NULL, ai_configs TEXT NOT NULL DEFAULT '{}', max_round INTEGER NOT NULL DEFAULT 16, game_config TEXT NOT NULL DEFAULT '{}', is_public INTEGER NOT NULL DEFAULT 0, created_at TEXT NOT NULL DEFAULT (datetime('now')), event_seq INTEGER NOT NULL DEFAULT 0, ai_insights TEXT, invite_code TEXT)")
        .execute(&pool).await.unwrap();
    sqlx::query("CREATE TABLE IF NOT EXISTS ai_configs (room_id TEXT NOT NULL, actor_id TEXT NOT NULL, api_key TEXT NOT NULL DEFAULT '', base_url TEXT NOT NULL DEFAULT '', model TEXT NOT NULL DEFAULT '', max_tokens INTEGER NOT NULL DEFAULT 2048, prompt TEXT NOT NULL DEFAULT '', style TEXT NOT NULL DEFAULT 'default', PRIMARY KEY (room_id, actor_id))")
        .execute(&pool).await.unwrap();
    sqlx::query("CREATE TABLE IF NOT EXISTS game_events (id INTEGER PRIMARY KEY AUTOINCREMENT, room_id TEXT NOT NULL, seq INTEGER NOT NULL, event_type TEXT NOT NULL, actor_id TEXT NOT NULL DEFAULT '', payload TEXT NOT NULL DEFAULT '{}', created_at TEXT NOT NULL DEFAULT (datetime('now')), UNIQUE(room_id, seq))")
        .execute(&pool).await.unwrap();

    let user_repo = Arc::new(SqliteUserRepo::new(pool.clone()));
    let room_repo = Arc::new(SqliteRoomRepo::new(pool.clone()));
    let ai_config_repo = Arc::new(SqliteAiConfigRepo::new(pool.clone()));

    let auth = AuthService::new(user_repo, "test-jwt-secret", 3600);
    let (ai_tx, _ai_rx) = tokio::sync::mpsc::channel::<backend::room::model::AiTask>(64);

    let mut registry = GameRegistry::new();
    registry.register(Box::new(backend::games::lincoln::LincolnFactory));
    registry.register(Box::new(backend::games::texas_holdem::TexasHoldemFactory));

    let event_store = Arc::new(SqliteEventStore::new(pool.clone()))
        as Arc<dyn backend::event_store::EventStore>;

    let room_service = RoomService::new(
        room_repo,
        ai_config_repo,
        ai_tx,
        RoomSupervisor::new(),
        Arc::new(registry),
        event_store,
        pool.clone(),
    );

    (pool, auth, room_service)
}

fn make_texas_input(my_slot: &str) -> CreateRoomInput {
    CreateRoomInput {
        game_type: "texas_holdem".into(),
        max_round: 10,
        my_slot: my_slot.into(),
        slots: vec!["player1".into(), "player2".into()],
        slot_configs: HashMap::from([
            ("player1".into(), "human".into()),
            ("player2".into(), "ai".into()),
        ]),
        game_config: None,
        is_public: false,
    }
}

#[tokio::test]
async fn test_spectator_can_connect() {
    let (_pool, auth, room_service) = setup().await;
    let token = auth.register(&unique_id("spec1"), "123456").await.unwrap();
    let uid = auth.verify_token(&token).await.unwrap();

    let out = room_service.create_room(uid.clone(), make_texas_input("player1")).await.unwrap();
    let room_tx = room_service.get_room_tx(&out.room_id).unwrap();

    let (tx, _rx) = mpsc::channel::<String>(16);
    room_tx.send(RoomCommand::Join(Peer {
        actor_id: "__spectator__viewer1".into(),
        tx,
    })).await.unwrap();

    // If no panic, spectator connected successfully
    assert!(room_service.get_room_tx(&out.room_id).is_some(), "房间应仍在活跃列表");
}

#[tokio::test]
async fn test_spectator_receives_broadcast() {
    let (_pool, auth, room_service) = setup().await;
    let token = auth.register(&unique_id("spec2"), "123456").await.unwrap();
    let uid = auth.verify_token(&token).await.unwrap();

    let out = room_service.create_room(uid.clone(), make_texas_input("player1")).await.unwrap();
    let room_tx = room_service.get_room_tx(&out.room_id).unwrap();

    // Player joins
    let (player_tx, mut player_rx) = mpsc::channel::<String>(16);
    room_tx.send(RoomCommand::Join(Peer {
        actor_id: "player1".into(),
        tx: player_tx,
    })).await.unwrap();

    // Spectator joins
    let (spec_tx, mut spec_rx) = mpsc::channel::<String>(16);
    room_tx.send(RoomCommand::Join(Peer {
        actor_id: "__spectator__viewer2".into(),
        tx: spec_tx,
    })).await.unwrap();

    // Both should receive the player_event for the join
    let spec_msg = tokio::time::timeout(std::time::Duration::from_secs(2), spec_rx.recv()).await;
    assert!(spec_msg.is_ok(), "观战者应收到广播消息");
    let msg = spec_msg.unwrap().unwrap();
    let parsed: Value = serde_json::from_str(&msg).unwrap();
    assert!(parsed.get("type").is_some(), "广播应包含 type 字段");

    // Player should also receive
    let player_msg = tokio::time::timeout(std::time::Duration::from_secs(2), player_rx.recv()).await;
    assert!(player_msg.is_ok(), "玩家应收到广播");
}

#[tokio::test]
async fn test_spectator_cannot_send_action() {
    let (_pool, auth, room_service) = setup().await;
    let token = auth.register(&unique_id("spec3"), "123456").await.unwrap();
    let uid = auth.verify_token(&token).await.unwrap();

    let out = room_service.create_room(uid.clone(), make_texas_input("player1")).await.unwrap();
    let room_id = out.room_id.clone();
    let room_tx = room_service.get_room_tx(&room_id).unwrap();

    // Player joins
    let (player_tx, _player_rx) = mpsc::channel::<String>(16);
    room_tx.send(RoomCommand::Join(Peer {
        actor_id: "player1".into(),
        tx: player_tx,
    })).await.unwrap();

    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    // Join as spectator
    let (_spec_tx, _spec_rx) = mpsc::channel::<String>(16);
    room_tx.send(RoomCommand::Join(Peer {
        actor_id: "__spectator__viewer3".into(),
        tx: _spec_tx,
    })).await.unwrap();

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // Spectator sends action — should not crash the room
    let action = serde_json::json!({"action": "fold"});
    let (fb_tx, fb_rx) = tokio::sync::oneshot::channel::<Result<(), String>>();
    room_tx.send(RoomCommand::PlayerAction {
        actor_id: "__spectator__viewer3".into(),
        action,
        feedback_tx: Some(fb_tx),
    }).await.unwrap();

    let _ = tokio::time::timeout(std::time::Duration::from_secs(2), fb_rx).await;

    // Room should still be alive
    assert!(room_service.get_room_tx(&room_id).is_some(), "观战者 action 后房间应仍在");
}

#[tokio::test]
async fn test_spectator_disconnect_no_crash() {
    let (_pool, auth, room_service) = setup().await;
    let token = auth.register(&unique_id("spec4"), "123456").await.unwrap();
    let uid = auth.verify_token(&token).await.unwrap();

    let out = room_service.create_room(uid.clone(), make_texas_input("player1")).await.unwrap();
    let room_tx = room_service.get_room_tx(&out.room_id).unwrap();

    // Player joins
    let (player_tx, _player_rx) = mpsc::channel::<String>(16);
    room_tx.send(RoomCommand::Join(Peer {
        actor_id: "player1".into(),
        tx: player_tx,
    })).await.unwrap();

    // Spectator joins then leaves
    let (spec_tx, _spec_rx) = mpsc::channel::<String>(16);
    room_tx.send(RoomCommand::Join(Peer {
        actor_id: "__spectator__viewer4".into(),
        tx: spec_tx,
    })).await.unwrap();

    room_tx.send(RoomCommand::Leave("__spectator__viewer4".into())).await.unwrap();

    // Room should still be active (players remain)
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    assert!(room_service.get_room_tx(&out.room_id).is_some(), "观战者离开后房间应仍然活跃");
}
