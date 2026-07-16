use async_trait::async_trait;
use serde::Serialize;
use serde_json::Value;

use crate::room::error::RoomError;

/// 游戏事件日志条目
#[derive(Debug, Clone, Serialize)]
#[allow(dead_code)]
pub struct GameEvent {
    pub room_id: String,
    pub seq: i64,
    pub event_type: String,
    pub actor_id: String,
    pub payload: Value,
}

#[async_trait]
pub trait EventStore: Send + Sync {
    /// 追加一个事件到日志
    async fn append(&self, room_id: &str, event_type: &str, actor_id: &str, payload: &Value) -> Result<i64, RoomError>;
}

pub struct SqliteEventStore {
    pool: sqlx::SqlitePool,
}

impl SqliteEventStore {
    pub fn new(pool: sqlx::SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl EventStore for SqliteEventStore {
    async fn append(&self, room_id: &str, event_type: &str, actor_id: &str, payload: &Value) -> Result<i64, RoomError> {
        let payload_str = serde_json::to_string(payload).unwrap_or_default();
        let seq: i64 = sqlx::query_scalar(
            r#"
            INSERT INTO game_events (room_id, seq, event_type, actor_id, payload)
            VALUES (?, COALESCE((SELECT MAX(seq) + 1 FROM game_events WHERE room_id = ?), 1), ?, ?, ?)
            RETURNING seq
            "#,
        )
        .bind(room_id)
        .bind(room_id)
        .bind(event_type)
        .bind(actor_id)
        .bind(&payload_str)
        .fetch_one(&self.pool)
        .await?;

        // 更新房间的 event_seq
        sqlx::query("UPDATE rooms SET event_seq = ? WHERE room_id = ?")
            .bind(seq)
            .bind(room_id)
            .execute(&self.pool)
            .await?;

        Ok(seq)
    }
}
