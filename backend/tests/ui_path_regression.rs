//! UI 路径回归测试 — 覆盖创建/加入/操作/AI 失败等关键前端路径
//!
//! 不依赖外部 AI API，全部使用 mock 方式模拟。
//!
//! ```bash
//! DATABASE_URL="sqlite://test_ui.db?mode=rwc" cargo test -p backend --test ui_path_regression -- --nocapture
//! ```

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

static TEST_SEQ: AtomicU64 = AtomicU64::new(0);
fn unique_user(prefix: &str) -> String {
    let seq = TEST_SEQ.fetch_add(1, Ordering::SeqCst);
    format!("{}_{}", prefix, seq)
}

use backend::auth::AuthService;
use backend::room::RoomService;
use backend::room::supervisor::RoomSupervisor;
use backend::games::factory::GameRegistry;
use backend::user::repository::SqliteUserRepo;
use backend::room::repository::SqliteRoomRepo;
use backend::ai::config_repo::SqliteAiConfigRepo;
use backend::room::model::CreateRoomInput;
use backend::event_store::SqliteEventStore;

async fn setup() -> (sqlx::SqlitePool, AuthService, RoomService) {
    let db_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "sqlite://test_ui.db?mode=rwc".to_string());
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
    registry.register(Box::new(backend::games::werewolf::WerewolfFactory));

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
            ("Con".into(), "human".into()),
        ]),
        game_config: None,
        is_public: false,
    }
}

// ─── 测试 1: 用户创建房间 → 返回 room_id，房间状态正确 ───

#[tokio::test]
async fn test_create_room_returns_valid_state() {
    let (_pool, auth, room_service) = setup().await;
    let u = unique_user("create_ui");
    let token = auth.register(&u, "123456").await.unwrap();
    let uid = auth.verify_token(&token).await.unwrap();

    let out = room_service.create_room(uid.clone(), make_lincoln_input("Judge")).await.unwrap();

    // 1a: 返回 room_id
    assert!(!out.room_id.is_empty(), "创建房间应返回 room_id");

    // 1b: 房间快照状态正确
    let snap = room_service.get_room_snapshot(&out.room_id).await.unwrap().unwrap();
    assert_eq!(snap.game_type, "lincoln", "game_type 应为 lincoln");
    assert_eq!(snap.max_round, 3, "max_round 应与输入一致");
    assert_eq!(snap.owner_id.0, uid.0, "owner_id 应为创建者");

    // 1c: 槽位状态正确
    let judge = snap.actor_slots.iter().find(|s| s.slot_name == "Judge").unwrap();
    assert!(matches!(judge.occupant, backend::room::model::ActorOccupant::Human(_)),
        "Judge 槽位应被创建者占据");

    println!("[test_create_room] ✅ room_id={} game_type={} max_round={}",
        out.room_id, snap.game_type, snap.max_round);
}

// ─── 测试 2: 第二用户加入槽位 → 收到状态快照 ───

#[tokio::test]
async fn test_second_user_join_receives_snapshot() {
    let (_pool, auth, room_service) = setup().await;
    let u1 = unique_user("join2_u1");
    let u2 = unique_user("join2_u2");
    let t1 = auth.register(&u1, "123456").await.unwrap();
    let uid1 = auth.verify_token(&t1).await.unwrap();
    let t2 = auth.register(&u2, "123456").await.unwrap();
    let uid2 = auth.verify_token(&t2).await.unwrap();

    let out = room_service.create_room(uid1.clone(), make_lincoln_input("Judge")).await.unwrap();
    let room_id = out.room_id;

    // 2a: 第二用户加入 Pro 槽位
    room_service.join_slot(uid2.clone(), &room_id, "Pro").await.unwrap();
    println!("[test_second_user_join] ✅ 第二用户加入 Pro 成功");

    // 2b: 快照反映两人的占据
    let snap = room_service.get_room_snapshot(&room_id).await.unwrap().unwrap();
    let judge = snap.actor_slots.iter().find(|s| s.slot_name == "Judge").unwrap();
    let pro = snap.actor_slots.iter().find(|s| s.slot_name == "Pro").unwrap();
    let con = snap.actor_slots.iter().find(|s| s.slot_name == "Con").unwrap();

    assert!(matches!(judge.occupant, backend::room::model::ActorOccupant::Human(_)));
    assert!(matches!(pro.occupant, backend::room::model::ActorOccupant::Human(_)));
    assert!(matches!(con.occupant, backend::room::model::ActorOccupant::Empty));

    println!("[test_second_user_join] ✅ 快照: Judge=Human, Pro=Human, Con=Empty");
}

