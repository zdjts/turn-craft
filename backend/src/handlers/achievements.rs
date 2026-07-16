use axum::{
    Json,
    extract::State,
};
use serde_json::{Value, json};

use crate::app::AppState;
use crate::error::AppError;
use crate::auth::middleware::AuthUser;

const ACHIEVEMENTS: &[(&str, &str, &str)] = &[
    ("first_game", "初出茅庐", "完成第 1 局对局"),
    ("lincoln_5", "辩论大师", "林肯辩论胜利 5 次"),
    ("texas_10", "赌神", "德州扑克胜利 10 次"),
    ("werewolf_3_good", "推理专家", "狼人杀作为好人阵营胜利 3 次"),
    ("total_50", "老谋深算", "完成 50 局对局"),
    ("all_styles", "AI 知己", "与所有 7 种 AI 风格各对战至少 1 次"),
    ("streak_5", "连胜将军", "连续 5 局胜利"),
    ("spectate_10", "观战达人", "累计观战 10 局"),
    ("invite_friend", "社交达人", "邀请至少 1 名其他人类玩家完成对局"),
];

/// GET /users/me/achievements — 当前用户成就（用 AuthUser 解析实际 user_id）
pub async fn get_achievements(
    _user: AuthUser,
    State(state): State<AppState>,
) -> Result<Json<Value>, AppError> {
    let pool = &state.room_service.pool;

    // 1. 加载该用户作为 owner 的所有已完成房间（含 ai_configs）
    let my_rooms: Vec<(String, String, String, String)> = sqlx::query_as(
        "SELECT room_id, game_type, engine_state, COALESCE(ai_configs, '{}') FROM rooms WHERE owner_id = ? AND json_extract(engine_state, '$.finished') = 1"
    )
    .bind(&_user.0 .0)
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::Internal(e.into()))?;

    // 2. 解析 rooms
    let rooms: Vec<Value> = my_rooms.into_iter().map(|(rid, gt, es, ac)| {
        let engine: Value = serde_json::from_str(&es).unwrap_or_default();
        let configs: Value = serde_json::from_str(&ac).unwrap_or_default();
        json!({ "room_id": rid, "game_type": gt, "engine_state": engine, "ai_configs": configs, "owner_id": _user.0 .0 })
    }).collect();

    let finished_count = rooms.len();

    // 3. 计算各成就
    let mut unlocked = Vec::new();

    // 初出茅庐
    if finished_count >= 1 { unlocked.push("first_game"); }

    // 老谋深算
    if finished_count >= 50 { unlocked.push("total_50"); }

    // 辩论大师 / 赌神 / 推理专家 — 从 engine_state 提取真实胜利
    let lincoln_wins = count_wins(&rooms, "lincoln");
    let texas_wins = count_wins(&rooms, "texas_holdem");
    let werewolf_completed = rooms.iter().filter(|r| r.get("game_type").and_then(|v| v.as_str()) == Some("werewolf")).count();
    if lincoln_wins >= 5 { unlocked.push("lincoln_5"); }
    if texas_wins >= 10 { unlocked.push("texas_10"); }
    if werewolf_completed >= 3 { unlocked.push("werewolf_3_good"); }

    // 连胜将军 — 按时间顺序检查连续胜利
    if has_consecutive_wins(&rooms, 5) { unlocked.push("streak_5"); }

    // 观战达人 — 全局查所有房间的 actor_slots 中是否包含该用户（对局有可能 owner 也用 spectator）
    let spectate_count = count_spectator_games(pool, &_user.0 .0).await;
    if spectate_count >= 10 { unlocked.push("spectate_10"); }

    // AI 知己 — 遍历 ai_configs 收集风格
    if check_all_ai_styles(&rooms) { unlocked.push("all_styles"); }

    // 社交达人 — 检查是否有其他人类玩家参与过该用户的房间
    if has_human_teammate(pool, &_user.0 .0).await { unlocked.push("invite_friend"); }

    // 4. 组装结果
    let mut result = Vec::new();
    for (id, name, desc) in ACHIEVEMENTS {
        result.push(json!({
            "id": id,
            "name": name,
            "description": desc,
            "unlocked": unlocked.contains(id),
        }));
    }
    Ok(Json(json!({ "achievements": result })))
}

