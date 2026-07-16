use std::sync::Arc;
use std::collections::HashMap;

use backend::auth::AuthService;
use backend::room::RoomService;
use backend::room::supervisor::RoomSupervisor;
use backend::room::model::CreateRoomInput;
use backend::games::factory::GameRegistry;
use backend::user::repository::SqliteUserRepo;
use backend::room::repository::SqliteRoomRepo;
use backend::ai::config_repo::SqliteAiConfigRepo;
use backend::event_store::SqliteEventStore;

static DB_SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(300);

fn unique_id(prefix: &str) -> String {
    let seq = DB_SEQ.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    format!("{}_{}", prefix, seq)
}

async fn setup() -> (sqlx::SqlitePool, AuthService, RoomService) {
    let db_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "sqlite://test_v8_invite.db?mode=rwc".to_string());
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
async fn test_invite_create_returns_code_and_link() {
    let (_pool, auth, room_service) = setup().await;
    let token = auth.register(&unique_id("inv_user"), "123456").await.unwrap();
    let uid = auth.verify_token(&token).await.unwrap();

    let out = room_service.create_room(uid.clone(), make_lincoln_input("Judge")).await.unwrap();
    let code = room_service.create_invite(&out.room_id).await.unwrap();

    assert_eq!(code.len(), 8, "邀请码应为 8 位");
    assert!(code.chars().all(|c| c.is_alphanumeric()), "邀请码应只包含字母数字");
}

#[tokio::test]
async fn test_invite_resolve_returns_room_id() {
    let (pool, auth, room_service) = setup().await;
    let token = auth.register(&unique_id("inv_user2"), "123456").await.unwrap();
    let uid = auth.verify_token(&token).await.unwrap();

    let out = room_service.create_room(uid.clone(), make_lincoln_input("Judge")).await.unwrap();
    let code = room_service.create_invite(&out.room_id).await.unwrap();

    // Resolve via direct SQL (simulating resolve_invite handler)
    let resolved: Option<String> = sqlx::query_scalar("SELECT room_id FROM rooms WHERE invite_code = ?")
        .bind(&code)
        .fetch_optional(&pool)
        .await.unwrap();

    assert_eq!(resolved, Some(out.room_id), "邀请码应解析到正确的 room_id");
}

#[tokio::test]
async fn test_invite_resolve_invalid_code_404() {
    let pool = setup().await.0;
    let resolved: Option<String> = sqlx::query_scalar("SELECT room_id FROM rooms WHERE invite_code = ?")
        .bind("invalid!")
        .fetch_optional(&pool)
        .await.unwrap();
    assert!(resolved.is_none(), "无效邀请码应返回 None");
}

#[tokio::test]
async fn test_invite_same_room_returns_same_code() {
    let (_pool, auth, room_service) = setup().await;
    let token = auth.register(&unique_id("inv_user3"), "123456").await.unwrap();
    let uid = auth.verify_token(&token).await.unwrap();

    let out = room_service.create_room(uid.clone(), make_lincoln_input("Judge")).await.unwrap();
    let code1 = room_service.create_invite(&out.room_id).await.unwrap();
    let code2 = room_service.create_invite(&out.room_id).await.unwrap();

    assert_eq!(code1, code2, "同一房间两次请求应返回相同邀请码");
}
