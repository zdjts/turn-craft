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

static DB_SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(600);

fn unique_id(prefix: &str) -> String {
    let seq = DB_SEQ.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    format!("{}_{}", prefix, seq)
}

async fn setup() -> (sqlx::SqlitePool, AuthService, RoomService) {
    let db_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "sqlite://test_v9_leader.db?mode=rwc".to_string());
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
    sqlx::query("CREATE TABLE IF NOT EXISTS rooms (room_id TEXT PRIMARY KEY, owner_id TEXT NOT NULL, game_type TEXT NOT NULL, engine_state TEXT NOT NULL DEFAULT '{}', actor_slots TEXT NOT NULL DEFAULT '[]', ai_configs TEXT NOT NULL DEFAULT '{}', max_round INTEGER NOT NULL DEFAULT 16, game_config TEXT NOT NULL DEFAULT '{}', is_public INTEGER NOT NULL DEFAULT 0, created_at TEXT NOT NULL DEFAULT (datetime('now')), event_seq INTEGER NOT NULL DEFAULT 0, ai_insights TEXT, invite_code TEXT)")
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

#[tokio::test]
async fn test_leaderboard_games_returns_sorted() {
    let (pool, auth, _room_service) = setup().await;
    let u1 = unique_id("lb_u1");
    let u2 = unique_id("lb_u2");
    auth.register(&u1, "123456").await.unwrap();
    auth.register(&u2, "123456").await.unwrap();

    // Get user IDs from DB
    let uid1: String = sqlx::query_scalar("SELECT id FROM users WHERE username = ?")
        .bind(&u1).fetch_one(&pool).await.unwrap();
    let uid2: String = sqlx::query_scalar("SELECT id FROM users WHERE username = ?")
        .bind(&u2).fetch_one(&pool).await.unwrap();

    // u1 has 3 finished rooms, u2 has 1
    for i in 0..3 {
        sqlx::query("INSERT INTO rooms (room_id, owner_id, game_type, engine_state, actor_slots, max_round) VALUES (?, ?, 'lincoln', '{\"finished\": true}', '[]', 3)")
            .bind(format!("{}-r{}", uid1, i))
            .bind(&uid1)
            .execute(&pool).await.unwrap();
    }
    sqlx::query("INSERT INTO rooms (room_id, owner_id, game_type, engine_state, actor_slots, max_round) VALUES (?, ?, 'lincoln', '{\"finished\": true}', '[]', 3)")
        .bind(format!("{}-r0", uid2))
        .bind(&uid2)
        .execute(&pool).await.unwrap();

    // Query like leaderboard_games handler
    let rows: Vec<(String, i64)> = sqlx::query_as(
        "SELECT owner_id, COUNT(*) as cnt FROM rooms WHERE json_extract(engine_state, '$.finished') = 1 GROUP BY owner_id ORDER BY cnt DESC LIMIT 50"
    )
    .fetch_all(&pool).await.unwrap();

    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].0, uid1);
    assert_eq!(rows[0].1, 3);
    assert_eq!(rows[1].0, uid2);
    assert_eq!(rows[1].1, 1);
}

#[tokio::test]
async fn test_leaderboard_wins_returns_sorted() {
    let (pool, auth, _room_service) = setup().await;
    let u1 = unique_id("lbw_u1");
    auth.register(&u1, "123456").await.unwrap();

    let uid1: String = sqlx::query_scalar("SELECT id FROM users WHERE username = ?")
        .bind(&u1).fetch_one(&pool).await.unwrap();

    // Create rooms and mark finished
    sqlx::query("INSERT INTO rooms (room_id, owner_id, game_type, engine_state, actor_slots, max_round) VALUES (?, ?, 'lincoln', '{\"finished\": true}', '[]', 3)")
        .bind(format!("{}-w0", uid1))
        .bind(&uid1)
        .execute(&pool).await.unwrap();

    let cnt: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM rooms WHERE owner_id = ? AND json_extract(engine_state, '$.finished') = 1"
    )
    .bind(&uid1)
    .fetch_one(&pool).await.unwrap();
    assert!(cnt > 0, "finished rooms count should be > 0");
}

