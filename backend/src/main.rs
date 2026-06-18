use std::sync::Arc;
use tokio::net::TcpListener;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod ai;
mod app;
mod auth;
mod config;
mod error;
mod games;
mod handlers;
mod room;
mod user;

use crate::room::model::AiTask;
use crate::ai::listener::AiWorker;
use crate::app::{AppState, build_router};
use crate::config::CONFIG;

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("🚀 大模型高并发辩论游戏服务器正在初始化核心基建...");

    // 1. AI 任务队列通道与 Worker 后台线程
    let (ai_tx, ai_rx) = tokio::sync::mpsc::channel::<AiTask>(CONFIG.ai_task_capacity);
    let ai_worker = AiWorker::new();
    tokio::spawn(async move {
        tracing::info!("🤖 [AI Worker Pipeline] 异步常驻后台线程已成功激活，开始监听全局任务...");
        ai_worker.start_consuming(ai_rx).await;
    });

    // 2. 建立数据库连接池并运行迁移
    let pool = sqlx::SqlitePool::connect(&CONFIG.database_url).await.expect("无法连接到 SQLite 数据库");
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("数据库迁移执行失败");

    // Initialize system user and __defaults__ room for global AI config defaults
    sqlx::query("INSERT OR IGNORE INTO users (id, username, password_hash) VALUES ('system', 'system', 'system')")
        .execute(&pool)
        .await
        .expect("Failed to initialize system user");
    sqlx::query("INSERT OR IGNORE INTO rooms (room_id, owner_id, game_type, engine_state, actor_slots) VALUES ('__defaults__', 'system', 'system', '{}', '{}')")
        .execute(&pool)
        .await
        .expect("Failed to initialize __defaults__ room");


    // 3. 初始化 Repository
    let user_repo = Arc::new(crate::user::repository::SqliteUserRepo::new(pool.clone()))
        as Arc<dyn crate::user::repository::UserRepository>;
    let room_repo = Arc::new(crate::room::repository::SqliteRoomRepo::new(pool.clone()))
        as Arc<dyn crate::room::repository::RoomRepository>;
    let ai_config_repo = Arc::new(crate::ai::config_repo::SqliteAiConfigRepo::new(pool.clone()))
        as Arc<dyn crate::ai::config_repo::AiConfigRepository>;

    // 4. 初始化 GameRegistry 并注册游戏工厂
    let mut game_registry = crate::games::GameRegistry::new();
    game_registry.register(Box::new(crate::games::lincoln::LincolnFactory));
    game_registry.register(Box::new(crate::games::texas_holdem::TexasHoldemFactory));
    let game_registry = Arc::new(game_registry);

    // 5. 初始化 Service 与 Supervisor
    let auth_service = Arc::new(crate::auth::AuthService::new(
        user_repo,
        &CONFIG.jwt_secret,
        CONFIG.jwt_expires_in_secs,
    ));
    let supervisor = crate::room::supervisor::RoomSupervisor::new();
    tokio::spawn(supervisor.clone().run());

    let room_service = Arc::new(crate::room::RoomService::new(
        room_repo,
        ai_config_repo.clone(),
        ai_tx.clone(),
        supervisor,
        game_registry,
    ));

    // 6. 从数据库恢复之前所有的活跃房间
    tracing::info!("正在从数据库恢复历史房间...");
    if let Err(e) = room_service.restore_all().await {
        tracing::error!(error = ?e, "从数据库恢复房间失败");
    }

    // 7. 构建应用 State 与 Axum 路由
    let app_state = AppState {
        auth_service,
        room_service,
        ai_service: Arc::new(crate::ai::AIService::new(ai_config_repo)),
    };

    let app = build_router(app_state);

    let addr = format!("{}:{}", CONFIG.server_host, CONFIG.server_port);
    let listener = TcpListener::bind(&addr).await.unwrap();
    tracing::info!("🔥 服务器成功起航！正在高效监听：http://{}", addr);

    axum::serve(listener, app).await.unwrap();
}
