pub struct RoomService {
    room_repo: Arc<dyn RoomRepository>,
    ai_config_repo: Arc<dyn AiConfigRepository>,
    ai_worker_tx: mpsc::Sender<AiTask>,
    active_rooms: Arc<DashMap<String, mpsc::Sender<RoomCommand>>>,
}

impl RoomService {
    pub async fn create_room(
        &self,
        owner_id: UserId,
        input: CreateRoomInput,
    ) -> Result<CreateRoomOutput, AppError> {
        let room_id = format!("room_{}", uuid::Uuid::new_v4());

        // 1. 构建 ActorSlot 列表
        let slots = build_slots(&input, &owner_id);
        let role_config = make_role_config(&slots);

        // 2. 创建引擎 (game factory 不再收 DashMap)
        let (engine, ai_configs) = create_engine(
            &room_id,
            &input.game_type,
            &role_config,
            &input.my_slot,
            input.max_round,
            &input.game_config,
        )?;

        // 3. 保存 AI 配置
        for (actor_id, config) in &ai_configs {
            self.ai_config_repo.set(&room_id, actor_id, config).await?;
        }

        // 4. 保存房间
        let snapshot = RoomSnapshot {
            room_id: room_id.clone(),
            owner_id,
            game_type: input.game_type,
            engine_state: engine.to_json(),
            actor_slots: slots,
            ai_configs,
            max_round: input.max_round,
            created_at: chrono::Utc::now().naive_utc(),
        };
        self.room_repo.save(&snapshot).await?;

        // 5. 启动 RoomActor + side effect handler
        let (effect_tx, effect_rx) = mpsc::channel::<SideEffect>(64);
        let room_tx = spawn_game_room(room_id.clone(), engine, effect_tx);
        self.spawn_effect_handler(room_id.clone(), effect_rx);
        self.active_rooms.insert(room_id.clone(), room_tx);

        Ok(CreateRoomOutput {
            room_id,
            assigned_slot: input.my_slot,
        })
    }

    fn spawn_effect_handler(&self, room_id: String, mut rx: mpsc::Receiver<SideEffect>) {
        let repo = self.room_repo.clone();
        let ai_repo = self.ai_config_repo.clone();
        let ai_tx = self.ai_worker_tx.clone();

        tokio::spawn(async move {
            while let Some(effect) = rx.recv().await {
                match effect {
                    SideEffect::TriggerAi { actor_id, snapshot } => {
                        if let Ok(config) = ai_repo.get(&room_id, &actor_id).await {
                            let _ = ai_tx
                                .send(AiTask {
                                room_id: room_id.clone(),
                                actor_id,
                                snapshot,
                                ai_config: config,
                                tools: None,
                                retries: 0,
                                reply_tx: /* 需要 Room Actor 的 tx */ todo!(),
                            })
                                .await;
                        }
                    }
                    SideEffect::PersistSnapshot(snapshot) => {
                        let _ = repo.save(&snapshot).await;
                    }
                    SideEffect::GameOver => {
                        tracing::info!(room_id = %room_id, "游戏结束");
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
        slot.authorize(&user_id)?;

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

        Ok(())
    }

    pub async fn restore_all(&self) -> Result<(), AppError> {
        // 从 SQLite 加载所有房间 → 重建引擎 → 启动 Actor
        todo!()
    }
}
