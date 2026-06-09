use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::{error, info, warn};

#[derive(Debug, Clone, Serialize)]
pub struct CreateRoomRequest {
    pub game_type: String,
    pub max_round: usize,
    pub my_role: String,
    pub role_config: HashMap<String, String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateRoomResponse {
    pub status: String,
    pub room_id: Option<String>,
    pub actor_id: Option<String>,
    pub message: Option<String>,
}

pub async fn create_room(req: &CreateRoomRequest) -> Result<CreateRoomResponse, String> {
    let url = format!("{}/rooms", crate::BACKEND_ORIGIN);

    info!(target: "api", url = %url, game_type = %req.game_type, my_role = %req.my_role, "发起创建房间请求");

    let body_str = serde_json::to_string(req).map_err(|e| {
        error!(target: "api", error = %e, "请求体序列化失败");
        e.to_string()
    })?;

    let resp = gloo_net::http::Request::post(&url)
        .header("Content-Type", "application/json")
        .body(body_str)
        .map_err(|e| format!("构建请求失败: {e}"))?
        .send()
        .await
        .map_err(|e| format!("HTTP 请求失败: {e}"))?;

    if !resp.ok() {
        let status_code = resp.status();
        error!(target: "api", status = status_code, "服务器返回错误");
        return Err(format!("HTTP {}: {}", status_code, resp.status_text()));
    }

    let body: Value = resp
        .json()
        .await
        .map_err(|e| format!("响应解析失败: {e}"))?;

    let status = body
        .get("status")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();

    let room_id = body
        .get("room_id")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let actor_id = body
        .get("actor_id")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let message = body
        .get("message")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    if status == "success" {
        info!(target: "api", room_id = ?room_id, actor_id = ?actor_id, "房间创建成功");
    } else {
        warn!(target: "api", status = %status, message = ?message, "房间创建失败");
    }

    Ok(CreateRoomResponse {
        status,
        room_id,
        actor_id,
        message,
    })
}

// ═══════════════════════════════════════════════════════
//  AI 配置 API
// ═══════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct AiConfigData {
    pub api_key: String,
    pub base_url: String,
    pub model: String,
    pub max_tokens: u32,
    pub prompt: String,
}

pub async fn get_ai_configs(room_id: &str) -> Result<HashMap<String, AiConfigData>, String> {
    let url = format!("{}/rooms/{}/ai-config", crate::BACKEND_ORIGIN, room_id);

    info!(target: "api", url = %url, "GET AI 配置");

    let resp = gloo_net::http::Request::get(&url)
        .send()
        .await
        .map_err(|e| format!("HTTP GET 失败: {e}"))?;

    if !resp.ok() {
        let status = resp.status();
        error!(target: "api", status = status, "GET AI 配置失败");
        return Err(format!("HTTP {}", status));
    }

    let body: Value = resp
        .json()
        .await
        .map_err(|e| format!("响应解析失败: {e}"))?;

    let configs_map = body
        .get("configs")
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default();

    let mut result = HashMap::new();
    for (actor_id, cfg_val) in configs_map {
        if let Ok(cfg) = serde_json::from_value::<AiConfigData>(cfg_val) {
            result.insert(actor_id, cfg);
        }
    }

    info!(target: "api", count = result.len(), "GET AI 配置成功");
    Ok(result)
}

pub async fn update_ai_config(
    room_id: &str,
    actor_id: &str,
    config: &AiConfigData,
) -> Result<(), String> {
    let url = format!(
        "{}/rooms/{}/ai-config/{}",
        crate::BACKEND_ORIGIN, room_id, actor_id
    );

    info!(target: "api", url = %url, "PUT AI 配置");

    let body_str = serde_json::to_string(config).map_err(|e| e.to_string())?;

    let resp = gloo_net::http::Request::put(&url)
        .header("Content-Type", "application/json")
        .body(body_str)
        .map_err(|e| format!("构建请求失败: {e}"))?
        .send()
        .await
        .map_err(|e| format!("HTTP PUT 失败: {e}"))?;

    if !resp.ok() {
        let status = resp.status();
        error!(target: "api", status = status, "PUT AI 配置失败");
        return Err(format!("HTTP {}", status));
    }

    info!(target: "api", actor_id = %actor_id, "PUT AI 配置成功");
    Ok(())
}
