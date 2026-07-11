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
pub enum RoomCommand {
    PlayerAction {
        actor_id: String,
        action: Value,
        feedback_tx: Option<tokio::sync::oneshot::Sender<Result<(), String>>>,
    },
    Join(Peer),
    Leave(String),
    Shutdown,
    /// 广播流式 chunk 到所有已连接的 WebSocket peer
    BroadcastStreamChunk {
        actor_id: String,
        content: String,
        is_done: bool,
    },
}

impl std::fmt::Debug for RoomCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PlayerAction {
                actor_id, action, ..
            } => f
                .debug_struct("PlayerAction")
                .field("actor_id", actor_id)
                .field("action", action)
                .field("feedback_tx", &"<Sender>")
                .finish(),
            Self::Join(arg0) => f.debug_tuple("Join").field(arg0).finish(),
            Self::Leave(arg0) => f.debug_tuple("Leave").field(arg0).finish(),
            Self::Shutdown => write!(f, "Shutdown"),
            Self::BroadcastStreamChunk {
                actor_id, is_done, ..
            } => f
                .debug_struct("BroadcastStreamChunk")
                .field("actor_id", actor_id)
                .field("is_done", is_done)
                .finish(),
        }
    }
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
    /// SideEffect 通道 — 用于从 AiWorker 发送流式 chunk 到 EffectHandler
    pub effect_tx: mpsc::Sender<super::actor::SideEffect>,
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
            ActorOccupant::Empty => Err(RoomError::NotFound), // Must call join_room API first to claim the slot
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
