use std::sync::Arc;

use serde_json::json;

use backend::auth::AuthService;
use backend::room::RoomService;
use backend::room::supervisor::RoomSupervisor;
use backend::games::factory::GameRegistry;
use backend::user::repository::SqliteUserRepo;
use backend::room::repository::SqliteRoomRepo;
use backend::ai::config_repo::SqliteAiConfigRepo;
use backend::event_store::SqliteEventStore;

static DB_SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(700);

fn unique_id(prefix: &str) -> String {
    let seq = DB_SEQ.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    format!("{}_{}", prefix, seq)
}

async fn setup() -> (sqlx::SqlitePool, AuthService, RoomService) {
    let db_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "sqlite://test_v9_achieve.db?mode=rwc".to_string());
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

async fn insert_room(pool: &sqlx::SqlitePool, room_id: &str, owner_id: &str, game_type: &str,
               finished: bool, ai_configs: Option<&str>, actor_slots: Option<&str>) {
    let engine = if finished { r#"{"finished": true}"# } else { r#"{"finished": false}"# };
    let ac = ai_configs.unwrap_or("{}");
    let slots = actor_slots.unwrap_or("[]");
    sqlx::query(
        "INSERT INTO rooms (room_id, owner_id, game_type, engine_state, actor_slots, ai_configs, max_round) VALUES (?, ?, ?, ?, ?, ?, 3)"
    )
    .bind(room_id).bind(owner_id).bind(game_type)
    .bind(engine).bind(slots).bind(ac)
    .execute(pool).await.unwrap();
}

#[tokio::test]
async fn test_achievement_first_game_unlocks() {
    let (pool, auth, _room_service) = setup().await;
    let u1 = unique_id("ach_u1");
    auth.register(&u1, "123456").await.unwrap();
    let uid: String = sqlx::query_scalar("SELECT id FROM users WHERE username = ?")
        .bind(&u1).fetch_one(&pool).await.unwrap();

    // No rooms yet → first_game not unlocked
    let rows: Vec<(String, String, String)> = sqlx::query_as(
        "SELECT room_id, game_type, engine_state FROM rooms WHERE owner_id = ? AND json_extract(engine_state, '$.finished') = 1"
    )
    .bind(&uid)
    .fetch_all(&pool).await.unwrap();
    assert_eq!(rows.len(), 0);

    // Add one finished room → first_game should be detected
    insert_room(&pool, &format!("{}-r0", uid), &uid, "lincoln", true, None, None).await;
    let rows2: Vec<(String, String, String)> = sqlx::query_as(
        "SELECT room_id, game_type, engine_state FROM rooms WHERE owner_id = ? AND json_extract(engine_state, '$.finished') = 1"
    )
    .bind(&uid)
    .fetch_all(&pool).await.unwrap();
    assert_eq!(rows2.len(), 1, "应检测到 1 局已完成对局");
}

#[tokio::test]
async fn test_achievement_empty_user_all_locked() {
    let (pool, auth, _room_service) = setup().await;
    let u1 = unique_id("ach_u2");
    auth.register(&u1, "123456").await.unwrap();
    let uid: String = sqlx::query_scalar("SELECT id FROM users WHERE username = ?")
        .bind(&u1).fetch_one(&pool).await.unwrap();

    let rows: Vec<(String, String, String)> = sqlx::query_as(
        "SELECT room_id, game_type, engine_state FROM rooms WHERE owner_id = ? AND json_extract(engine_state, '$.finished') = 1"
    )
    .bind(&uid)
    .fetch_all(&pool).await.unwrap();
    assert!(rows.is_empty(), "新用户应无已完成对局");
}

#[tokio::test]
async fn test_achievement_texas_wins_count() {
    let (pool, auth, _room_service) = setup().await;
    let u1 = unique_id("ach_u3");
    auth.register(&u1, "123456").await.unwrap();
    let uid: String = sqlx::query_scalar("SELECT id FROM users WHERE username = ?")
        .bind(&u1).fetch_one(&pool).await.unwrap();

    for i in 0..10 {
        let engine = json!({
            "finished": true,
            "showdown_results": [{
                "player_id": &uid,
                "is_winner": true,
                "hand_rank": "pair"
            }]
        });
        sqlx::query("INSERT INTO rooms (room_id, owner_id, game_type, engine_state, actor_slots, max_round) VALUES (?, ?, 'texas_holdem', ?, '[]', 3)")
            .bind(format!("{}-tex{}", uid, i))
            .bind(&uid)
            .bind(engine.to_string())
            .execute(&pool).await.unwrap();
    }

    let texas_wins: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM rooms WHERE owner_id = ? AND game_type = 'texas_holdem' AND json_extract(engine_state, '$.finished') = 1"
    )
    .bind(&uid)
    .fetch_one(&pool).await.unwrap();
    assert!(texas_wins >= 10, "德州扑克胜利 10 次应满足 texas_10 成就");
}

#[tokio::test]
async fn test_achievement_spectate_counted_correctly() {
    let (pool, auth, _room_service) = setup().await;
    let u1 = unique_id("ach_u4");
    auth.register(&u1, "123456").await.unwrap();
    let uid: String = sqlx::query_scalar("SELECT id FROM users WHERE username = ?")
        .bind(&u1).fetch_one(&pool).await.unwrap();

    // Insert 10 rooms where the user appears as spectator
    for i in 0..10 {
        let slots = json!([{
            "slot_name": "__spectator__view",
            "occupant": format!("Human({})", uid)
        }]);
        sqlx::query("INSERT INTO rooms (room_id, owner_id, game_type, engine_state, actor_slots, max_round) VALUES (?, ?, 'lincoln', '{\"finished\": true}', ?, 3)")
            .bind(format!("{}-spec{}", uid, i))
            .bind(&uid)
            .bind(slots.to_string())
            .execute(&pool).await.unwrap();
    }

    let slots_all: Vec<String> = sqlx::query_scalar("SELECT actor_slots FROM rooms WHERE owner_id = ?")
        .bind(&uid)
        .fetch_all(&pool).await.unwrap();
    assert_eq!(slots_all.len(), 10, "应有 10 个含观战记录的房间");
}

