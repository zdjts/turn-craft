use axum::{Json, extract::State};
use serde_json::{Value, json};

use crate::app::AppState;

pub async fn list_games(
    State(state): State<AppState>,
) -> Result<Json<Value>, axum::http::StatusCode> {
    let metas = state.room_service.game_registry.all_meta();
    Ok(Json(json!({
        "status": "success",
        "games": metas,
    })))
}
