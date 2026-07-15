use axum::{Json, extract::State};
use serde::Deserialize;
use serde_json::{Value, json};
use tracing::info;

use crate::app::AppState;
use crate::auth::middleware::AuthUser;
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

fn validate_username(username: &str) -> Result<(), AppError> {
    if username.is_empty() {
        return Err(AppError::BadRequest("用户名不能为空".into()));
    }
    if username.len() < 3 || username.len() > 32 {
        return Err(AppError::BadRequest("用户名长度须在 3-32 个字符之间".into()));
    }
    if !username.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
        return Err(AppError::BadRequest("用户名只能包含字母、数字、下划线和连字符".into()));
    }
    Ok(())
}

fn validate_password(password: &str) -> Result<(), AppError> {
    if password.len() < 6 {
        return Err(AppError::BadRequest("密码长度不能少于 6 位".into()));
    }
    Ok(())
}

pub async fn register(
    State(state): State<AppState>,
    Json(input): Json<RegisterInput>,
) -> Result<Json<Value>, AppError> {
    validate_username(&input.username)?;
    validate_password(&input.password)?;
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
    if input.username.is_empty() || input.password.is_empty() {
        return Err(AppError::BadRequest("用户名和密码不能为空".into()));
    }
    info!("收到用户登录请求: username={}", input.username);

    let token = state
        .auth_service
        .login(&input.username, &input.password)
        .await?;
    info!("用户登录成功: username={}", input.username);

    Ok(Json(json!({ "status": "success", "token": token })))
}

#[derive(Deserialize)]
pub struct ChangePasswordInput {
    pub old_password: String,
    pub new_password: String,
}

pub async fn change_password(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Json(input): Json<ChangePasswordInput>,
) -> Result<Json<Value>, AppError> {
    validate_password(&input.new_password)?;
    state
        .auth_service
        .change_password(&user_id, &input.old_password, &input.new_password)
        .await?;
    Ok(Json(json!({ "status": "success", "message": "密码已更新" })))
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_username_valid() {
        assert!(super::validate_username("test_user").is_ok());
        assert!(super::validate_username("abc").is_ok());
        assert!(super::validate_username("user-name").is_ok());
    }

    #[test]
    fn test_username_empty() {
        assert!(super::validate_username("").is_err());
    }

    #[test]
    fn test_username_too_short() {
        assert!(super::validate_username("ab").is_err());
    }

    #[test]
    fn test_username_invalid_chars() {
        assert!(super::validate_username("test user").is_err());
        assert!(super::validate_username("test@user").is_err());
    }

    #[test]
    fn test_password_valid() {
        assert!(super::validate_password("123456").is_ok());
    }

    #[test]
    fn test_password_too_short() {
        assert!(super::validate_password("12345").is_err());
    }
}
