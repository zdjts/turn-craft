use crate::{
    ai::error::AiError, auth::error::AuthError, room::error::RoomError, user::error::UserError,
};

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error(transparent)]
    Auth(#[from] AuthError),
    #[error(transparent)]
    User(#[from] UserError),
    #[error(transparent)]
    Room(#[from] RoomError),
    #[error(transparent)]
    Ai(#[from] AiError),
    /// 仅用于 IO 错误、序列化失败等真正不可预料的错误
    #[error("内部错误: {0}")]
    Internal(#[from] anyhow::Error),
    #[error("房间不存在")]
    RoomNotFound,
    #[error("禁止访问")]
    Forbidden,
    #[error("请求参数无效: {0}")]
    BadRequest(String),
    #[error("操作过于频繁，请稍后再试")]
    #[allow(dead_code)]
    RateLimited,
}

impl axum::response::IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        tracing::error!(error = ?self, "AppError converted into response");

        let (status, code) = match &self {
            AppError::Auth(e) => match e {
                AuthError::InvalidCredentials => (axum::http::StatusCode::UNAUTHORIZED, "INVALID_CREDENTIALS"),
                AuthError::Unauthorized => (axum::http::StatusCode::UNAUTHORIZED, "UNAUTHORIZED"),
                AuthError::TokenExpired => (axum::http::StatusCode::UNAUTHORIZED, "TOKEN_EXPIRED"),
                AuthError::EmptyUsername => (axum::http::StatusCode::BAD_REQUEST, "EMPTY_USERNAME"),
                AuthError::WeakPassword => (axum::http::StatusCode::BAD_REQUEST, "WEAK_PASSWORD"),
                AuthError::User(_) => (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR"),
                AuthError::Database(_) => (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR"),
                AuthError::CryptoError(_) => (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR"),
            },
            AppError::User(e) => match e {
                UserError::UsernameTaken => (axum::http::StatusCode::CONFLICT, "USERNAME_TAKEN"),
                UserError::NotFound => (axum::http::StatusCode::NOT_FOUND, "USER_NOT_FOUND"),
                UserError::Database(_) => (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR"),
            },
            AppError::Room(e) => match e {
                RoomError::NotFound => (axum::http::StatusCode::NOT_FOUND, "ROOM_NOT_FOUND"),
                RoomError::SlotOccupied => (axum::http::StatusCode::CONFLICT, "SLOT_OCCUPIED"),
                RoomError::NotOwner => (axum::http::StatusCode::FORBIDDEN, "NOT_OWNER"),
                RoomError::UnsupportedGameType(_) => (axum::http::StatusCode::BAD_REQUEST, "UNSUPPORTED_GAME"),
                RoomError::EngineError(_) => (axum::http::StatusCode::BAD_REQUEST, "ENGINE_ERROR"),
                RoomError::Database(_) => (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR"),
                RoomError::Json(_) => (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR"),
            },
            AppError::Ai(e) => match e {
                AiError::ConfigNotFound => (axum::http::StatusCode::NOT_FOUND, "AI_CONFIG_NOT_FOUND"),
                AiError::ProviderError(_) => (axum::http::StatusCode::BAD_GATEWAY, "AI_PROVIDER_ERROR"),
                AiError::Database(_) => (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR"),
                AiError::Json(_) => (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR"),
            },
            AppError::Internal(_) => (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR"),
            AppError::RoomNotFound => (axum::http::StatusCode::NOT_FOUND, "ROOM_NOT_FOUND"),
            AppError::Forbidden => (axum::http::StatusCode::FORBIDDEN, "FORBIDDEN"),
            AppError::BadRequest(_) => (axum::http::StatusCode::BAD_REQUEST, "BAD_REQUEST"),
            AppError::RateLimited => (axum::http::StatusCode::TOO_MANY_REQUESTS, "RATE_LIMITED"),
        };
        (
            status,
            axum::Json(serde_json::json!({ "status": "error", "error": { "code": code, "message": self.to_string() } })),
        )
            .into_response()
    }
}
