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

static DB_SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(500);

fn unique_id(prefix: &str) -> String {
    let seq = DB_SEQ.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    format!("{}_{}", prefix, seq)
}

async fn setup() -> (sqlx::SqlitePool, AuthService, RoomService) {
    let db_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "sqlite://test_v8_events.db?mode=rwc".to_string());
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

fn make_lincoln_input(my_slot: &str) -> CreateRoomInput {
    CreateRoomInput {
        game_type: "lincoln".into(),
        max_round: 3,
        my_slot: my_slot.into(),
        slots: vec!["Judge".into(), "Pro".into(), "Con".into()],
        slot_configs: HashMap::from([
            ("Judge".into(), "human".into()),
            ("Pro".into(), "human".into()),
            ("Con".into(), "ai".into()),
        ]),
        game_config: None,
        is_public: false,
    }
}

#[tokio::test]
async fn test_player_event_join_broadcast() {
    let (_pool, auth, room_service) = setup().await;
    let token = auth.register(&unique_id("pe_u1"), "123456").await.unwrap();
    let uid = auth.verify_token(&token).await.unwrap();

    let out = room_service.create_room(uid.clone(), make_lincoln_input("Judge")).await.unwrap();
    let room_tx = room_service.get_room_tx(&out.room_id).unwrap();

    // First player joins
    let (tx1, mut rx1) = mpsc::channel::<String>(16);
    room_tx.send(RoomCommand::Join(Peer {
        actor_id: "Judge".into(),
        tx: tx1,
    })).await.unwrap();

    // Drain self-join + state messages for Judge
    let _ = tokio::time::timeout(std::time::Duration::from_secs(1), rx1.recv()).await;
    let _ = tokio::time::timeout(std::time::Duration::from_secs(1), rx1.recv()).await;

    // Second player joins
    let (tx2, _rx2) = mpsc::channel::<String>(16);
    room_tx.send(RoomCommand::Join(Peer {
        actor_id: "Pro".into(),
        tx: tx2,
    })).await.unwrap();

    // First player should receive player_event with "joined"
    let msg = tokio::time::timeout(std::time::Duration::from_secs(2), rx1.recv()).await;
    assert!(msg.is_ok(), "第一人应收到广播");
    let parsed: Value = serde_json::from_str(&msg.unwrap().unwrap()).unwrap();
    assert_eq!(parsed["type"], "player_event", "广播类型应为 player_event");
    assert_eq!(parsed["event"], "joined", "事件应为 joined");
    assert_eq!(parsed["actor_id"], "Pro", "加入者应为 Pro");
}

#[tokio::test]
async fn test_player_event_leave_broadcast() {
    let (_pool, auth, room_service) = setup().await;
    let token = auth.register(&unique_id("pe_u2"), "123456").await.unwrap();
    let uid = auth.verify_token(&token).await.unwrap();

    let out = room_service.create_room(uid.clone(), make_lincoln_input("Judge")).await.unwrap();
    let room_tx = room_service.get_room_tx(&out.room_id).unwrap();

    // First player joins
    let (tx1, mut rx1) = mpsc::channel::<String>(16);
    room_tx.send(RoomCommand::Join(Peer {
        actor_id: "Judge".into(),
        tx: tx1,
    })).await.unwrap();

    // Drain self-join + state
    let _ = tokio::time::timeout(std::time::Duration::from_secs(1), rx1.recv()).await;
    let _ = tokio::time::timeout(std::time::Duration::from_secs(1), rx1.recv()).await;

    // Second player joins
    let (tx2, _rx2) = mpsc::channel::<String>(16);
    room_tx.send(RoomCommand::Join(Peer {
        actor_id: "Pro".into(),
        tx: tx2,
    })).await.unwrap();

    // Drain the Pro join broadcast from rx1
    let _ = tokio::time::timeout(std::time::Duration::from_secs(1), rx1.recv()).await;

    // Second player leaves
    room_tx.send(RoomCommand::Leave("Pro".into())).await.unwrap();

    // First player should receive player_event with "left"
    let msg = tokio::time::timeout(std::time::Duration::from_secs(2), rx1.recv()).await;
    assert!(msg.is_ok(), "第一人应收到离开广播");
    let parsed: Value = serde_json::from_str(&msg.unwrap().unwrap()).unwrap();
    assert_eq!(parsed["type"], "player_event", "广播类型应为 player_event");
    assert_eq!(parsed["event"], "left", "事件应为 left");
}

#[tokio::test]
async fn test_player_event_spectator_flag() {
    let (_pool, auth, room_service) = setup().await;
    let token = auth.register(&unique_id("pe_u3"), "123456").await.unwrap();
    let uid = auth.verify_token(&token).await.unwrap();

    let out = room_service.create_room(uid.clone(), make_lincoln_input("Judge")).await.unwrap();
    let room_tx = room_service.get_room_tx(&out.room_id).unwrap();

    // Player joins
    let (tx1, mut rx1) = mpsc::channel::<String>(16);
    room_tx.send(RoomCommand::Join(Peer {
        actor_id: "Judge".into(),
        tx: tx1,
    })).await.unwrap();

    // Drain self-join + state
    let _ = tokio::time::timeout(std::time::Duration::from_secs(1), rx1.recv()).await;
    let _ = tokio::time::timeout(std::time::Duration::from_secs(1), rx1.recv()).await;

    // Spectator joins
    let (tx2, _rx2) = mpsc::channel::<String>(16);
    room_tx.send(RoomCommand::Join(Peer {
        actor_id: "__spectator__viewer".into(),
        tx: tx2,
    })).await.unwrap();

    // Player should receive player_event with is_spectator: true
    let msg = tokio::time::timeout(std::time::Duration::from_secs(2), rx1.recv()).await;
    assert!(msg.is_ok(), "玩家应收到观战者加入广播");
    let parsed: Value = serde_json::from_str(&msg.unwrap().unwrap()).unwrap();
    assert_eq!(parsed["type"], "player_event");
    assert_eq!(parsed["event"], "joined");
    assert_eq!(parsed["actor_id"], "viewer", "观战者的 actor_id 应去除 __spectator__ 前缀");
    assert_eq!(parsed["is_spectator"], true, "观战者标志应为 true");
}
