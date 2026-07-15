use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode, errors::ErrorKind};
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
    .map_err(|e| {
        if matches!(e.kind(), ErrorKind::ExpiredSignature) {
            AuthError::TokenExpired
        } else {
            AuthError::Unauthorized
        }
    })?;

    Ok(UserId(token_data.claims.sub))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_expiry_detection() {
        let user_id = UserId("test-user".into());
        let secret = "test-secret";

        // Normal token should verify
        let token = issue_jwt(&user_id, secret, chrono::Duration::seconds(3600)).unwrap();
        let result = verify_jwt(&token, secret);
        assert!(result.is_ok(), "valid token should verify");

        // Expired token should return ExpiredSignature error
        use jsonwebtoken::Validation;
        let expired_token = issue_jwt(&user_id, secret, chrono::Duration::seconds(-3600)).unwrap();
        let result = decode::<Claims>(
            &expired_token,
            &DecodingKey::from_secret(secret.as_ref()),
            &Validation::default(),
        );
        match result {
            Err(e) if matches!(e.kind(), ErrorKind::ExpiredSignature) => {},
            other => panic!("expected expired, got {:?}", other),
        }
    }

    #[test]
    fn test_invalid_token() {
        let result = verify_jwt("garbage", "secret");
        match result {
            Err(AuthError::Unauthorized) => {},
            other => panic!("expected Unauthorized, got {:?}", other),
        }
    }
}
