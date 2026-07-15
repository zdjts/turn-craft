use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::{error, info, warn};

// ═══════════════════════════════════════════════════════
//  Token 管理
// ═══════════════════════════════════════════════════════

pub fn get_token() -> Option<String> {
    web_sys::window()?
        .local_storage()
        .ok()??
        .get_item("token")
        .ok()?
}

pub fn set_token(token: &str) {
    if let Some(win) = web_sys::window() {
        if let Ok(Some(storage)) = win.local_storage() {
            let _ = storage.set_item("token", token);
        }
    }
}

pub fn remove_token() {
    if let Some(win) = web_sys::window() {
        if let Ok(Some(storage)) = win.local_storage() {
            let _ = storage.remove_item("token");
        }
    }
}

pub fn set_username(name: &str) {
    if let Some(win) = web_sys::window() {
        if let Ok(Some(storage)) = win.local_storage() {
            let _ = storage.set_item("username", name);
        }
    }
}

pub fn get_username() -> Option<String> {
    web_sys::window()?
        .local_storage()
        .ok()??
        .get_item("username")
        .ok()?
}

pub fn remove_username() {
    if let Some(win) = web_sys::window() {
        if let Ok(Some(storage)) = win.local_storage() {
            let _ = storage.remove_item("username");
        }
    }
}

// ═══════════════════════════════════════════════════════
//  注册 / 登录 API
// ═══════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize)]
pub struct AuthRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AuthResponse {
    pub status: String,
    pub token: Option<String>,
    pub message: Option<String>,
}

pub async fn register(req: &AuthRequest) -> Result<AuthResponse, String> {
    let url = format!("{}/register", crate::BACKEND_ORIGIN);
    let body_str = serde_json::to_string(req).map_err(|e| e.to_string())?;

    let resp = gloo_net::http::Request::post(&url)
        .header("Content-Type", "application/json")
        .body(body_str)
        .map_err(|e| format!("构建请求失败: {e}"))?
        .send()
        .await
        .map_err(|e| format!("HTTP 请求失败: {e}"))?;

    if !resp.ok() {
        return Err(format!("HTTP {}: {}", resp.status(), resp.status_text()));
    }

    let body: AuthResponse = resp
        .json()
        .await
        .map_err(|e| format!("解析响应失败: {e}"))?;
    Ok(body)
}

pub async fn login(req: &AuthRequest) -> Result<AuthResponse, String> {
    let url = format!("{}/login", crate::BACKEND_ORIGIN);
    let body_str = serde_json::to_string(req).map_err(|e| e.to_string())?;

    let resp = gloo_net::http::Request::post(&url)
        .header("Content-Type", "application/json")
        .body(body_str)
        .map_err(|e| format!("构建请求失败: {e}"))?
        .send()
        .await
        .map_err(|e| format!("HTTP 请求失败: {e}"))?;

    if !resp.ok() {
        return Err(format!("HTTP {}: {}", resp.status(), resp.status_text()));
    }

    let body: AuthResponse = resp
        .json()
        .await
        .map_err(|e| format!("解析响应失败: {e}"))?;
    Ok(body)
}

// ═══════════════════════════════════════════════════════
//  房间 API
// ═══════════════════════════════════════════════════════

/// 创建房间请求
#[derive(Debug, Clone, Serialize)]
pub struct CreateRoomRequest {
    pub game_type: String,
    pub max_round: usize,
    pub my_slot: String,
    pub slots: Vec<String>,
    pub slot_configs: HashMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub game_config: Option<Value>,
    pub is_public: bool,
}

/// 创建房间响应
#[derive(Debug, Clone, Deserialize)]
pub struct CreateRoomResponse {
    pub status: String,
    pub room_id: Option<String>,
    pub actor_id: Option<String>,
    pub message: Option<String>,
}

pub async fn create_room(req: &CreateRoomRequest) -> Result<CreateRoomResponse, String> {
    let url = format!("{}/rooms", crate::BACKEND_ORIGIN);

    info!(target: "api", url = %url, game_type = %req.game_type, my_slot = %req.my_slot, "发起创建房间请求");

    let body_str = serde_json::to_string(req).map_err(|e| {
        error!(target: "api", error = %e, "请求体序列化失败");
        e.to_string()
    })?;

    let mut request =
        gloo_net::http::Request::post(&url).header("Content-Type", "application/json");

    if let Some(token) = get_token() {
        request = request.header("Authorization", &format!("Bearer {token}"));
    }

    let resp = request
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

/// AI 配置数据
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct AiConfigData {
    pub api_key: String,
    pub base_url: String,
    pub model: String,
    pub max_tokens: u32,
    pub prompt: String,
    #[serde(default)]
    pub style: String,
}

pub async fn get_ai_configs(room_id: &str) -> Result<HashMap<String, AiConfigData>, String> {
    let url = format!("{}/rooms/{}/ai-config", crate::BACKEND_ORIGIN, room_id);

    info!(target: "api", url = %url, "GET AI 配置");

    let mut request = gloo_net::http::Request::get(&url);
    if let Some(token) = get_token() {
        request = request.header("Authorization", &format!("Bearer {token}"));
    }

    let resp = request
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
        crate::BACKEND_ORIGIN,
        room_id,
        actor_id
    );

    info!(target: "api", url = %url, "PUT AI 配置");

    let body_str = serde_json::to_string(config).map_err(|e| e.to_string())?;

    let mut request = gloo_net::http::Request::put(&url).header("Content-Type", "application/json");

    if let Some(token) = get_token() {
        request = request.header("Authorization", &format!("Bearer {token}"));
    }

    let resp = request
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

// ═══════════════════════════════════════════════════════
//  房间列表 API
// ═══════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RoomSnapshotData {
    pub room_id: String,
    pub owner_id: String,
    pub game_type: String,
    pub engine_state: Value,
    pub actor_slots: Value,
    pub ai_configs: Value,
    pub max_round: usize,
    pub created_at: String,
    pub is_public: bool,
}