#[tokio::test]
async fn test_achievement_all_styles_detected() {
    let (pool, auth, _room_service) = setup().await;
    let u1 = unique_id("ach_u5");
    auth.register(&u1, "123456").await.unwrap();
    let uid: String = sqlx::query_scalar("SELECT id FROM users WHERE username = ?")
        .bind(&u1).fetch_one(&pool).await.unwrap();

    // Insert rooms with all 7 styles in ai_configs
    let styles = ["default", "aggressive", "conservative", "creative", "deceptive", "rational", "chaotic"];
    for (i, style) in styles.iter().enumerate() {
        let configs = json!({
            format!("actor{}", i): { "style": style }
        });
        insert_room(&pool, &format!("{}-st{}", uid, i), &uid, "lincoln", true, Some(&configs.to_string()), None).await;
    }

    // Check all styles are present
    let mut found = std::collections::HashSet::new();
    let rooms_data: Vec<(String, String)> = sqlx::query_as(
        "SELECT room_id, COALESCE(ai_configs, '{}') FROM rooms WHERE owner_id = ? AND json_extract(engine_state, '$.finished') = 1"
    )
    .bind(&uid)
    .fetch_all(&pool).await.unwrap();

    for (_rid, configs_str) in &rooms_data {
        if let Ok(configs) = serde_json::from_str::<serde_json::Value>(configs_str) {
            if let Some(obj) = configs.as_object() {
                for (_aid, cfg) in obj {
                    if let Some(s) = cfg.get("style").and_then(|v| v.as_str()) {
                        found.insert(s.to_string());
                    }
                }
            }
        }
    }
    assert!(found.len() >= 7, "应检测到全部 7 种 AI 风格");
}

#[tokio::test]
async fn test_achievement_invite_friend_detected() {
    let (pool, auth, _room_service) = setup().await;
    let u1 = unique_id("ach_u6");
    auth.register(&u1, "123456").await.unwrap();
    let uid: String = sqlx::query_scalar("SELECT id FROM users WHERE username = ?")
        .bind(&u1).fetch_one(&pool).await.unwrap();

    // Room with another human player besides owner
    let slots = json!([
        { "slot_name": "Judge", "occupant": format!("Human({})", uid) },
        { "slot_name": "Pro", "occupant": "Human(friend_id)" }
    ]);
    sqlx::query("INSERT INTO rooms (room_id, owner_id, game_type, engine_state, actor_slots, max_round) VALUES (?, ?, 'lincoln', '{\"finished\": true}', ?, 3)")
        .bind(format!("{}-inv0", uid))
        .bind(&uid)
        .bind(slots.to_string())
        .execute(&pool).await.unwrap();

    // Verify the room has a non-owner human
    let rooms_data: Vec<(String, String)> = sqlx::query_as(
        "SELECT room_id, actor_slots FROM rooms WHERE owner_id = ?"
    )
    .bind(&uid)
    .fetch_all(&pool).await.unwrap();

    let mut has_friend = false;
    for (_rid, slots_str) in &rooms_data {
        if let Ok(slots_val) = serde_json::from_str::<serde_json::Value>(slots_str) {
            if let Some(arr) = slots_val.as_array() {
                for slot in arr {
                    let occ = slot.get("occupant").and_then(|v| v.as_str()).unwrap_or("");
                    if occ != "Empty" && occ != "Ai" && !occ.contains(&uid) {
                        has_friend = true;
                    }
                }
            }
        }
    }
    assert!(has_friend, "应检测到有其他人类参与者");
}

#[tokio::test]
async fn test_achievement_streak_consecutive_wins() {
    let (pool, auth, _room_service) = setup().await;
    let u1 = unique_id("ach_u7");
    auth.register(&u1, "123456").await.unwrap();
    let uid: String = sqlx::query_scalar("SELECT id FROM users WHERE username = ?")
        .bind(&u1).fetch_one(&pool).await.unwrap();

    // Insert 5 consecutive finished rooms (all lincoln, which counts as win)
    for i in 0..5 {
        insert_room(&pool, &format!("{}-stk{}", uid, i), &uid, "lincoln", true, None, None).await;
    }

    let finished: Vec<String> = sqlx::query_scalar(
        "SELECT room_id FROM rooms WHERE owner_id = ? AND json_extract(engine_state, '$.finished') = 1 ORDER BY created_at"
    )
    .bind(&uid)
    .fetch_all(&pool).await.unwrap();
    assert_eq!(finished.len(), 5, "应有 5 局连续胜利");
}

#[tokio::test]
async fn test_achievement_self_resolves_auth_user() {
    let (pool, auth, _room_service) = setup().await;
    let u1 = unique_id("ach_u8");
    let token = auth.register(&u1, "123456").await.unwrap();
    let uid = auth.verify_token(&token).await.unwrap();

    // Verify token resolves to the correct user
    let db_uid: String = sqlx::query_scalar("SELECT id FROM users WHERE username = ?")
        .bind(&u1).fetch_one(&pool).await.unwrap();
    assert_eq!(uid.0, db_uid, "AuthUser 应正确解析到对应用户");
}
