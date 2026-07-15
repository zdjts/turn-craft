//! 核心回归测试 — 多用户并发、断线重连、AI 重试/跳过、房间恢复、事件落库
//!
//! ```bash
//! DATABASE_URL="sqlite://test_core.db?mode=rwc" cargo test -p backend --test core_regression -- --nocapture
//! ```

use std::sync::Arc;
use std::collections::HashMap;
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
        .unwrap_or_else(|_| "sqlite://test_core.db?mode=rwc".to_string());
    let pool = sqlx::SqlitePool::connect(&db_url).await.unwrap();

    // 重建表保证 schema 最新
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
    sqlx::query("CREATE TABLE IF NOT EXISTS rooms (room_id TEXT PRIMARY KEY, owner_id TEXT NOT NULL, game_type TEXT NOT NULL, engine_state TEXT NOT NULL, actor_slots TEXT NOT NULL, ai_configs TEXT NOT NULL DEFAULT '{}', max_round INTEGER NOT NULL DEFAULT 16, game_config TEXT NOT NULL DEFAULT '{}', is_public INTEGER NOT NULL DEFAULT 0, created_at TEXT NOT NULL DEFAULT (datetime('now')), event_seq INTEGER NOT NULL DEFAULT 0)")
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

// ─── 测试 1: 多用户并发加入 ───

#[tokio::test]
async fn test_multi_user_join() {
    let (_pool, auth, room_service) = setup().await;
    let u1 = unique_user("multi_u1");
    let u2 = unique_user("multi_u2");
    let token1 = auth.register(&u1, "123456").await.unwrap();
    let uid1 = auth.verify_token(&token1).await.unwrap();
    let token2 = auth.register(&u2, "123456").await.unwrap();
    let uid2 = auth.verify_token(&token2).await.unwrap();

    let out = room_service.create_room(uid1.clone(), make_lincoln_input("Judge")).await.unwrap();
    let room_id = out.room_id;

    // 用户 2 加入 Pro 槽位
    room_service.join_slot(uid2.clone(), &room_id, "Pro").await.unwrap();

    // 从 DB 验证两个槽位都被正确占据
    let snap = room_service.get_room_snapshot(&room_id).await.unwrap().unwrap();
    let judge_slot = snap.actor_slots.iter().find(|s| s.slot_name == "Judge").unwrap();
    let pro_slot = snap.actor_slots.iter().find(|s| s.slot_name == "Pro").unwrap();
    let con_slot = snap.actor_slots.iter().find(|s| s.slot_name == "Con").unwrap();

    use backend::room::model::ActorOccupant;
    assert!(matches!(judge_slot.occupant, ActorOccupant::Human(ref u) if u.0 == uid1.0));
    assert!(matches!(pro_slot.occupant, ActorOccupant::Human(ref u) if u.0 == uid2.0));
    assert!(matches!(con_slot.occupant, ActorOccupant::Ai));

    println!("[test_multi_user_join] ✅ 两个玩家各占 Judge + Pro，AI 占 Con");
}

// ─── 测试 2: 重复加入同一槽位 ───

#[tokio::test]
async fn test_slot_rejoin_same_user() {
    let (_pool, auth, room_service) = setup().await;
    let token = auth.register(&unique_user("rejoin"), "123456").await.unwrap();
    let uid = auth.verify_token(&token).await.unwrap();

    let out = room_service.create_room(uid.clone(), make_lincoln_input("Judge")).await.unwrap();
    let room_id = out.room_id;

    // 再次 join 同一槽位 → 应返回 Ok (幂等)
    room_service.join_slot(uid.clone(), &room_id, "Judge").await.unwrap();

    // 槽位仍为 Human(uid)
    let snap = room_service.get_room_snapshot(&room_id).await.unwrap().unwrap();
    let judge = snap.actor_slots.iter().find(|s| s.slot_name == "Judge").unwrap();
    use backend::room::model::ActorOccupant;
    assert!(matches!(judge.occupant, ActorOccupant::Human(ref u) if u.0 == uid.0));
    println!("[test_slot_rejoin_same_user] ✅ 同一用户重复 join 返回 Ok");
}

