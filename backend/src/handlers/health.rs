use axum::{Json, extract::State};
use serde_json::{json, Value};

use crate::app::AppState;

pub async fn health(
    State(state): State<AppState>,
) -> Json<Value> {
    let uptime = chrono::Utc::now()
        .signed_duration_since(state.start_time)
        .num_seconds();

    Json(json!({
        "status": "ok",
        "uptime_seconds": uptime.max(0),
    }))
}

pub async fn index() -> impl axum::response::IntoResponse {
    axum::response::Redirect::temporary("/index.html")
}
