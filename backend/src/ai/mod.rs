pub mod client;
pub mod config_repo;
pub mod env;
pub mod error;
pub mod insights;
pub mod listener;

use crate::ai::config_repo::AiConfigRepository;
use std::sync::Arc;

pub struct AIService {
    pub config_repo: Arc<dyn AiConfigRepository>,
}

impl AIService {
    pub fn new(config_repo: Arc<dyn AiConfigRepository>) -> Self {
        Self { config_repo }
    }
}
