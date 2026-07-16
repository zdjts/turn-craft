use std::sync::Arc;

use axum::{
    Router,
    routing::{delete, get, post, put},
};
use tower_http::cors::CorsLayer;

use crate::handlers;

/// 应用全局状态：包含重构后的核心服务
#[derive(Clone)]
pub struct AppState {
    pub auth_service: Arc<crate::auth::AuthService>,
    pub room_service: Arc<crate::room::RoomService>,
    pub ai_service: Arc<crate::ai::AIService>,
    pub start_time: chrono::DateTime<chrono::Utc>,
}

pub fn build_router(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(tower_http::cors::Any)
        .allow_methods(tower_http::cors::Any)
        .allow_headers(tower_http::cors::Any);

    Router::new()
        .route("/", get(handlers::health::index))
        .route("/health", get(handlers::health::health))
        .route("/games", get(handlers::game::list_games))
        .route("/register", post(handlers::auth::register))
        .route("/login", post(handlers::auth::login))
        .route("/users/me/password", put(handlers::auth::change_password))
        .route("/rooms", post(handlers::room::create_room))
        .route("/rooms/public", get(handlers::room::list_public_rooms))
        .route("/rooms/history", get(handlers::room::list_history_rooms))
        .route("/rooms/{room_id}", delete(handlers::room::delete_room))
        .route("/rooms/{room_id}", get(handlers::room::get_room))
        .route("/rooms/{room_id}/join", post(handlers::room::join_room))
        .route(
            "/rooms/{room_id}/public",
            put(handlers::room::set_room_public),
        )
        .route(
            "/rooms/{room_id}/ai-config",
            get(handlers::ai_config::get_ai_config),
        )
        .route(
            "/rooms/{room_id}/ai-config/{actor_id}",
            put(handlers::ai_config::update_ai_config),
        )
        .route(
            "/rooms/{room_id}/ai-insights",
            get(handlers::ai_insights::get_ai_insights),
        )
        .route("/rooms/{room_id}/invite", get(handlers::room::create_invite))
        .route("/invite/{code}", get(handlers::room::resolve_invite))
        .route("/leaderboard/games", get(handlers::leaderboard::leaderboard_games))
        .route("/leaderboard/wins", get(handlers::leaderboard::leaderboard_wins))
        .route("/leaderboard/experienced", get(handlers::leaderboard::leaderboard_experienced))
        .route("/leaderboard/by-game/{game_type}", get(handlers::leaderboard::leaderboard_by_game))
        .route("/users/me/achievements", get(handlers::achievements::get_achievements))
        .route("/ws/{room_id}/{actor_id}", get(handlers::ws::ws_handler))
        .layer(cors)
        .with_state(state)
}
