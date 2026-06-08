
pub async fn health() -> &'static str {
    "ok"
}

pub async fn index() -> impl axum::response::IntoResponse {
    axum::response::Redirect::temporary("/index.html")
}
