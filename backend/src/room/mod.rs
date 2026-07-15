pub mod actor;
pub mod error;
pub mod model;
pub mod repository;
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
use crate::event_store::EventStore;

/// AI 失败后自动重试的延迟秒数（actor 状态机控制重试次数）
const AI_RETRY_DELAY_SECS: u64 = 5;

pub struct RoomService {
    room_repo: Arc<dyn RoomRepository>,
    ai_config_repo: Arc<dyn AiConfigRepository>,
    ai_worker_tx: mpsc::Sender<AiTask>,
    active_rooms: Arc<DashMap<String, mpsc::Sender<RoomCommand>>>,
    supervisor: RoomSupervisor,
    pub game_registry: Arc<GameRegistry>,
    event_store: Arc<dyn EventStore>,
}

impl RoomService {
    pub fn new(
        room_repo: Arc<dyn RoomRepository>,
        ai_config_repo: Arc<dyn AiConfigRepository>,
        ai_worker_tx: mpsc::Sender<AiTask>,
        supervisor: RoomSupervisor,
        game_registry: Arc<GameRegistry>,
        event_store: Arc<dyn EventStore>,
    ) -> Self {
        Self {
            room_repo,
            ai_config_repo,
            ai_worker_tx,
            active_rooms: Arc::new(DashMap::new()),
            supervisor,
            game_registry,
            event_store,
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

    /// 启动单个房间的异步副作用处理任务 (保存状态、触发 AI、事件日志)
    fn spawn_effect_handler(
        &self,
        room_id: String,
        effect_tx: mpsc::Sender<SideEffect>,
        mut rx: mpsc::Receiver<SideEffect>,
    ) {
        let repo = self.room_repo.clone();
        let ai_repo = self.ai_config_repo.clone();
        let ai_tx = self.ai_worker_tx.clone();
        let active_rooms = self.active_rooms.clone();
        let supervisor = self.supervisor.clone();
        let event_store = self.event_store.clone();

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
                    SideEffect::SaveEngineState { room_id: rid, engine_state } => {
                        if let Ok(state_str) = serde_json::to_string(&engine_state) {
                            let _ = repo.save_engine_state(&rid, &state_str).await;
                        }
                    }
                    SideEffect::AppendEvent { event_type, actor_id, payload, .. } => {
                        let _ = event_store.append(&room_id, &event_type, &actor_id, &payload).await;
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
                    SideEffect::StreamChunk {
                        actor_id,
                        content,
                        is_done,
                    } => {
                        if let Some(room_tx) = active_rooms.get(&room_id) {
                            let _ = room_tx
                                .send(crate::room::model::RoomCommand::BroadcastStreamChunk {
                                    actor_id,
                                    content,
                                    is_done,
                                })
                                .await;
                        }
                    }
                    SideEffect::AiFailed { actor_id, error } => {
                        tracing::warn!(
                            room_id = %room_id, actor_id = %actor_id,
                            delay_secs = AI_RETRY_DELAY_SECS, error = %error,
                            "AI 动作失败，{}s 后自动重试（actor 状态机控制次数）", AI_RETRY_DELAY_SECS
                        );
                        if let Some(room_tx) = active_rooms.get(&room_id).map(|r| r.value().clone()) {
                            tokio::spawn(async move {
                                tokio::time::sleep(std::time::Duration::from_secs(AI_RETRY_DELAY_SECS)).await;
                                let _ = room_tx.send(RoomCommand::RetryAi { actor_id }).await;
                            });
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

        let slot = snapshot
            .actor_slots
            .iter()
            .find(|s| s.slot_name == slot_name)
            .ok_or(AppError::Forbidden)?;
        slot.authorize(&user_id).map_err(AppError::Room)?;

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
        let mut restored = 0usize;
        let mut skipped = 0usize;
        for snap in &snapshots {
            // 跳过已标记为 __defaults__ 的内部房间
            if snap.room_id == "__defaults__" {
                continue;
            }

            let factory = match self.game_registry.get(&snap.game_type) {
                Some(f) => f,
                None => {
                    tracing::warn!(room_id = %snap.room_id, game_type = %snap.game_type, "不支持的遊戲类型，跳过");
                    skipped += 1;
                    continue;
                }
            };

            let engine = match factory.restore(&snap.engine_state) {
                Ok(eng) => eng,
                Err(e) => {
                    tracing::error!(room_id = %snap.room_id, game_type = %snap.game_type, error = ?e, "引擎恢复失败，跳过");
                    skipped += 1;
                    continue;
                }
            };

            // 已结束的游戏不启动 Actor（保留存档供回放）
            if engine.is_finished() {
                tracing::info!(room_id = %snap.room_id, "游戏已结束，跳过 Actor 启动（保留存档）");
                skipped += 1;
                continue;
            }

            let (effect_tx, effect_rx) = mpsc::channel::<SideEffect>(64);
            let effect_tx_for_handler = effect_tx.clone();
            let room_tx = spawn_game_room(snap.room_id.clone(), engine, effect_tx);
            self.spawn_effect_handler(snap.room_id.clone(), effect_tx_for_handler, effect_rx);
            self.active_rooms
                .insert(snap.room_id.clone(), room_tx.clone());

            self.supervisor.track(snap.room_id.clone(), room_tx).await;
            tracing::info!(room_id = %snap.room_id, game_type = %snap.game_type, "房间已成功恢复并加入保活监控");
            restored += 1;
        }
        tracing::info!(total = %snapshots.len(), restored, skipped, "restore_all 完成");
        Ok(())
    }

    pub async fn list_public_rooms(
        &self,
        game_type: Option<&str>,
        page: usize,
        per_page: usize,
    ) -> Result<(Vec<RoomSnapshot>, usize), AppError> {
        let (rooms, total) = self.room_repo
            .list_public_filtered(game_type, page.max(1), per_page.max(1).min(100))
            .await
            .map_err(AppError::Room)?;
        Ok((rooms, total))
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
            .find(|s| s.slot_name == slot_name);

        let is_new_occupancy = match slot {
            Some(slot) => match &slot.occupant {
                self::model::ActorOccupant::Empty => {
                    slot.occupant = self::model::ActorOccupant::Human(user_id.clone());
                    true
                }
                self::model::ActorOccupant::Human(id) if id == &user_id => {
                    return Ok(()); // already occupied by same user
                }
                _ => {
                    return Err(AppError::Forbidden); // slot taken or is AI
                }
            },
            None if slot_name == "spectator" => {
                // Spectator: create a new slot dynamically
                snapshot.actor_slots.push(self::model::ActorSlot {
                    slot_name: slot_name.to_string(),
                    occupant: self::model::ActorOccupant::Human(user_id.clone()),
                });
                true
            }
            None => {
                return Err(AppError::Forbidden);
            }
        };

        self.room_repo
            .save(&snapshot)
            .await
            .map_err(AppError::Room)?;

        // 通知引擎槽位已被占据
        if is_new_occupancy {
            if let Some(room_tx) = self.active_rooms.get(room_id) {
                let _ = room_tx
                    .send(RoomCommand::SlotOccupied {
                        slot_name: slot_name.to_string(),
                        user_id,
                    })
                    .await;
            }
        }

        Ok(())
    }
}

fn build_slots(input: &CreateRoomInput, owner_id: &UserId) -> Vec<self::model::ActorSlot> {
    let mut slots: Vec<self::model::ActorSlot> = input
        .slots
        .iter()
        .map(|name| {
            let occupant = match input.slot_configs.get(name).map(|s| s.as_str()) {
                Some("human") => {
                    if name == &input.my_slot {
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
        .collect();

    if input.my_slot == "spectator" {
        slots.push(self::model::ActorSlot {
            slot_name: "spectator".to_string(),
            occupant: self::model::ActorOccupant::Human(owner_id.clone()),
        });
    }

    slots
}
