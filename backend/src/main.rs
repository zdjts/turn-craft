use std::path::Path;
use std::sync::Arc;

use dashmap::DashMap;
use tokio::net::TcpListener;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod ai;
mod app;
mod games;
mod handlers;
mod network;

use crate::ai::env::AiConfig;
use crate::network::manager::RoomManager;
use crate::network::room::AiTask;

use self::{
    ai::listener::AiWorker,
    app::{AppState, build_router},
};

const CONFIG_FILE: &str = "ai_configs.json";

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("🚀 大模型高并发辩论游戏服务器正在初始化核心基建...");

    let (ai_tx, ai_rx) = tokio::sync::mpsc::channel::<AiTask>(1024);

    let ai_worker = AiWorker::new();
    tokio::spawn(async move {
        tracing::info!("🤖 [AI Worker Pipeline] 异步常驻后台线程已成功激活，开始监听全局任务...");
        ai_worker.start_consuming(ai_rx).await;
    });

    let room_manager = Arc::new(RoomManager::new());

    // 从文件加载持久化的 AI 配置
    let ai_configs = Arc::new(load_configs_from_file());

    let app_state = AppState {
        room_manager,
        ai_tx,
        ai_configs,
    };

    let app = build_router(app_state);

    let addr = "0.0.0.0:8080";
    let listener = TcpListener::bind(addr).await.unwrap();
    tracing::info!("🔥 服务器成功起航！正在高效监听：http://{}", addr);

    axum::serve(listener, app).await.unwrap();
}

fn load_configs_from_file() -> DashMap<String, AiConfig> {
    let map = DashMap::new();
    if !Path::new(CONFIG_FILE).exists() {
        tracing::info!(file = CONFIG_FILE, "AI 配置文件不存在，使用空配置");
        return map;
    }
    match std::fs::read_to_string(CONFIG_FILE) {
        Ok(json) => match serde_json::from_str::<std::collections::HashMap<String, AiConfig>>(&json)
        {
            Ok(parsed) => {
                let count = parsed.len();
                for (k, v) in parsed {
                    map.insert(k, v);
                }
                tracing::info!(file = CONFIG_FILE, count, "AI 配置已从文件加载");
            }
            Err(e) => {
                tracing::error!(file = CONFIG_FILE, error = %e, "AI 配置文件解析失败，忽略");
            }
        },
        Err(e) => {
            tracing::error!(file = CONFIG_FILE, error = %e, "读取 AI 配置文件失败");
        }
    }
    map
}

/// 将当前 AI 配置持久化到文件（在 handler 中调用）
pub fn save_configs_to_file(configs: &DashMap<String, AiConfig>) {
    let map: std::collections::HashMap<String, AiConfig> = configs
        .iter()
        .map(|entry| (entry.key().clone(), entry.value().clone()))
        .collect();
    match serde_json::to_string_pretty(&map) {
        Ok(json) => {
            if let Err(e) = std::fs::write(CONFIG_FILE, json) {
                tracing::error!(file = CONFIG_FILE, error = %e, "写入 AI 配置文件失败");
            }
        }
        Err(e) => {
            tracing::error!(error = %e, "序列化 AI 配置失败");
        }
    }
}
