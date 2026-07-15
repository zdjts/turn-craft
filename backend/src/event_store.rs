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

    /// 获取房间的事件列表（分页）
    async fn list_events(&self, room_id: &str, offset: i64, limit: i64) -> Result<Vec<GameEvent>, RoomError>;

    /// 获取房间当前事件序号
    async fn current_seq(&self, room_id: &str) -> Result<i64, RoomError>;
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

    async fn list_events(&self, room_id: &str, offset: i64, limit: i64) -> Result<Vec<GameEvent>, RoomError> {
        use sqlx::Row;
        let rows = sqlx::query(
            "SELECT room_id, seq, event_type, actor_id, payload, created_at FROM game_events WHERE room_id = ? ORDER BY seq ASC LIMIT ? OFFSET ?",
        )
        .bind(room_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        let events = rows
            .into_iter()
            .map(|r| {
                let room_id: String = r.get("room_id");
                let seq: i64 = r.get("seq");
                let event_type: String = r.get("event_type");
                let actor_id: String = r.get("actor_id");
                let payload_str: String = r.get("payload");
                let payload: Value = serde_json::from_str(&payload_str).unwrap_or_default();
                GameEvent {
                    room_id,
                    seq,
                    event_type,
                    actor_id,
                    payload,
                }
            })
            .collect();

        Ok(events)
    }

    async fn current_seq(&self, room_id: &str) -> Result<i64, RoomError> {
        let seq: Option<i64> = sqlx::query_scalar(
            "SELECT event_seq FROM rooms WHERE room_id = ?",
        )
        .bind(room_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(seq.unwrap_or(0))
    }
}
