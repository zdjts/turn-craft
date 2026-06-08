use std::sync::Arc;

use axum::{
    Router,
    routing::{delete, get, post},
};
use tower_http::cors::CorsLayer;

use crate::{
    handlers,
    network::{manager::RoomManager, room::AiTask},
};

#[derive(Clone)]
pub struct AppState {
    pub room_manager: Arc<RoomManager>,
    pub ai_tx: tokio::sync::mpsc::Sender<AiTask>,
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
        .route("/ws/{room_id}/{actor_id}", get(handlers::ws::ws_handler))
        .layer(cors)
        .with_state(state)
}
