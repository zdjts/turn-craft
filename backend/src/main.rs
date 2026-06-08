use std::sync::Arc;
use tokio::net::TcpListener;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

// 假设这些是我们之前共同编写的模块导入
mod ai;
mod app;
mod games;
mod handlers;
mod network;

use crate::network::manager::RoomManager;
use crate::network::room::AiTask;

use self::{
    ai::listener::AiWorker,
    app::{AppState, build_router},
}; // 上一步编写的 WebSocket 网关

#[tokio::main]
async fn main() {
    // 初始化工业级统一日志追踪与监控系统
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("🚀 大模型高并发辩论游戏服务器正在初始化核心基建...");

    // 5. 撕开全局 AI 异步任务通信总线管道（容量 1024）
    let (ai_tx, ai_rx) = tokio::sync::mpsc::channel::<AiTask>(1024);

    // 6. 激活后台常驻的 AI 任务消费者（AiWorker Pipeline 独立常驻协程）
    let ai_worker = AiWorker::new();
    tokio::spawn(async move {
        tracing::info!("🤖 [AI Worker Pipeline] 异步常驻后台线程已成功激活，开始监听全局任务...");
        ai_worker.start_consuming(ai_rx).await;
    });

    // 7. 实例化全局动态房间管理器
    let room_manager = Arc::new(RoomManager::new());

    // 8. 熔铸并注入全局统一上下文应用状态
    let app_state = AppState {
        room_manager,
        ai_tx,
    };

    // 9. 编织完整路由网格并挂载跨域中间件
    let app = build_router(app_state);

    // 10. 绑定端口并对外开放接入
    let addr = "0.0.0.0:8080";
    let listener = TcpListener::bind(addr).await.unwrap();
    tracing::info!("🔥 服务器成功起航！正在高效监听：http://{}", addr);

    // 正式启动无状态、非阻塞的高并发 Axum 网络网络内核
    axum::serve(listener, app).await.unwrap();
}
