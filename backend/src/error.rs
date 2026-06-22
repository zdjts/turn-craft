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
}

impl axum::response::IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        tracing::error!(error = ?self, "AppError converted into response");
        let status = match &self {
            AppError::Auth(e) => match e {
                AuthError::InvalidCredentials => axum::http::StatusCode::UNAUTHORIZED,
                AuthError::Unauthorized => axum::http::StatusCode::UNAUTHORIZED,
                AuthError::EmptyUsername => axum::http::StatusCode::BAD_REQUEST,
                AuthError::WeakPassword => axum::http::StatusCode::BAD_REQUEST,
                AuthError::User(_) => axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                AuthError::Database(_) => axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                AuthError::CryptoError(_) => axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            },
            AppError::User(e) => match e {
                UserError::UsernameTaken => axum::http::StatusCode::CONFLICT,
                UserError::NotFound => axum::http::StatusCode::NOT_FOUND,
                UserError::Database(_) => axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            },
            AppError::Room(e) => match e {
                RoomError::NotFound => axum::http::StatusCode::NOT_FOUND,
                RoomError::SlotOccupied => axum::http::StatusCode::CONFLICT,
                RoomError::NotOwner => axum::http::StatusCode::FORBIDDEN,
                RoomError::UnsupportedGameType(_) => axum::http::StatusCode::BAD_REQUEST,
                RoomError::EngineError(_) => axum::http::StatusCode::BAD_REQUEST,
                RoomError::Database(_) => axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                RoomError::Json(_) => axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            },
            AppError::Ai(e) => match e {
                AiError::ConfigNotFound => axum::http::StatusCode::NOT_FOUND,
                AiError::ProviderError(_) => axum::http::StatusCode::BAD_GATEWAY,
                AiError::Database(_) => axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                AiError::Json(_) => axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            },
            AppError::Internal(_) => axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            AppError::RoomNotFound => axum::http::StatusCode::NOT_FOUND,
            AppError::Forbidden => axum::http::StatusCode::FORBIDDEN,
        };
        (
            status,
            axum::Json(serde_json::json!({ "status": "error", "message": self.to_string() })),
        )
            .into_response()
    }
}
