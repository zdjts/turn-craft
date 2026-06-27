pub mod actor;
pub mod error;
pub mod model;
pub mod repository;
// pub mod service;
pub mod supervisor;

use std::sync::Arc;

use dashmap::DashMap;
use tokio::sync::mpsc;

use crate::ai::config_repo::AiConfigRepository;
use crate::error::AppError;
use crate::games::GameRegistry;
use crate::room::error::RoomError;
use crate::user::model::UserId;

use self::actor::{SideEffect, spawn_game_room};
use self::model::{AiTask, CreateRoomInput, CreateRoomOutput, Peer, RoomCommand, RoomSnapshot};
use self::repository::RoomRepository;
use self::supervisor::RoomSupervisor;

pub struct RoomService {
    room_repo: Arc<dyn RoomRepository>,
    ai_config_repo: Arc<dyn AiConfigRepository>,
    ai_worker_tx: mpsc::Sender<AiTask>,
    active_rooms: Arc<DashMap<String, mpsc::Sender<RoomCommand>>>,
    supervisor: RoomSupervisor,
    game_registry: Arc<GameRegistry>,
}

impl RoomService {
    pub fn new(
        room_repo: Arc<dyn RoomRepository>,
        ai_config_repo: Arc<dyn AiConfigRepository>,
        ai_worker_tx: mpsc::Sender<AiTask>,
        supervisor: RoomSupervisor,
        game_registry: Arc<GameRegistry>,
    ) -> Self {
        Self {
            room_repo,
            ai_config_repo,
            ai_worker_tx,
            active_rooms: Arc::new(DashMap::new()),
            supervisor,
            game_registry,
        }
    }

    pub async fn create_room(
        &self,
        owner_id: UserId,
        input: CreateRoomInput,
    ) -> Result<CreateRoomOutput, AppError> {
        let factory = self.game_registry.get(&input.game_type).ok_or_else(|| {
            AppError::Room(RoomError::UnsupportedGameType(input.game_type.clone()))
        })?;

        let room_id = format!("room_{}", uuid::Uuid::new_v4());

        // 1. 构建 ActorSlot 列表
        let slots = build_slots(&input, &owner_id);

        // 2. 创建引擎和 AI 配置
        let (engine, ai_configs) = factory
            .create(&room_id, &owner_id, &input, &*self.ai_config_repo)
            .await?;

        // 3. 保存房间
        let snapshot = RoomSnapshot {
            room_id: room_id.clone(),
            owner_id,
            game_type: input.game_type,
            engine_state: engine.to_json(),
            actor_slots: slots,
            ai_configs: ai_configs.clone(),
            max_round: input.max_round,
            created_at: chrono::Utc::now().naive_utc(),
            is_public: input.is_public,
        };
        self.room_repo
            .save(&snapshot)
            .await
            .map_err(AppError::Room)?;

        // 4. 保存 AI 配置
        for (actor_id, config) in &ai_configs {
            self.ai_config_repo
                .set(&room_id, actor_id, config)
                .await
                .map_err(AppError::Ai)?;
        }

        // 5. 启动 RoomActor + side effect handler
        let (effect_tx, effect_rx) = mpsc::channel::<SideEffect>(64);
        let effect_tx_for_handler = effect_tx.clone();
        let room_tx = spawn_game_room(room_id.clone(), engine, effect_tx);
        self.spawn_effect_handler(room_id.clone(), effect_tx_for_handler, effect_rx);
        self.active_rooms.insert(room_id.clone(), room_tx);

        Ok(CreateRoomOutput {
            room_id,
            assigned_slot: input.my_slot,
        })
    }

