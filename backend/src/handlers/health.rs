
/// 健康检查端点
pub async fn health() -> &'static str {
    "ok"
}

/// 首页重定向到前端
pub async fn index() -> impl axum::response::IntoResponse {
    axum::response::Redirect::temporary("/index.html")
}