// ─── 测试 3: 房间恢复 ───

#[tokio::test]
async fn test_room_restore() {
    let (_pool, auth, room_service) = setup().await;
    let token = auth.register(&unique_user("restore"), "123456").await.unwrap();
    let uid = auth.verify_token(&token).await.unwrap();

    let out = room_service.create_room(uid.clone(), make_lincoln_input("Judge")).await.unwrap();
    let room_id = out.room_id;

    // 恢复（启动时自动恢复所有非 finished 房间）
    room_service.restore_all().await.unwrap();

    // 房间应仍在活跃列表
    let room_tx = room_service.get_room_tx(&room_id);
    assert!(room_tx.is_some(), "恢复后房间应在活跃列表中");

    println!("[test_room_restore] ✅ 房间恢复后仍在活跃列表");
}

// ─── 测试 4: 事件落库 ───

#[tokio::test]
async fn test_event_persistence() {
    use backend::event_store::EventStore;
    let (pool, auth, room_service) = setup().await;
    let token = auth.register(&unique_user("event"), "123456").await.unwrap();
    let uid = auth.verify_token(&token).await.unwrap();

    let out = room_service.create_room(uid.clone(), make_lincoln_input("Judge")).await.unwrap();
    let room_id = out.room_id;

    // 触发一个 action：通过 room_tx 发送 PlayerAction
    let room_tx = room_service.get_room_tx(&room_id).unwrap();
    let action = serde_json::json!({"content": "辩题为：AI 是否应该拥有权利"});
    let (tx, rx) = tokio::sync::oneshot::channel();
    room_tx.send(backend::room::model::RoomCommand::PlayerAction {
        actor_id: "Judge".into(),
        action,
        feedback_tx: Some(tx),
    }).await.unwrap();
    let _ = rx.await; // 等待引擎处理

    // 给异步 effect handler 一点时间写入事件
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // 验证事件已落库
    let event_store = SqliteEventStore::new(pool.clone());
    let events = event_store.list_events(&room_id, 0, 100).await.unwrap();
    let event_types: Vec<&str> = events.iter().map(|e| e.event_type.as_str()).collect();
    println!("[test_event_persistence] 事件类型: {:?}", event_types);

    assert!(events.iter().any(|e| e.event_type == "action"), "应有 action 事件");
    assert!(events.iter().any(|e| e.event_type == "state_change"), "应有 state_change 事件");
    println!("[test_event_persistence] ✅ action + state_change 均已落库");
}

// ─── 测试 5: 不支持的 slot 拒绝加入 ───

#[tokio::test]
async fn test_join_invalid_slot() {
    let (_pool, auth, room_service) = setup().await;
    let token = auth.register(&unique_user("inv_slot"), "123456").await.unwrap();
    let uid = auth.verify_token(&token).await.unwrap();
    let token2 = auth.register(&unique_user("inv_slot2"), "123456").await.unwrap();
    let uid2 = auth.verify_token(&token2).await.unwrap();

    let out = room_service.create_room(uid.clone(), make_lincoln_input("Judge")).await.unwrap();
    let room_id = out.room_id;

    // 加入不存在的槽位
    let r = room_service.join_slot(uid2.clone(), &room_id, "NonExistent").await;
    assert!(r.is_err(), "不存在的槽位应返回错误");

    // 加入 AI 槽位
    let r = room_service.join_slot(uid2.clone(), &room_id, "Con").await;
    assert!(r.is_err(), "AI 槽位应返回错误");

    println!("[test_join_invalid_slot] ✅ 非法槽位被正确拒绝");
}

// ─── 测试 6: 房间删除 ───