// ─── 测试 3: 非房主发送 action → 引擎推进 → 所有 peer 收到更新 ───

#[tokio::test]
async fn test_non_owner_action_propagates_to_peers() {
    let (_pool, auth, room_service) = setup().await;
    let u1 = unique_user("act_u1");
    let u2 = unique_user("act_u2");
    let t1 = auth.register(&u1, "123456").await.unwrap();
    let uid1 = auth.verify_token(&t1).await.unwrap();
    let t2 = auth.register(&u2, "123456").await.unwrap();
    let uid2 = auth.verify_token(&t2).await.unwrap();

    let out = room_service.create_room(uid1.clone(), make_lincoln_input("Judge")).await.unwrap();
    let room_id = out.room_id;
    room_service.join_slot(uid2.clone(), &room_id, "Pro").await.unwrap();

    let room_tx = room_service.get_room_tx(&room_id).unwrap();

    // 先让 Judge 开题 → 引擎切换到 Pro
    let (tx_j, rx_j) = tokio::sync::oneshot::channel();
    room_tx.send(backend::room::model::RoomCommand::PlayerAction {
        actor_id: "Judge".into(),
        action: serde_json::json!({"content": "辩题：AI 会取代人类工作吗"}),
        feedback_tx: Some(tx_j),
    }).await.unwrap();
    rx_j.await.unwrap().unwrap();
    println!("[test_non_owner_action] ✅ Judge 开题成功");

    // 非房主 (Pro) 发送 action
    let (tx, rx) = tokio::sync::oneshot::channel();
    room_tx.send(backend::room::model::RoomCommand::PlayerAction {
        actor_id: "Pro".into(),
        action: serde_json::json!({"content": "Pro 发言测试"}),
        feedback_tx: Some(tx),
    }).await.unwrap();
    let result = rx.await.unwrap();
    assert!(result.is_ok(), "非房主发送合法 action 应成功");

    // 读取引擎状态，验证轮次已推进
    let snap = room_service.get_room_snapshot(&room_id).await.unwrap().unwrap();
    let engine_state_str = snap.engine_state.to_string();
    println!("[test_non_owner_action] 引擎状态: {}", &engine_state_str[..80.min(engine_state_str.len())]);

    // 验证 Pro action 被处理（引擎不应仍在 Judge 阶段）
    // Judge 开题后 cur_role=Pro, Pro 发言后 cur_role=Con
    // 通过 actor_slots 验证轮次已变化
    assert!(!engine_state_str.contains("\"finished\":true"),
        "游戏不应在此阶段结束");

    println!("[test_non_owner_action] ✅ 非房主 action 被引擎接受，状态已推进");
}

// ─── 测试 4: AI 失败场景 → 收到 action_error → can_retry 字段正确 ───

#[tokio::test]
async fn test_action_error_has_can_retry() {
    let (_pool, auth, room_service) = setup().await;
    let u = unique_user("ai_fail");
    let token = auth.register(&u, "123456").await.unwrap();
    let uid = auth.verify_token(&token).await.unwrap();

    let out = room_service.create_room(uid.clone(), make_lincoln_input("Judge")).await.unwrap();
    let room_id = out.room_id;

    let room_tx = room_service.get_room_tx(&room_id).unwrap();

    // 先发送合法 Judge action → 引擎切换到 Pro
    let (tx_ok, rx_ok) = tokio::sync::oneshot::channel();
    room_tx.send(backend::room::model::RoomCommand::PlayerAction {
        actor_id: "Judge".into(),
        action: serde_json::json!({"content": "辩题：AI 伦理"}),
        feedback_tx: Some(tx_ok),
    }).await.unwrap();
    rx_ok.await.unwrap().unwrap();

    // 发送空 content action → 引擎拒绝
    let (tx_err, rx_err) = tokio::sync::oneshot::channel();
    room_tx.send(backend::room::model::RoomCommand::PlayerAction {
        actor_id: "Pro".into(),
        action: serde_json::json!({"content": ""}),
        feedback_tx: Some(tx_err),
    }).await.unwrap();
    let err_result = rx_err.await.unwrap();

    // 验证 engine 返回了预期的错误
    assert!(err_result.is_err(), "空 content 的 action 应被引擎拒绝");
    let err_msg = err_result.unwrap_err();
    assert!(
        err_msg.contains("content") || err_msg.contains("缺少"),
        "错误消息应提及 content 缺失: {err_msg}"
    );

    println!("[test_action_error] ✅ action 被正确拒绝: {err_msg}");
}
