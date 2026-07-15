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
    async fn save_engine_state(&self, room_id: &str, engine_state: &str) -> Result<(), RoomError>;
    async fn list_public_filtered(
        &self,
        game_type: Option<&str>,
        page: usize,
        per_page: usize,
    ) -> Result<(Vec<RoomSnapshot>, usize), RoomError>;
    async fn set_public(&self, room_id: &str, is_public: bool) -> Result<(), RoomError>;
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
        sqlx::query(
            r#"
            INSERT INTO rooms
                (room_id, owner_id, game_type, engine_state, actor_slots, ai_configs, max_round, created_at, is_public)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(room_id) DO UPDATE SET
                engine_state = excluded.engine_state,
                actor_slots  = excluded.actor_slots,
                ai_configs   = excluded.ai_configs,
                max_round    = excluded.max_round,
                is_public    = excluded.is_public
            "#,
        )
        .bind(&snapshot.room_id)
        .bind(snapshot.owner_id.as_ref())
        .bind(&snapshot.game_type)
        .bind(serde_json::to_string(&snapshot.engine_state)?)
        .bind(serde_json::to_string(&snapshot.actor_slots)?)
        .bind(serde_json::to_string(&snapshot.ai_configs)?)
        .bind(snapshot.max_round as i64)
        .bind(created_at)
        .bind(if snapshot.is_public { 1i64 } else { 0i64 })
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn load(&self, room_id: &str) -> Result<Option<RoomSnapshot>, RoomError> {
        use sqlx::Row;
        let row = sqlx::query("SELECT room_id, owner_id, game_type, engine_state, actor_slots, ai_configs, max_round, created_at, is_public FROM rooms WHERE room_id = ?")
            .bind(room_id)
            .fetch_optional(&self.pool)
            .await?;
        match row {
            Some(r) => {
                let room_id: String = r.get("room_id");
                let owner_id: String = r.get("owner_id");
                let game_type: String = r.get("game_type");
                let engine_state: String = r.get("engine_state");
                let actor_slots: String = r.get("actor_slots");
                let ai_configs: String = r.get("ai_configs");
                let max_round: i64 = r.get("max_round");
                let created_at: String = r.get("created_at");
                let is_public: i64 = r.get("is_public");

                Ok(Some(RoomSnapshot {
                    room_id,
                    owner_id: UserId(owner_id),
                    game_type,
                    engine_state: serde_json::from_str(&engine_state)?,
                    actor_slots: serde_json::from_str(&actor_slots)?,
                    ai_configs: serde_json::from_str(&ai_configs).unwrap_or_default(),
                    max_round: max_round as usize,
                    created_at: chrono::NaiveDateTime::parse_from_str(
                        &created_at,
                        "%Y-%m-%d %H:%M:%S",
                    )
                    .unwrap_or_else(|_| chrono::Utc::now().naive_utc()),
                    is_public: is_public != 0,
                }))
            }
            None => Ok(None),
        }
    }

    async fn delete(&self, room_id: &str) -> Result<(), RoomError> {
        sqlx::query("DELETE FROM rooms WHERE room_id = ?")
            .bind(room_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn list_by_user(&self, user_id: &UserId) -> Result<Vec<RoomSnapshot>, RoomError> {
        use sqlx::Row;
        let rows = sqlx::query("SELECT room_id, owner_id, game_type, engine_state, actor_slots, ai_configs, max_round, created_at, is_public FROM rooms WHERE owner_id = ? ORDER BY created_at DESC")
            .bind(user_id.as_ref())
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
                let is_public: i64 = r.get("is_public");

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
                    is_public: is_public != 0,
                }
            })
            .collect();
        Ok(snapshots)
    }

    async fn save_engine_state(&self, room_id: &str, engine_state: &str) -> Result<(), RoomError> {
        sqlx::query("UPDATE rooms SET engine_state = ? WHERE room_id = ?")
            .bind(engine_state)
            .bind(room_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn list_all(&self) -> Result<Vec<RoomSnapshot>, RoomError> {
        use sqlx::Row;
        let rows = sqlx::query("SELECT room_id, owner_id, game_type, engine_state, actor_slots, ai_configs, max_round, created_at, is_public FROM rooms ORDER BY created_at DESC")
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
                let is_public: i64 = r.get("is_public");

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
                    is_public: is_public != 0,
                }
            })
            .collect();
        Ok(snapshots)
    }

    async fn list_public_filtered(
        &self,
        game_type: Option<&str>,
        page: usize,
        per_page: usize,
    ) -> Result<(Vec<RoomSnapshot>, usize), RoomError> {
        use sqlx::Row;

        let offset = (page.saturating_sub(1)) * per_page;

        // Count total
        let count_sql = if game_type.is_some() {
            "SELECT COUNT(*) as cnt FROM rooms WHERE is_public = 1 AND game_type = ?"
        } else {
            "SELECT COUNT(*) as cnt FROM rooms WHERE is_public = 1"
        };
        let mut count_query = sqlx::query(count_sql);
        if let Some(gt) = game_type {
            count_query = count_query.bind(gt);
        }
        let count_row = count_query.fetch_one(&self.pool).await?;
        let total: i64 = count_row.get("cnt");

        let sql = if game_type.is_some() {
            "SELECT room_id, owner_id, game_type, engine_state, actor_slots, ai_configs, max_round, created_at, is_public FROM rooms WHERE is_public = 1 AND game_type = ? ORDER BY created_at DESC LIMIT ? OFFSET ?"
        } else {
            "SELECT room_id, owner_id, game_type, engine_state, actor_slots, ai_configs, max_round, created_at, is_public FROM rooms WHERE is_public = 1 ORDER BY created_at DESC LIMIT ? OFFSET ?"
        };
        let mut query = sqlx::query(sql);
        if let Some(gt) = game_type {
            query = query.bind(gt);
        }
        query = query.bind(per_page as i64).bind(offset as i64);

        let rows = query.fetch_all(&self.pool).await?;
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
                let is_public: i64 = r.get("is_public");
                RoomSnapshot {
                    room_id,
                    owner_id: UserId(owner_id),
                    game_type,
                    engine_state: serde_json::from_str(&engine_state).unwrap_or_default(),
                    actor_slots: serde_json::from_str(&actor_slots).unwrap_or_default(),
                    ai_configs: serde_json::from_str(&ai_configs).unwrap_or_default(),
                    max_round: max_round as usize,
                    created_at: chrono::NaiveDateTime::parse_from_str(&created_at, "%Y-%m-%d %H:%M:%S")
                        .unwrap_or_else(|_| chrono::Utc::now().naive_utc()),
                    is_public: is_public != 0,
                }
            })
            .collect();
        Ok((snapshots, total as usize))
    }

    async fn set_public(&self, room_id: &str, is_public: bool) -> Result<(), RoomError> {
        sqlx::query("UPDATE rooms SET is_public = ? WHERE room_id = ?")
            .bind(if is_public { 1i64 } else { 0i64 })
            .bind(room_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
