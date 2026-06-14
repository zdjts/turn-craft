use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

use super::error::AuthError;
use crate::user::model::UserId;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub exp: usize,
}

pub fn issue_jwt(
    user_id: &UserId,
    secret: &str,
    expires: chrono::Duration,
) -> Result<String, AuthError> {
    let exp = (chrono::Utc::now() + expires).timestamp() as usize;
    let claims = Claims {
        sub: user_id.as_ref().to_string(),
        exp,
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_ref()),
    )
    .map_err(|e| AuthError::CryptoError(e.to_string()))
}

pub fn verify_jwt(token: &str, secret: &str) -> Result<UserId, AuthError> {
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_ref()),
        &Validation::default(),
    )
    .map_err(|_| AuthError::Unauthorized)?;

    Ok(UserId(token_data.claims.sub))
}
