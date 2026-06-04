use std::{collections::HashMap, fmt::Debug, sync::Arc};

use tokio::sync::{self, RwLock, mpsc};

use super::room::RoomCommand;

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
pub struct RoomHandle {
    pub room_id: String,
    pub tx: mpsc::Sender<RoomCommand>,
}
pub struct RoomManager {
    rooms: Arc<RwLock<HashMap<String, RoomHandle>>>,
}

impl RoomManager {
    pub fn new() -> Self {
        Self {
            rooms: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    pub async fn register(
        &self,
        room_id: String,
        tx: mpsc::Sender<RoomCommand>,
    ) -> Result<(), String> {
        let mut rooms = self.rooms.write().await;
        if rooms.contains_key(&room_id) {
            return Err(format!("房间 {room_id} 已经存在"));
        }
        rooms.insert(room_id.clone(), RoomHandle { room_id, tx });
        Ok(())
    }
}
