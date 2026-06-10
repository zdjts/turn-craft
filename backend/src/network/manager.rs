use std::{fmt::Debug, sync::Arc};

use dashmap::DashMap;
use tokio::sync::{self, mpsc};

use super::room::RoomCommand;

/// 网络对端：WebSocket 连接的玩家
pub struct Peer {
    pub actor_id: String,
    pub tx: sync::mpsc::Sender<String>,
}
impl Debug for Peer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Peer")
            .field("actor_id", &self.actor_id)
            .field("tx", &"<Sender>")
            .finish()
    }
}

/// 房间句柄：持有房间 ID 和命令发送通道
pub struct RoomHandle {
    pub room_id: String,
    pub tx: mpsc::Sender<RoomCommand>,
}

/// 房间管理器：并发安全的房间注册表
pub struct RoomManager {
    pub rooms: Arc<DashMap<String, RoomHandle>>,
}

impl RoomManager {
    pub fn new() -> Self {
        Self {
            rooms: Arc::new(DashMap::new()),
        }
    }
    /// 注册新房间，若已存在则返回错误
    pub async fn register(
        &self,
        room_id: String,
        tx: mpsc::Sender<RoomCommand>,
    ) -> Result<(), String> {
        match self.rooms.entry(room_id.clone()) {
            dashmap::Entry::Occupied(_) => Err(format!("房间{room_id}已经存在")),
            dashmap::Entry::Vacant(vacant_entry) => {
                vacant_entry.insert(RoomHandle { room_id, tx });
                Ok(())
            }
        }
    }
}
