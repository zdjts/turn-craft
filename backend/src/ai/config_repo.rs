use std::collections::HashMap;

use async_trait::async_trait;
use sqlx::Row;

use super::{env::AiConfig, error::AiError};

#[async_trait]
pub trait AiConfigRepository: Send + Sync {
    async fn get(&self, room_id: &str, actor_id: &str) -> Result<AiConfig, AiError>;
    async fn set(&self, room_id: &str, actor_id: &str, config: &AiConfig) -> Result<(), AiError>;
    async fn get_all_for_room(&self, room_id: &str) -> Result<HashMap<String, AiConfig>, AiError>;
    async fn delete_room(&self, room_id: &str) -> Result<(), AiError>;
}

pub struct SqliteAiConfigRepo {
    pub pool: sqlx::SqlitePool,
}

impl SqliteAiConfigRepo {
    pub fn new(pool: sqlx::SqlitePool) -> Self {
        Self { pool }
    }
}

fn row_to_ai_config(row: &sqlx::sqlite::SqliteRow) -> AiConfig {
    AiConfig {
        api_key: row.get("api_key"),
        base_url: row.get("base_url"),
        model: row.get("model"),
        max_tokens: row.get::<i64, _>("max_tokens") as u32,
        prompt: row.get("prompt"),
    }
}

#[async_trait]
impl AiConfigRepository for SqliteAiConfigRepo {
    async fn get(&self, room_id: &str, actor_id: &str) -> Result<AiConfig, AiError> {
        let row = sqlx::query(
            "SELECT api_key, base_url, model, max_tokens, prompt FROM ai_configs WHERE room_id = ? AND actor_id = ?",
        )
        .bind(room_id)
        .bind(actor_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(AiError::ConfigNotFound)?;
        Ok(row_to_ai_config(&row))
    }

    async fn set(&self, room_id: &str, actor_id: &str, config: &AiConfig) -> Result<(), AiError> {
        sqlx::query(
            r#"
            INSERT OR REPLACE INTO ai_configs
                (room_id, actor_id, api_key, base_url, model, max_tokens, prompt)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(room_id)
        .bind(actor_id)
        .bind(&config.api_key)
        .bind(&config.base_url)
        .bind(&config.model)
        .bind(config.max_tokens as i64)
        .bind(&config.prompt)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn get_all_for_room(&self, room_id: &str) -> Result<HashMap<String, AiConfig>, AiError> {
        let rows = sqlx::query(
            "SELECT actor_id, api_key, base_url, model, max_tokens, prompt FROM ai_configs WHERE room_id = ?",
        )
        .bind(room_id)
        .fetch_all(&self.pool)
        .await?;
        let map: HashMap<String, AiConfig> = rows
            .into_iter()
            .map(|row| {
                let actor_id: String = row.get("actor_id");
                let config = row_to_ai_config(&row);
                (actor_id, config)
            })
            .collect();
        Ok(map)
    }

    async fn delete_room(&self, room_id: &str) -> Result<(), AiError> {
        sqlx::query("DELETE FROM ai_configs WHERE room_id = ?")
            .bind(room_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
