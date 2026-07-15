use async_trait::async_trait;
use sqlx::SqlitePool;

use super::{
    error::UserError,
    model::{User, UserId},
};

#[async_trait]
pub trait UserRepository: Send + Sync {
    async fn create(&self, user: &User) -> Result<(), UserError>;
    async fn find_by_username(&self, username: &str) -> Result<Option<User>, UserError>;
    async fn find_by_id(&self, id: &UserId) -> Result<Option<User>, UserError>;
    async fn update_password(&self, user_id: &UserId, hash: &str) -> Result<(), UserError>;
}

pub struct SqliteUserRepo {
    pool: SqlitePool,
}

impl SqliteUserRepo {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl UserRepository for SqliteUserRepo {
    async fn create(&self, user: &User) -> Result<(), UserError> {
        let created_at = user.created_at.format("%Y-%m-%d %H:%M:%S").to_string();
        sqlx::query!(
            "INSERT INTO users (id, username, password_hash, created_at) VALUES (?, ?, ?, ?)",
            user.id.as_ref(),
            user.username,
            user.password_hash,
            created_at,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| {
            if e.to_string().contains("UNIQUE") {
                UserError::UsernameTaken
            } else {
                UserError::Database(e)
            }
        })?;
        Ok(())
    }

    async fn find_by_username(&self, username: &str) -> Result<Option<User>, UserError> {
        let row = sqlx::query!(
            "SELECT id, username, password_hash, created_at FROM users WHERE username = ?",
            username,
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| User {
            id: UserId(r.id),
            username: r.username,
            password_hash: r.password_hash,
            created_at: chrono::NaiveDateTime::parse_from_str(&r.created_at, "%Y-%m-%d %H:%M:%S")
                .unwrap_or_else(|_| chrono::Utc::now().naive_utc()),
        }))
    }

    async fn find_by_id(&self, id: &UserId) -> Result<Option<User>, UserError> {
        let row = sqlx::query!(
            "SELECT id, username, password_hash, created_at FROM users WHERE id = ?",
            id.as_ref(),
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| User {
            id: UserId(r.id),
            username: r.username,
            password_hash: r.password_hash,
            created_at: chrono::NaiveDateTime::parse_from_str(&r.created_at, "%Y-%m-%d %H:%M:%S")
                .unwrap_or_else(|_| chrono::Utc::now().naive_utc()),
        }))
    }

    async fn update_password(&self, user_id: &UserId, hash: &str) -> Result<(), UserError> {
        let rows = sqlx::query("UPDATE users SET password_hash = ? WHERE id = ?")
            .bind(hash)
            .bind(user_id.as_ref())
            .execute(&self.pool)
            .await
            .map_err(UserError::Database)?;
        if rows.rows_affected() == 0 {
            return Err(UserError::NotFound);
        }
        Ok(())
    }
}
