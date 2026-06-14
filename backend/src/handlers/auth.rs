use axum::{Json, extract::State};
use serde::Deserialize;
use serde_json::{Value, json};
use tracing::info;

use crate::app::AppState;
use crate::error::AppError;

#[derive(Deserialize)]
pub struct RegisterInput {
    pub username: String,
    pub password: String,
}

#[derive(Deserialize)]
pub struct LoginInput {
    pub username: String,
    pub password: String,
}

pub async fn register(
    State(state): State<AppState>,
    Json(input): Json<RegisterInput>,
) -> Result<Json<Value>, AppError> {
    info!("收到用户注册请求: username={}", input.username);
    let token = state
        .auth_service
        .register(&input.username, &input.password)
        .await?;
    info!("用户注册成功: username={}", input.username);

    Ok(Json(json!({ "status": "success", "token": token })))
}

pub async fn login(
    State(state): State<AppState>,
    Json(input): Json<LoginInput>,
) -> Result<Json<Value>, AppError> {
    info!("收到用户登录请求: username={}", input.username);

    let token = state
        .auth_service
        .login(&input.username, &input.password)
        .await?;
    info!("用户登录成功: username={}", input.username);

    Ok(Json(json!({ "status": "success", "token": token })))
}