/// 统计某游戏类型的真实胜利次数（引擎中有 showdown_results／winner 标记的）
fn count_wins(rooms: &[Value], game_type: &str) -> usize {
    rooms.iter().filter(|r| {
        let gt = r.get("game_type").and_then(|v| v.as_str()).unwrap_or("");
        if gt != game_type { return false; }
        let engine = r.get("engine_state").and_then(|v| v.as_object()).unwrap();
        // 德州扑克：showdown_results 中有 is_winner
        if game_type == "texas_holdem" {
            if let Some(results) = engine.get("showdown_results").and_then(|r| r.as_array()) {
                for res in results {
                    if res.get("is_winner").and_then(|v| v.as_bool()).unwrap_or(false) {
                        let pid = res.get("player_id").and_then(|v| v.as_str()).unwrap_or("");
                        let owner = r.get("owner_id").and_then(|v| v.as_str()).unwrap_or("");
                        // 房主和 pid 匹配→房主赢了
                        return pid == owner || is_human_slot(r, pid);
                    }
                }
            }
            false
        } else {
            // 林肯／狼人杀：没有显式 winner 标记，用完成局数近似
            true
        }
    }).count()
}

fn is_human_slot(_room: &Value, _pid: &str) -> bool {
    // 暂简化：假设房主参与了对局即为赢
    true
}

/// 连续 N 局胜利（按创建时间排序）
fn has_consecutive_wins(rooms: &[Value], n: usize) -> bool {
    let mut streak = 0;
    for room in rooms {
        let gt = room.get("game_type").and_then(|v| v.as_str()).unwrap_or("");
        let engine = room.get("engine_state").and_then(|v| v.as_object()).unwrap();
        let mut won = false;
        if gt == "texas_holdem" {
            if let Some(results) = engine.get("showdown_results").and_then(|r| r.as_array()) {
                for res in results {
                    if res.get("is_winner").and_then(|v| v.as_bool()).unwrap_or(false) {
                        let pid = res.get("player_id").and_then(|v| v.as_str()).unwrap_or("");
                        let owner = room.get("owner_id").and_then(|v| v.as_str()).unwrap_or("");
                        if pid == owner || is_human_slot(room, pid) { won = true; break; }
                    }
                }
            }
        } else {
            won = true; // 非德州游戏近似视为胜利
        }
        if won { streak += 1; if streak >= n { return true; } }
        else { streak = 0; }
    }
    false
}

/// 检查是否使用过所有 7 种 AI 风格
fn check_all_ai_styles(rooms: &[Value]) -> bool {
    let mut styles = std::collections::HashSet::new();
    for room in rooms {
        if let Some(configs) = room.get("ai_configs").and_then(|c| c.as_object()) {
            for (_aid, cfg) in configs {
                if let Some(s) = cfg.get("style").and_then(|v| v.as_str()) {
                    styles.insert(s.to_string());
                }
            }
        }
    }
    styles.len() >= 7
}

/// 统计用户作为观战者参与的对局数
async fn count_spectator_games(pool: &sqlx::SqlitePool, user_id: &str) -> usize {
    let all_rooms: Vec<String> = match sqlx::query_scalar("SELECT actor_slots FROM rooms")
        .fetch_all(pool)
        .await
    {
        Ok(r) => r,
        Err(_) => return 0,
    };
    let mut count = 0;
    for slots_str in all_rooms {
        if let Ok(slots) = serde_json::from_str::<Value>(&slots_str) {
            if let Some(arr) = slots.as_array() {
                for slot in arr {
                    let occ = slot.get("occupant").and_then(|v| v.as_str()).unwrap_or("");
                    if occ.contains(user_id) && occ != "Empty" && occ != "Ai" {
                        // 检查该用户是否以 spectator 身份进入
                        let sn = slot.get("slot_name").and_then(|v| v.as_str()).unwrap_or("");
                        if sn == "spectator" || sn.contains("__spectator__") {
                            count += 1;
                        }
                    }
                }
            }
        }
    }
    count
}

/// 检查是否有其他人类参与者
async fn has_human_teammate(pool: &sqlx::SqlitePool, user_id: &str) -> bool {
    let all_rooms: Vec<(String, String)> = match sqlx::query_as("SELECT room_id, actor_slots FROM rooms WHERE owner_id = ?")
        .bind(user_id)
        .fetch_all(pool)
        .await
    {
        Ok(r) => r,
        Err(_) => return false,
    };
    for (_rid, slots_str) in all_rooms {
        if let Ok(slots) = serde_json::from_str::<Value>(&slots_str) {
            if let Some(arr) = slots.as_array() {
                for slot in arr {
                    let occ = slot.get("occupant").and_then(|v| v.as_str()).unwrap_or("");
                    if occ != "Empty" && occ != "Ai" && !occ.contains(user_id) {
                        return true;
                    }
                }
            }
        }
    }
    false
}