    /// 启动单个房间的异步副作用处理任务 (保存历史、触发 AI)
    fn spawn_effect_handler(&self, room_id: String, effect_tx: mpsc::Sender<SideEffect>, mut rx: mpsc::Receiver<SideEffect>) {
        let repo = self.room_repo.clone();
        let ai_repo = self.ai_config_repo.clone();
        let ai_tx = self.ai_worker_tx.clone();
        let active_rooms = self.active_rooms.clone();
        let supervisor = self.supervisor.clone();

        tokio::spawn(async move {
            while let Some(effect) = rx.recv().await {
                match effect {
                    SideEffect::TriggerAi {
                        actor_id,
                        snapshot,
                        tools,
                    } => {
                        tracing::info!(room_id = %room_id, actor_id = %actor_id, "收到 TriggerAi 事件，准备获取 AI 配置");
                        match ai_repo.get(&room_id, &actor_id).await {
                            Ok(config) => {
                                if let Some(room_tx) = active_rooms.get(&room_id) {
                                    let reply_tx = room_tx.clone();
                                    let _ = ai_tx
                                        .send(AiTask {
                                            room_id: room_id.clone(),
                                            actor_id,
                                            snapshot,
                                            ai_config: config,
                                            tools,
                                            retries: 0,
                                            reply_tx,
                                            effect_tx: effect_tx.clone(),
                                        })
                                        .await;
                                } else {
                                    tracing::warn!(room_id = %room_id, "找不到活跃房间，放弃发送 AI 任务");
                                }
                            }
                            Err(e) => {
                                tracing::error!(room_id = %room_id, actor_id = %actor_id, error = ?e, "获取 AI 配置失败，无法触发 AI");
                            }
                        }
                    }
                    SideEffect::PersistSnapshot(snapshot) => {
                        let _ = repo.save(&snapshot).await;
                    }
                    SideEffect::GameOver => {
                        tracing::info!(room_id = %room_id, "游戏结束");
                    }
                    SideEffect::RoomEmpty => {
                        tracing::info!(room_id = %room_id, "房间空闲，开始保活监控");
                        if let Some(room_tx) = active_rooms.get(&room_id) {
                            supervisor.track(room_id.clone(), room_tx.clone()).await;
                        }
                    }
                    SideEffect::PeerJoined => {
                        tracing::info!(room_id = %room_id, "玩家加入/重连，停止保活监控");
                        supervisor.release(&room_id).await;
                    }
                    SideEffect::StreamChunk { actor_id, content, is_done } => {
                        if let Some(room_tx) = active_rooms.get(&room_id) {
                            let _ = room_tx.send(crate::room::model::RoomCommand::BroadcastStreamChunk {
                                actor_id,
                                content,
                                is_done,
                            }).await;
                        }
                    }
                }
            }
        });
    }

    pub async fn connect(
        &self,
        user_id: UserId,
        room_id: &str,
        slot_name: &str,
        peer_tx: mpsc::Sender<String>,
    ) -> Result<(), AppError> {
        let snapshot = self
            .room_repo
            .load(room_id)
            .await?
            .ok_or(AppError::RoomNotFound)?;
        if slot_name != "spectator" && !slot_name.starts_with("spectator") {
            let slot = snapshot
                .actor_slots
                .iter()
                .find(|s| s.slot_name == slot_name)
                .ok_or(AppError::Forbidden)?;
            slot.authorize(&user_id).map_err(AppError::Room)?;
        }

        let room_tx = self
            .active_rooms
            .get(room_id)
            .ok_or(AppError::RoomNotFound)?;
        room_tx
            .send(RoomCommand::Join(Peer {
                actor_id: slot_name.to_string(),
                tx: peer_tx,
            }))
            .await
            .map_err(|_| AppError::RoomNotFound)?;

        self.supervisor.release(room_id).await;

        Ok(())
    }

    pub fn get_room_tx(&self, room_id: &str) -> Option<mpsc::Sender<RoomCommand>> {
        self.active_rooms.get(room_id).map(|r| r.value().clone())
    }

    pub async fn shutdown_room(&self, room_id: &str) -> Result<(), AppError> {
        if let Some((_, room_tx)) = self.active_rooms.remove(room_id) {
            let _ = room_tx.send(RoomCommand::Shutdown).await;
        }
        self.supervisor.release(room_id).await;
        Ok(())
    }

    pub async fn delete_room(&self, user_id: UserId, room_id: &str) -> Result<(), AppError> {
        let snapshot = self
            .room_repo
            .load(room_id)
            .await?
            .ok_or(AppError::RoomNotFound)?;
        if snapshot.owner_id != user_id {
            return Err(AppError::Forbidden);
        }
        self.shutdown_room(room_id).await?;
        self.room_repo
            .delete(room_id)
            .await
            .map_err(AppError::Room)?;
        self.ai_config_repo
            .delete_room(room_id)
            .await
            .map_err(AppError::Ai)?;
        Ok(())
    }

