use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::mpsc;

use crate::{ai::env::AiConfig, user::model::UserId};

use super::error::RoomError;

#[derive(Debug)]
pub struct Peer {
    pub actor_id: String,
    pub tx: mpsc::Sender<String>,
}

/// 房间命令协议 — Actor 和 AI Worker 共用
#[derive(Debug)]
pub enum RoomCommand {
    PlayerAction { actor_id: String, action: Value },
    Join(Peer),
    Leave(String),
    Shutdown,
}

/// AI 任务 — 供 AiWorker 消费
pub struct AiTask {
    pub room_id: String,
    pub actor_id: String,
    pub snapshot: String,
    pub reply_tx: mpsc::Sender<RoomCommand>,
    pub ai_config: AiConfig,
    pub tools: Option<Value>,
    pub retries: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActorOccupant {
    Human(UserId),
    Ai,
    Empty,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActorSlot {
    pub slot_name: String,
    pub occupant: ActorOccupant,
}

impl ActorSlot {
    /// 检查当前用户是否有权使用这个槽位
    pub fn authorize(&self, user_id: &UserId) -> Result<(), RoomError> {
        match &self.occupant {
            ActorOccupant::Human(owner) if owner == user_id => Ok(()),
            ActorOccupant::Human(_) => Err(RoomError::NotFound),
            ActorOccupant::Ai => Err(RoomError::NotFound),
            ActorOccupant::Empty => Ok(()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateRoomInput {
    pub game_type: String,
    pub max_round: usize,
    pub my_slot: String,
    pub slots: Vec<String>,
    pub slot_configs: std::collections::HashMap<String, String>,
    pub game_config: Option<serde_json::Value>,
    #[serde(default)]
    pub is_public: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateRoomOutput {
    pub room_id: String,
    pub assigned_slot: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomSnapshot {
    pub room_id: String,
    pub owner_id: UserId,
    pub game_type: String,
    pub engine_state: serde_json::Value,
    pub actor_slots: Vec<ActorSlot>,
    pub ai_configs: HashMap<String, AiConfig>,
    pub max_round: usize,
    pub created_at: chrono::NaiveDateTime,
    pub is_public: bool,
}