pub async fn get_public_rooms() -> Result<Vec<RoomSnapshotData>, String> {
    let url = format!("{}/rooms/public", crate::BACKEND_ORIGIN);
    let mut request = gloo_net::http::Request::get(&url);
    if let Some(token) = get_token() {
        request = request.header("Authorization", &format!("Bearer {token}"));
    }
    let resp = request
        .send()
        .await
        .map_err(|e| format!("HTTP GET 失败: {e}"))?;
    if !resp.ok() {
        return Err(format!("HTTP {}", resp.status()));
    }
    let body: Value = resp.json().await.map_err(|e| format!("解析失败: {e}"))?;
    let rooms_val = body.get("rooms").ok_or("缺少 rooms 字段")?;
    let rooms: Vec<RoomSnapshotData> =
        serde_json::from_value(rooms_val.clone()).map_err(|e| e.to_string())?;
    Ok(rooms)
}

pub async fn get_history_rooms() -> Result<Vec<RoomSnapshotData>, String> {
    let url = format!("{}/rooms/history", crate::BACKEND_ORIGIN);
    let mut request = gloo_net::http::Request::get(&url);
    if let Some(token) = get_token() {
        request = request.header("Authorization", &format!("Bearer {token}"));
    }
    let resp = request
        .send()
        .await
        .map_err(|e| format!("HTTP GET 失败: {e}"))?;
    if !resp.ok() {
        return Err(format!("HTTP {}", resp.status()));
    }
    let body: Value = resp.json().await.map_err(|e| format!("解析失败: {e}"))?;
    let rooms_val = body.get("rooms").ok_or("缺少 rooms 字段")?;
    let rooms: Vec<RoomSnapshotData> =
        serde_json::from_value(rooms_val.clone()).map_err(|e| e.to_string())?;
    Ok(rooms)
}

pub async fn join_room(room_id: &str, slot_name: &str) -> Result<(), String> {
    let url = format!("{}/rooms/{}/join", crate::BACKEND_ORIGIN, room_id);
    let mut request = gloo_net::http::Request::post(&url);
    if let Some(token) = get_token() {
        request = request.header("Authorization", &format!("Bearer {token}"));
    }

    let payload = serde_json::json!({
        "slot_name": slot_name
    });

    let resp = request
        .json(&payload)
        .map_err(|e| format!("序列化失败: {e}"))?
        .send()
        .await
        .map_err(|e| format!("HTTP POST 失败: {e}"))?;

    if !resp.ok() {
        let err_msg = resp.text().await.unwrap_or_default();
        return Err(format!("加入房间失败: HTTP {} {}", resp.status(), err_msg));
    }
    Ok(())
}

pub async fn set_room_public(room_id: &str, is_public: bool) -> Result<(), String> {
    let url = format!("{}/rooms/{}/public", crate::BACKEND_ORIGIN, room_id);
    let body = serde_json::json!({ "is_public": is_public });
    let body_str = body.to_string();
    let mut request = gloo_net::http::Request::put(&url).header("Content-Type", "application/json");
    if let Some(token) = get_token() {
        request = request.header("Authorization", &format!("Bearer {token}"));
    }
    let resp = request
        .body(body_str)
        .map_err(|e| format!("构建请求失败: {e}"))?
        .send()
        .await
        .map_err(|e| format!("HTTP 请求失败: {e}"))?;
    if !resp.ok() {
        return Err(format!("HTTP {}", resp.status()));
    }
    Ok(())
}

pub async fn delete_room(room_id: &str) -> Result<(), String> {
    let url = format!("{}/rooms/{}", crate::BACKEND_ORIGIN, room_id);
    let mut request = gloo_net::http::Request::delete(&url);
    if let Some(token) = get_token() {
        request = request.header("Authorization", &format!("Bearer {token}"));
    }
    let resp = request
        .send()
        .await
        .map_err(|e| format!("HTTP 请求失败: {e}"))?;
    if !resp.ok() {
        return Err(format!("HTTP {}", resp.status()));
    }
    Ok(())
}

pub async fn get_room(room_id: &str) -> Result<RoomSnapshotData, String> {
    let url = format!("{}/rooms/{}", crate::BACKEND_ORIGIN, room_id);
    let mut request = gloo_net::http::Request::get(&url);
    if let Some(token) = get_token() {
        request = request.header("Authorization", &format!("Bearer {token}"));
    }
    let resp = request
        .send()
        .await
        .map_err(|e| format!("HTTP 请求失败: {e}"))?;
    if !resp.ok() {
        return Err(format!("HTTP {}", resp.status()));
    }
    let body: Value = resp.json().await.map_err(|e| format!("解析失败: {e}"))?;
    let room_val = body.get("room").ok_or("缺少 room 字段")?;
    let room: RoomSnapshotData =
        serde_json::from_value(room_val.clone()).map_err(|e| e.to_string())?;
    Ok(room)
}
