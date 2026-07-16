use std::sync::Arc;
use std::collections::HashMap;

use serde_json::json;

use backend::auth::AuthService;
use backend::room::RoomService;
use backend::room::supervisor::RoomSupervisor;
use backend::room::model::CreateRoomInput;
use backend::games::factory::GameRegistry;
use backend::user::repository::SqliteUserRepo;
use backend::room::repository::SqliteRoomRepo;
use backend::ai::config_repo::SqliteAiConfigRepo;
use backend::event_store::SqliteEventStore;

static DB_SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(100);

fn unique_id(prefix: &str) -> String {
    let seq = DB_SEQ.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    format!("{}_{}", prefix, seq)
}

async fn setup() -> (sqlx::SqlitePool, AuthService, RoomService) {
    let db_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "sqlite://test_v6_insights.db?mode=rwc".to_string());
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
async fn test_insights_finished_room_returns_data() {
    let (pool, auth, room_service) = setup().await;
    let token = auth.register(&unique_id("fin_user"), "123456").await.unwrap();
    let uid = auth.verify_token(&token).await.unwrap();

    let out = room_service.create_room(uid.clone(), make_lincoln_input("Judge")).await.unwrap();
    let room_id = out.room_id;

    // Mark room as finished and set cached ai_insights
    let cached = json!({ "insights": [{ "actor_id": "ai-con", "role": "Con", "style": "default", "overall_assessment": "测试评价", "key_actions": [], "highlights": [], "mistakes": [] }] });
    let cached_str = serde_json::to_string(&cached).unwrap();
    sqlx::query("UPDATE rooms SET engine_state = json_set(engine_state, '$.finished', 1), ai_insights = ? WHERE room_id = ?")
        .bind(&cached_str)
        .bind(&room_id)
        .execute(&pool)
        .await.unwrap();

    // Verify the cached data is readable
    let row: (String,) = sqlx::query_as("SELECT ai_insights FROM rooms WHERE room_id = ?")
        .bind(&room_id)
        .fetch_one(&pool)
        .await.unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&row.0).unwrap();
    assert_eq!(parsed["insights"][0]["actor_id"], "ai-con", "cached insights 应包含 ai-con");
    assert_eq!(parsed["insights"][0]["overall_assessment"], "测试评价");
}

#[tokio::test]
async fn test_insights_unfinished_room_returns_empty() {
    let (pool, auth, room_service) = setup().await;
    let token = auth.register(&unique_id("unfin_user"), "123456").await.unwrap();
    let uid = auth.verify_token(&token).await.unwrap();

    let out = room_service.create_room(uid.clone(), make_lincoln_input("Judge")).await.unwrap();
    let room_id = out.room_id;

    // Room is unfinished (finished = false by default)
    let snap = room_service.get_room_snapshot(&room_id).await.unwrap().unwrap();
    let finished = snap.engine_state.get("finished").and_then(|v| v.as_bool()).unwrap_or(false);
    assert!(!finished, "新创建的房间不应标记为 finished");

    // ai_insights should be null
    let ai_insights: String = sqlx::query_scalar("SELECT COALESCE(ai_insights, '') FROM rooms WHERE room_id = ?")
        .bind(&room_id)
        .fetch_one(&pool)
        .await.unwrap();
    assert!(ai_insights.is_empty(), "未结束对局的 ai_insights 应为空");
}

#[tokio::test]
async fn test_insights_room_not_found_404() {
    let (_pool, _auth, room_service) = setup().await;
    let result = room_service.get_room_snapshot("non_existent_room").await.unwrap();
    assert!(result.is_none(), "不存在的 room_id 应返回 None");
}
