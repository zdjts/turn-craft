use std::{fmt::Debug, sync::Arc};

use dashmap::DashMap;
use tokio::sync::{self, mpsc};

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
    pub rooms: Arc<DashMap<String, RoomHandle>>,
}

impl RoomManager {
    pub fn new() -> Self {
        Self {
            rooms: Arc::new(DashMap::new()),
        }
    }
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
