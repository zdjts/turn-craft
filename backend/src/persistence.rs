use std::collections::HashMap;
use std::path::Path;

use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::{error, info};

use crate::ai::env::AiConfig;

const ROOMS_FILE: &str = "rooms.json";
const CONFIG_FILE: &str = "ai_configs.json";

/// 房间快照：用于持久化存储房间状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomSnapshot {
    pub room_id: String,
    pub game_type: String,
    pub engine_state: Value,
    pub role_config: HashMap<String, String>,
    pub ai_configs: HashMap<String, AiConfig>,
    pub max_round: usize,
}

/// 将所有房间快照持久化到文件
pub fn save_rooms(snapshots: &DashMap<String, RoomSnapshot>) {
    let map: HashMap<String, RoomSnapshot> = snapshots
        .iter()
        .map(|entry| (entry.key().clone(), entry.value().clone()))
        .collect();

    match serde_json::to_string_pretty(&map) {
        Ok(json) => {
            if let Err(e) = std::fs::write(ROOMS_FILE, json) {
                error!(file = ROOMS_FILE, error = %e, "写入房间快照文件失败");
            }
        }
        Err(e) => {
            error!(error = %e, "序列化房间快照失败");
        }
    }
}

/// 从文件加载房间快照
pub fn load_rooms() -> DashMap<String, RoomSnapshot> {
    let map = DashMap::new();
    if !Path::new(ROOMS_FILE).exists() {
        info!(file = ROOMS_FILE, "房间快照文件不存在，跳过恢复");
        return map;
    }
    match std::fs::read_to_string(ROOMS_FILE) {
        Ok(json) => match serde_json::from_str::<HashMap<String, RoomSnapshot>>(&json) {
            Ok(parsed) => {
                let count = parsed.len();
                for (k, v) in parsed {
                    map.insert(k, v);
                }
                info!(file = ROOMS_FILE, count, "房间快照已从文件加载");
            }
            Err(e) => {
                error!(file = ROOMS_FILE, error = %e, "房间快照文件解析失败，忽略");
            }
        },
        Err(e) => {
            error!(file = ROOMS_FILE, error = %e, "读取房间快照文件失败");
        }
    }
    map
}

/// 单个房间保存（更新或插入）
pub fn save_room_snapshot(
    snapshots: &DashMap<String, RoomSnapshot>,
    room_id: &str,
    snapshot: RoomSnapshot,
) {
    snapshots.insert(room_id.to_string(), snapshot);
    save_rooms(snapshots);
}

/// 单个房间移除
pub fn remove_room_snapshot(snapshots: &DashMap<String, RoomSnapshot>, room_id: &str) {
    snapshots.remove(room_id);
    save_rooms(snapshots);
}

/// 从文件加载 AI 配置
pub fn load_configs_from_file() -> DashMap<String, AiConfig> {
    let map = DashMap::new();
    if !Path::new(CONFIG_FILE).exists() {
        info!(file = CONFIG_FILE, "AI 配置文件不存在，使用空配置");
        return map;
    }
    match std::fs::read_to_string(CONFIG_FILE) {
        Ok(json) => match serde_json::from_str::<HashMap<String, AiConfig>>(&json) {
            Ok(parsed) => {
                let count = parsed.len();
                for (k, v) in parsed {
                    map.insert(k, v);
                }
                info!(file = CONFIG_FILE, count, "AI 配置已从文件加载");
            }
            Err(e) => {
                error!(file = CONFIG_FILE, error = %e, "AI 配置文件解析失败，忽略");
            }
        },
        Err(e) => {
            error!(file = CONFIG_FILE, error = %e, "读取 AI 配置文件失败");
        }
    }
    map
}

/// 将当前 AI 配置持久化到文件（在 handler 中调用）
pub fn save_configs_to_file(configs: &DashMap<String, AiConfig>) {
    let map: HashMap<String, AiConfig> = configs
        .iter()
        .map(|entry| (entry.key().clone(), entry.value().clone()))
        .collect();
    match serde_json::to_string_pretty(&map) {
        Ok(json) => {
            if let Err(e) = std::fs::write(CONFIG_FILE, json) {
                error!(file = CONFIG_FILE, error = %e, "写入 AI 配置文件失败");
            }
        }
        Err(e) => {
            error!(error = %e, "序列化 AI 配置失败");
        }
    }
}
