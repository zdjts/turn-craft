use async_trait::async_trait;
use platform_core::traits::{GameEngine, GameMeta};
use serde_json::Value;
use std::collections::HashMap;

use crate::ai::config_repo::AiConfigRepository;
use crate::ai::env::AiConfig;
use crate::error::AppError;
use crate::room::model::CreateRoomInput;

#[async_trait]
pub trait GameFactory: Send + Sync {
    fn game_type(&self) -> &str;
    fn meta(&self) -> GameMeta;
    async fn create(
        &self,
        room_id: &str,
        owner_id: &crate::user::model::UserId,
        input: &CreateRoomInput,
        config_repo: &dyn AiConfigRepository,
    ) -> Result<(Box<dyn GameEngine>, HashMap<String, AiConfig>), AppError>;
    fn restore(&self, state: &Value) -> Result<Box<dyn GameEngine>, AppError>;
}

pub struct GameRegistry {
    factories: HashMap<String, Box<dyn GameFactory>>,
}

impl GameRegistry {
    pub fn new() -> Self {
        Self {
            factories: HashMap::new(),
        }
    }
    pub fn register(&mut self, factory: Box<dyn GameFactory>) {
        self.factories
            .insert(factory.game_type().to_string(), factory);
    }
    pub fn get(&self, game_type: &str) -> Option<&dyn GameFactory> {
        self.factories.get(game_type).map(|b| b.as_ref())
    }
    #[allow(dead_code)]
    pub fn all_types(&self) -> Vec<&str> {
        self.factories.keys().map(|s| s.as_str()).collect()
    }
    pub fn all_meta(&self) -> Vec<GameMeta> {
        self.factories.values().map(|f| f.meta()).collect()
    }
}