    pub async fn restore_all(&self) -> Result<(), AppError> {
        let snapshots = self.room_repo.list_all().await.map_err(AppError::Room)?;
        for snap in snapshots {
            if let Some(factory) = self.game_registry.get(&snap.game_type) {
                let engine = match factory.restore(&snap.engine_state) {
                    Ok(eng) => eng,
                    Err(e) => {
                        tracing::error!(room_id = %snap.room_id, error = ?e, "无法恢复引擎，跳过此房间");
                        continue;
                    }
                };
                let (effect_tx, effect_rx) = mpsc::channel::<SideEffect>(64);
                let effect_tx_for_handler = effect_tx.clone();
                let room_tx = spawn_game_room(snap.room_id.clone(), engine, effect_tx);
                self.spawn_effect_handler(snap.room_id.clone(), effect_tx_for_handler, effect_rx);
                self.active_rooms
                    .insert(snap.room_id.clone(), room_tx.clone());

                // 恢复时默认开启保活监控，直到有玩家重连
                self.supervisor.track(snap.room_id.clone(), room_tx).await;
                tracing::info!(room_id = %snap.room_id, game_type = %snap.game_type, "房间已成功恢复并加入保活监控");
            }
        }
        Ok(())
    }

    pub async fn list_public_rooms(&self) -> Result<Vec<RoomSnapshot>, AppError> {
        let rooms = self.room_repo.list_all().await.map_err(AppError::Room)?;
        Ok(rooms.into_iter().filter(|r| r.is_public).collect())
    }

    pub async fn list_history_rooms(&self, user_id: UserId) -> Result<Vec<RoomSnapshot>, AppError> {
        self.room_repo
            .list_by_user(&user_id)
            .await
            .map_err(AppError::Room)
    }

    pub async fn set_room_public(
        &self,
        user_id: UserId,
        room_id: &str,
        is_public: bool,
    ) -> Result<(), AppError> {
        let snapshot = self
            .room_repo
            .load(room_id)
            .await?
            .ok_or(AppError::RoomNotFound)?;
        if snapshot.owner_id != user_id {
            return Err(AppError::Forbidden);
        }
        self.room_repo
            .set_public(room_id, is_public)
            .await
            .map_err(AppError::Room)?;
        Ok(())
    }

    pub async fn get_room_snapshot(&self, room_id: &str) -> Result<Option<RoomSnapshot>, AppError> {
        self.room_repo.load(room_id).await.map_err(AppError::Room)
    }

    pub async fn join_slot(
        &self,
        user_id: UserId,
        room_id: &str,
        slot_name: &str,
    ) -> Result<(), AppError> {
        let mut snapshot = self
            .room_repo
            .load(room_id)
            .await?
            .ok_or(AppError::RoomNotFound)?;

        let slot = snapshot
            .actor_slots
            .iter_mut()
            .find(|s| s.slot_name == slot_name)
            .ok_or(AppError::Forbidden)?;

        match &slot.occupant {
            self::model::ActorOccupant::Empty => {
                slot.occupant = self::model::ActorOccupant::Human(user_id);
            }
            self::model::ActorOccupant::Human(id) if id == &user_id => {
                // 已经占据该位置
                return Ok(());
            }
            _ => {
                return Err(AppError::Forbidden); // Slot taken or is AI
            }
        }

        self.room_repo
            .save(&snapshot)
            .await
            .map_err(AppError::Room)?;
        Ok(())
    }
}

fn build_slots(input: &CreateRoomInput, owner_id: &UserId) -> Vec<self::model::ActorSlot> {
    input
        .slots
        .iter()
        .map(|name| {
            let occupant = match input.slot_configs.get(name).map(|s| s.as_str()) {
                Some("human") => {
                    if name == &input.my_slot || (input.my_slot.is_empty() && name == "spectator") {
                        self::model::ActorOccupant::Human(owner_id.clone())
                    } else {
                        self::model::ActorOccupant::Empty
                    }
                }
                Some("ai") => self::model::ActorOccupant::Ai,
                _ => self::model::ActorOccupant::Empty,
            };
            self::model::ActorSlot {
                slot_name: name.clone(),
                occupant,
            }
        })
        .collect()
}
