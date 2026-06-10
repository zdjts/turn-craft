use std::sync::Arc;

use axum::{
    Router,
    routing::{delete, get, post, put},
};
use dashmap::DashMap;
use tower_http::cors::CorsLayer;

use crate::{
    ai::env::AiConfig,
    handlers,
    network::{manager::RoomManager, room::AiTask},
    persistence::RoomSnapshot,
};

#[derive(Clone)]
pub struct AppState {
    pub room_manager: Arc<RoomManager>,
    pub ai_tx: tokio::sync::mpsc::Sender<AiTask>,
    /// 全局 AI 配置存储：key = "{room_id}/{actor_id}"，DashMap 自身并发安全
    pub ai_configs: Arc<DashMap<String, AiConfig>>,
    /// 房间快照存储：用于持久化到磁盘
    pub snapshots: Arc<DashMap<String, RoomSnapshot>>,
}

pub fn build_router(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(tower_http::cors::Any)
        .allow_methods(tower_http::cors::Any)
        .allow_headers(tower_http::cors::Any);

    Router::new()
        .route("/", get(handlers::health::index))
        .route("/health", get(handlers::health::health))
        .route("/rooms", post(handlers::room::create_room))
        .route("/rooms/{room_id}", delete(handlers::room::delete_room))
        .route(
            "/rooms/{room_id}/ai-config",
            get(handlers::ai_config::get_ai_config),
        )
        .route(
            "/rooms/{room_id}/ai-config/{actor_id}",
            put(handlers::ai_config::update_ai_config),
        )
        .route("/ws/{room_id}/{actor_id}", get(handlers::ws::ws_handler))
        .layer(cors)
        .with_state(state)
}