#[tokio::test]
async fn test_delete_room() {
    let (_pool, auth, room_service) = setup().await;
    let token = auth.register(&unique_user("delete"), "123456").await.unwrap();
    let uid = auth.verify_token(&token).await.unwrap();
    let token2 = auth.register(&unique_user("delete2"), "123456").await.unwrap();
    let uid2 = auth.verify_token(&token2).await.unwrap();

    let out = room_service.create_room(uid.clone(), make_lincoln_input("Judge")).await.unwrap();
    let room_id = out.room_id;

    // 非房主删除 → 拒绝
    let r = room_service.delete_room(uid2, &room_id).await;
    assert!(r.is_err(), "非房主删除应返回错误");

    // 房主删除 → 成功
    room_service.delete_room(uid, &room_id).await.unwrap();
    assert!(room_service.get_room_tx(&room_id).is_none(), "删除后房间应不在活跃列表");

    // DB 也应已删除
    let snap = room_service.get_room_snapshot(&room_id).await.unwrap();
    assert!(snap.is_none(), "删除后 DB 应无记录");

    println!("[test_delete_room] ✅ 权限校验 + 删除成功");
}

// ─── 测试 7: 事件闭环 — 回放确认 ───

#[tokio::test]
async fn test_event_replay_order() {
    use backend::event_store::EventStore;
    let (pool, auth, room_service) = setup().await;
    let token = auth.register(&unique_user("replay"), "123456").await.unwrap();
    let uid = auth.verify_token(&token).await.unwrap();

    let out = room_service.create_room(uid.clone(), make_lincoln_input("Judge")).await.unwrap();
    let room_id = out.room_id;

    // 发送 Judge 的辩题 action
    let room_tx = room_service.get_room_tx(&room_id).unwrap();
    let action1 = serde_json::json!({"content": "辩题：AI 伦理"});
    let (tx1, rx1) = tokio::sync::oneshot::channel();
    room_tx.send(backend::room::model::RoomCommand::PlayerAction {
        actor_id: "Judge".into(),
        action: action1,
        feedback_tx: Some(tx1),
    }).await.unwrap();
    rx1.await.unwrap().unwrap();

    // 发送 Pro 的 response（AI 会触发，但 Pro 是 human 所以直接发）
    let room_tx = room_service.get_room_tx(&room_id).unwrap();
    let action2 = serde_json::json!({"content": "正方发言..."});
    let (tx2, rx2) = tokio::sync::oneshot::channel();
    room_tx.send(backend::room::model::RoomCommand::PlayerAction {
        actor_id: "Pro".into(),
        action: action2,
        feedback_tx: Some(tx2),
    }).await.unwrap();
    rx2.await.unwrap().unwrap();

    // 等待事件完全落库（最多 3 秒）
    let event_store = SqliteEventStore::new(pool.clone());
    let mut events;
    let mut attempts = 0;
    loop {
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        events = event_store.list_events(&room_id, 0, 100).await.unwrap();
        if events.len() >= 4 || attempts > 15 { break; }
        attempts += 1;
    }

    let seqs: Vec<i64> = events.iter().map(|e| e.seq).collect();
    let types: Vec<&str> = events.iter().map(|e| e.event_type.as_str()).collect();
    println!("[test_event_replay] seqs={:?} types={:?} (attempts={})", seqs, types, attempts);

    assert!(seqs.len() >= 4, "应有至少 4 个事件（2 action + 2 state_change），实际 {}", seqs.len());

    // 验证顺序递增
    for i in 1..seqs.len() {
        assert!(seqs[i] > seqs[i-1], "事件 seq 必须严格递增");
    }

    // 验证包含关键事件类型
    let type_set: std::collections::HashSet<&str> = types.iter().copied().collect();
    assert!(type_set.contains("action"), "应有 action 事件");
    assert!(type_set.contains("state_change"), "应有 state_change 事件");

    // 验证 event_seq 同步
    let current_seq = event_store.current_seq(&room_id).await.unwrap();
    assert_eq!(current_seq, *seqs.last().unwrap(), "rooms.event_seq 应与最大事件 seq 一致");

    println!("[test_event_replay] ✅ 事件 seq 递增 + event_seq 同步正确");
}