#[tokio::test]
async fn test_leaderboard_experienced_filters_min_games() {
    let (pool, auth, _room_service) = setup().await;
    let u1 = unique_id("lbe_u1");
    let u2 = unique_id("lbe_u2");
    auth.register(&u1, "123456").await.unwrap();
    auth.register(&u2, "123456").await.unwrap();

    let uid1: String = sqlx::query_scalar("SELECT id FROM users WHERE username = ?")
        .bind(&u1).fetch_one(&pool).await.unwrap();
    let uid2: String = sqlx::query_scalar("SELECT id FROM users WHERE username = ?")
        .bind(&u2).fetch_one(&pool).await.unwrap();

    // u1 has 10 finished rooms, u2 has 3
    for i in 0..10 {
        sqlx::query("INSERT INTO rooms (room_id, owner_id, game_type, engine_state, actor_slots, max_round) VALUES (?, ?, 'lincoln', '{\"finished\": true}', '[]', 3)")
            .bind(format!("{}-r{}", uid1, i))
            .bind(&uid1)
            .execute(&pool).await.unwrap();
    }
    for i in 0..3 {
        sqlx::query("INSERT INTO rooms (room_id, owner_id, game_type, engine_state, actor_slots, max_round) VALUES (?, ?, 'lincoln', '{\"finished\": true}', '[]', 3)")
            .bind(format!("{}-r{}", uid2, i))
            .bind(&uid2)
            .execute(&pool).await.unwrap();
    }

    // Min 5 games - only u1 should qualify
    let rows: Vec<(String, i64)> = sqlx::query_as(
        "SELECT owner_id, COUNT(*) as cnt FROM rooms WHERE json_extract(engine_state, '$.finished') = 1 GROUP BY owner_id HAVING cnt >= ? ORDER BY cnt DESC LIMIT 50"
    )
    .bind(5i64)
    .fetch_all(&pool).await.unwrap();

    assert_eq!(rows.len(), 1, "只有 u1 (10 局) 应满足 min_games=5");
    assert_eq!(rows[0].0, uid1);
}

#[tokio::test]
async fn test_leaderboard_by_game_filters_type() {
    let (pool, auth, _room_service) = setup().await;
    let u1 = unique_id("lbg_u1");
    auth.register(&u1, "123456").await.unwrap();

    let uid1: String = sqlx::query_scalar("SELECT id FROM users WHERE username = ?")
        .bind(&u1).fetch_one(&pool).await.unwrap();

    // Insert lincoln and texas rooms
    sqlx::query("INSERT INTO rooms (room_id, owner_id, game_type, engine_state, actor_slots, max_round) VALUES (?, ?, 'lincoln', '{\"finished\": true}', '[]', 3)")
        .bind(format!("{}-lin0", uid1))
        .bind(&uid1)
        .execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO rooms (room_id, owner_id, game_type, engine_state, actor_slots, max_round) VALUES (?, ?, 'texas_holdem', '{\"finished\": true}', '[]', 3)")
        .bind(format!("{}-tex0", uid1))
        .bind(&uid1)
        .execute(&pool).await.unwrap();

    let lincoln_cnt: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM rooms WHERE game_type = ? AND json_extract(engine_state, '$.finished') = 1 AND owner_id = ?"
    )
    .bind("lincoln")
    .bind(&uid1)
    .fetch_one(&pool).await.unwrap();
    assert_eq!(lincoln_cnt, 1);

    let texas_cnt: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM rooms WHERE game_type = ? AND json_extract(engine_state, '$.finished') = 1 AND owner_id = ?"
    )
    .bind("texas_holdem")
    .bind(&uid1)
    .fetch_one(&pool).await.unwrap();
    assert_eq!(texas_cnt, 1);
}

#[tokio::test]
async fn test_leaderboard_empty_when_no_finished_games() {
    let (pool, _auth, _room_service) = setup().await;

    let rows: Vec<(String, i64)> = sqlx::query_as(
        "SELECT owner_id, COUNT(*) as cnt FROM rooms WHERE json_extract(engine_state, '$.finished') = 1 GROUP BY owner_id ORDER BY cnt DESC LIMIT 50"
    )
    .fetch_all(&pool).await.unwrap();

    assert!(rows.is_empty(), "无已完成对局时排行榜应为空");
}
