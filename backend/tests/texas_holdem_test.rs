use futures_util::{SinkExt, StreamExt};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

const BASE_HTTP_URL: &str = "http://127.0.0.1:8080";
const BASE_WS_URL: &str = "ws://127.0.0.1:8080";

/// 与服务端 CreateRoomInput 完全对应的请求体
#[derive(Serialize)]
struct CreateRoomPayload {
    game_type: String,
    max_round: usize,
    my_role: String,
    role_config: HashMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    game_config: Option<serde_json::Value>,
}

/// 服务端返回的响应体
#[derive(Deserialize)]
struct CreateRoomResponse {
    status: String,
    room_id: String,
    actor_id: String,
}

/// 德州扑克游戏配置
#[derive(Serialize)]
struct TexasHoldemGameConfig {
    small_blind: u32,
    big_blind: u32,
    starting_chips: u32,
}

#[tokio::test]
async fn test_texas_holdem_flow() -> Result<(), Box<dyn Error>> {
    let http_client = Client::new();

    println!("========== 🎰 步骤 1: 创建德州扑克房间 ==========");

    // 构造角色配置：1个人类玩家，3个AI玩家
    let mut role_config = HashMap::new();
    role_config.insert("player1".to_string(), "human".to_string());
    role_config.insert("player2".to_string(), "ai".to_string());
    role_config.insert("player3".to_string(), "ai".to_string());
    role_config.insert("player4".to_string(), "ai".to_string());

    let game_config = TexasHoldemGameConfig {
        small_blind: 10,
        big_blind: 20,
        starting_chips: 1000,
    };

    let payload = CreateRoomPayload {
        game_type: "texas_holdem".to_string(),
        max_round: 10, // 10轮牌局
        my_role: "player1".to_string(),
        role_config,
        game_config: Some(serde_json::to_value(game_config)?),
    };

    let res = http_client
        .post(format!("{}/rooms", BASE_HTTP_URL))
        .json(&payload)
        .send()
        .await?;

    if !res.status().is_success() {
        println!("❌ 房间创建失败，HTTP 状态码: {}", res.status());
        let body = res.text().await.unwrap_or_default();
        println!("   响应体: {}", body);
        return Ok(());
    }

    let res_data: CreateRoomResponse = res.json().await?;
    if res_data.status != "success" {
        println!("❌ 服务端返回错误: {:?}", res_data.status);
        return Ok(());
    }

    let room_id = res_data.room_id;
    let actor_id = res_data.actor_id;
    println!("✅ 房间创建成功！房号 ID: {}", room_id);
    println!("   你的 actor_id: {}\n", actor_id);

    println!("========== 🔌 步骤 2: 连接 WebSocket 进入牌桌 ==========");
    let ws_url = format!("{}/ws/{}/{}", BASE_WS_URL, room_id, actor_id);
    println!("🔗 正在连接: {}", ws_url);

    let (ws_stream, _) = connect_async(ws_url).await?;
    println!("✅ 连接成功！你已作为【玩家1】进入牌桌。");
    println!("💡 提示：输入动作并回车（如：fold, call, raise 50, all_in）");
    println!("   输入 'exit' 可退出游戏并销毁房间。\n");

    let (mut ws_write, mut ws_read) = ws_stream.split();

    let read_task = tokio::spawn(async move {
        let mut game_started = false;
        while let Some(Ok(msg)) = ws_read.next().await {
            match msg {
                Message::Text(text) => {
                    // 尝试解析JSON消息
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
                        // 检查是否是游戏状态更新
                        if let Some(phase) = json.get("phase").and_then(|v| v.as_str()) {
                            if phase == "dealing" && !game_started {
                                println!("🃏 游戏开始，正在发牌...");
                                game_started = true;
                            }
                        }
                        
                        // 检查是否是轮到我们行动
                        if let Some(active_player) = json.get("active_player").and_then(|v| v.as_str()) {
                            if active_player == actor_id {
                                println!("🎯 轮到你行动了！请输入你的动作：");
                            }
                        }
                        
                        // 打印游戏状态摘要
                        if let Some(players) = json.get("players").and_then(|v| v.as_array()) {
                            println!("👥 玩家状态:");
                            for player in players {
                                let id = player.get("id").and_then(|v| v.as_str()).unwrap_or("?");
                                let chips = player.get("chips").and_then(|v| v.as_u64()).unwrap_or(0);
                                let folded = player.get("folded").and_then(|v| v.as_bool()).unwrap_or(false);
                                let status = if folded { "已弃牌" } else { "在游戏中" };
                                println!("   {} - 筹码: {} - {}", id, chips, status);
                            }
                        }
                    } else if text == "game_over" {
                        println!("\n📢 [系统通知]: 游戏结束！");
                        break;
                    } else {
                        println!("\n📥 [服务器消息]: {}", text);
                    }
                }
                Message::Close(_) => {
                    println!("\n🔌 [网络连接]: 服务器连接已断开。");
                    break;
                }
                _ => {}
            }
        }
    });

    let stdin = tokio::io::stdin();
    let mut reader = BufReader::new(stdin).lines();

    println!("🎰 欢迎来到德州扑克牌桌！等待游戏开始...");
    println!("   游戏规则：");
    println!("   - 每轮下注：小盲注=10，大盲注=20");
    println!("   - 起始筹码：1000");
    println!("   - 输入动作示例：fold, call, raise 50, all_in\n");

    while let Some(line) = reader.next_line().await? {
        let trimmed = line.trim().to_lowercase();
        if trimmed.is_empty() {
            continue;
        }

        if trimmed == "exit" {
            println!("👋 正在退出游戏...");
            break;
        }

        // 验证动作格式
        let valid_actions = ["fold", "check", "call", "raise", "all_in"];
        let action_valid = if trimmed.starts_with("raise ") {
            let amount_part = &trimmed[6..];
            amount_part.parse::<u32>().is_ok()
        } else {
            valid_actions.contains(&trimmed.as_str())
        };

        if !action_valid {
            println!("❌ 无效动作！请输入：fold/check/call/raise [金额]/all_in");
            continue;
        }

        // 构建动作JSON
        let action_json = if trimmed.starts_with("raise ") {
            let amount: u32 = trimmed[6..].parse().unwrap();
            serde_json::json!({
                "action": "raise",
                "amount": amount
            })
        } else {
            serde_json::json!({
                "action": trimmed
            })
        };

        if let Err(e) = ws_write
            .send(Message::Text(action_json.to_string().into()))
            .await
        {
            println!("❌ 发送动作失败: {:?}", e);
            break;
        }
        println!("📤 [你执行了]: {}", trimmed);
    }

    read_task.abort();

    println!("\n💥 === 步骤 3: 销毁房间 ===");
    let del_res = http_client
        .delete(format!("{}/rooms/{}", BASE_HTTP_URL, room_id))
        .send()
        .await;

    match del_res {
        Ok(response) => {
            if response.status().is_success() {
                println!(
                    "✅ 服务器响应：房间 {} 已成功销毁。",
                    room_id
                );
            } else {
                println!(
                    "⚠️ 销毁请求发出，但服务器返回了异常状态码: {}",
                    response.status()
                );
            }
        }
        Err(e) => println!("❌ 销毁房间请求失败: {:?}", e),
    }

    Ok(())
}
