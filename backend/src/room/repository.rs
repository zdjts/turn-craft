use async_trait::async_trait;

use crate::user::model::UserId;

use super::{error::RoomError, model::RoomSnapshot};

#[async_trait]
pub trait RoomRepository: Send + Sync {
    async fn save(&self, snapshot: &RoomSnapshot) -> Result<(), RoomError>;
    async fn load(&self, room_id: &str) -> Result<Option<RoomSnapshot>, RoomError>;
    async fn delete(&self, room_id: &str) -> Result<(), RoomError>;
    async fn list_by_user(&self, user_id: &UserId) -> Result<Vec<RoomSnapshot>, RoomError>;
    async fn list_all(&self) -> Result<Vec<RoomSnapshot>, RoomError>;
}

pub struct SqliteRoomRepo {
    pool: sqlx::SqlitePool,
}

impl SqliteRoomRepo {
    pub fn new(pool: sqlx::SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl RoomRepository for SqliteRoomRepo {
    async fn save(&self, snapshot: &RoomSnapshot) -> Result<(), RoomError> {
        let created_at = snapshot.created_at.format("%Y-%m-%d %H:%M:%S").to_string();
        sqlx::query!(
            r#"
            INSERT OR REPLACE INTO rooms
                (room_id, owner_id, game_type, engine_state, actor_slots, ai_configs, max_round, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            snapshot.room_id,
            snapshot.owner_id.as_ref(),
            snapshot.game_type,
            serde_json::to_string(&snapshot.engine_state)?,
            serde_json::to_string(&snapshot.actor_slots)?,
            serde_json::to_string(&snapshot.ai_configs)?,
            snapshot.max_round as i64,
            created_at,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn load(&self, room_id: &str) -> Result<Option<RoomSnapshot>, RoomError> {
        let row = sqlx::query!("SELECT * FROM rooms WHERE room_id = ?", room_id)
            .fetch_optional(&self.pool)
            .await?;
        match row {
            Some(r) => Ok(Some(RoomSnapshot {
                room_id: r.room_id,
                owner_id: UserId(r.owner_id),
                game_type: r.game_type,
                engine_state: serde_json::from_str(&r.engine_state)?,
                actor_slots: serde_json::from_str(&r.actor_slots)?,
                ai_configs: serde_json::from_str(&r.ai_configs).unwrap_or_default(),
                max_round: r.max_round as usize,
                created_at: chrono::NaiveDateTime::parse_from_str(
                    &r.created_at,
                    "%Y-%m-%d %H:%M:%S",
                )
                .unwrap_or_else(|_| chrono::Utc::now().naive_utc()),
            })),
            None => Ok(None),
        }
    }

    async fn delete(&self, room_id: &str) -> Result<(), RoomError> {
        sqlx::query!("DELETE FROM rooms WHERE room_id = ?", room_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn list_by_user(&self, user_id: &UserId) -> Result<Vec<RoomSnapshot>, RoomError> {
        let rows = sqlx::query!(
            "SELECT * FROM rooms WHERE owner_id = ? ORDER BY created_at DESC",
            user_id.as_ref(),
        )
        .fetch_all(&self.pool)
        .await?;
        let snapshots = rows
            .into_iter()
            .map(|r| RoomSnapshot {
                room_id: r.room_id,
                owner_id: UserId(r.owner_id),
                game_type: r.game_type,
                engine_state: serde_json::from_str(&r.engine_state).unwrap_or_default(),
                actor_slots: serde_json::from_str(&r.actor_slots).unwrap_or_default(),
                ai_configs: serde_json::from_str(&r.ai_configs).unwrap_or_default(),
                max_round: r.max_round as usize,
                created_at: chrono::NaiveDateTime::parse_from_str(
                    &r.created_at,
                    "%Y-%m-%d %H:%M:%S",
                )
                .unwrap_or_else(|_| chrono::Utc::now().naive_utc()),
            })
            .collect();
        Ok(snapshots)
    }

    async fn list_all(&self) -> Result<Vec<RoomSnapshot>, RoomError> {
        use sqlx::Row;
        let rows = sqlx::query("SELECT room_id, owner_id, game_type, engine_state, actor_slots, ai_configs, max_round, created_at FROM rooms ORDER BY created_at DESC")
            .fetch_all(&self.pool)
            .await?;
        let snapshots = rows
            .into_iter()
            .map(|r| {
                let room_id: String = r.get("room_id");
                let owner_id: String = r.get("owner_id");
                let game_type: String = r.get("game_type");
                let engine_state: String = r.get("engine_state");
                let actor_slots: String = r.get("actor_slots");
                let ai_configs: String = r.get("ai_configs");
                let max_round: i64 = r.get("max_round");
                let created_at: String = r.get("created_at");

                RoomSnapshot {
                    room_id,
                    owner_id: UserId(owner_id),
                    game_type,
                    engine_state: serde_json::from_str(&engine_state).unwrap_or_default(),
                    actor_slots: serde_json::from_str(&actor_slots).unwrap_or_default(),
                    ai_configs: serde_json::from_str(&ai_configs).unwrap_or_default(),
                    max_round: max_round as usize,
                    created_at: chrono::NaiveDateTime::parse_from_str(
                        &created_at,
                        "%Y-%m-%d %H:%M:%S",
                    )
                    .unwrap_or_else(|_| chrono::Utc::now().naive_utc()),
                }
            })
            .collect();
        Ok(snapshots)
    }
}
