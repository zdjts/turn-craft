use std::sync::Arc;

use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng},
};

use super::user::model::{User, UserId};
use super::user::repository::UserRepository;
use crate::auth::error::AuthError;
use crate::auth::jwt::{issue_jwt, verify_jwt};

pub mod error;
pub mod jwt;
pub mod middleware;

pub struct AuthService {
    user_repo: Arc<dyn UserRepository>,
    jwt_secret: String,
    jwt_expires: chrono::Duration,
}

impl AuthService {
    pub fn new(
        user_repo: Arc<dyn UserRepository>,
        jwt_secret: &str,
        jwt_expires_secs: u64,
    ) -> Self {
        Self {
            user_repo,
            jwt_secret: jwt_secret.to_string(),
            jwt_expires: chrono::Duration::seconds(jwt_expires_secs as i64),
        }
    }

    pub async fn register(&self, username: &str, password: &str) -> Result<String, AuthError> {
        if username.is_empty() {
            return Err(AuthError::EmptyUsername);
        }
        if password.len() < 6 {
            return Err(AuthError::WeakPassword);
        }

        let hash = hash_password(password)?;
        let user = User {
            id: UserId::new(),
            username: username.to_string(),
            password_hash: hash,
            created_at: chrono::Utc::now().naive_utc(),
        };

        self.user_repo.create(&user).await?;

        let token = issue_jwt(&user.id, &self.jwt_secret, self.jwt_expires)?;
        Ok(token)
    }

    pub async fn login(&self, username: &str, password: &str) -> Result<String, AuthError> {
        let user = self
            .user_repo
            .find_by_username(username)
            .await?
            .ok_or(AuthError::InvalidCredentials)?;

        verify_password(password, &user.password_hash)?;

        let token = issue_jwt(&user.id, &self.jwt_secret, self.jwt_expires)?;
        Ok(token)
    }

    pub async fn verify_token(&self, token: &str) -> Result<UserId, AuthError> {
        verify_jwt(token, &self.jwt_secret)
    }
}

fn hash_password(password: &str) -> Result<String, AuthError> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| AuthError::CryptoError(e.to_string()))
}

fn verify_password(password: &str, hash: &str) -> Result<(), AuthError> {
    let parsed_hash = PasswordHash::new(hash).map_err(|e| AuthError::CryptoError(e.to_string()))?;
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .map_err(|_| AuthError::InvalidCredentials)
}
