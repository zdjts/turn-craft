use axum::{
    extract::{FromRef, FromRequestParts},
    http::request::Parts,
};

use crate::app::AppState;
use crate::auth::error::AuthError;
use crate::error::AppError;
use crate::user::model::UserId;

pub struct AuthUser(pub UserId);

impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
    AppState: FromRef<S>,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let state = AppState::from_ref(state);
        let header = parts
            .headers
            .get("Authorization")
            .and_then(|v| v.to_str().ok())
            .ok_or(AppError::Auth(AuthError::Unauthorized))?;

        let token = header
            .strip_prefix("Bearer ")
            .ok_or(AppError::Auth(AuthError::Unauthorized))?;

        let user_id = state.auth_service.verify_token(token).await?;

        Ok(AuthUser(user_id))
    }
}
