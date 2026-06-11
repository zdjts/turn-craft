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

#[tokio::test]
async fn test_full_game_flow() -> Result<(), Box<dyn Error>> {
    let http_client = Client::new();

    println!("========== 📑 步骤 1: 正在自动发起 HTTP 请求创建房间 ==========");

    // 构造角色配置：judge 为人类，正方/反方为 AI
    let mut role_config = HashMap::new();
    role_config.insert("judge".to_string(), "human".to_string());
    role_config.insert("pro".to_string(), "ai".to_string());
    role_config.insert("con".to_string(), "ai".to_string());

    let payload = CreateRoomPayload {
        game_type: "lincoln".to_string(),
        max_round: 16,
        my_role: "judge".to_string(),
        role_config,
        game_config: None,
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

    println!("========== 🔌 步骤 2: 正在连接 WebSocket 长连接网关 ==========");
    let ws_url = format!("{}/ws/{}/{}", BASE_WS_URL, room_id, actor_id);
    println!("🔗 正在握手: {}", ws_url);

    let (ws_stream, _) = connect_async(ws_url).await?;
    println!("✅ 长连接成功建立！你已作为【真人裁判】进入房间。");
    println!("💡 提示：输入任意文本并回车即可发言；输入 'exit' 可主动销毁房间退出。\n");

    let (mut ws_write, mut ws_read) = ws_stream.split();

    let read_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = ws_read.next().await {
            match msg {
                Message::Text(text) => {
                    if text == "game_over" {
                        println!("\n📢 [系统通知]: 游戏已结束，请裁判做最终裁决。");
                    } else {
                        println!("\n📥 [全场广播]: {}", text);
                    }
                }
                Message::Close(_) => {
                    println!("\n🔌 [网络连接]: 服务器长连接已断开。");
                    break;
                }
                _ => {}
            }
        }
    });

    let stdin = tokio::io::stdin();
    let mut reader = BufReader::new(stdin).lines();

    println!("⚖️ 请宣读你的开场白和辩题（例如：'辩题是AI是否取代程序员，请正方开始'）：");

    while let Some(line) = reader.next_line().await? {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if trimmed == "exit" {
            println!("👋 正在准备退出并清理房间...");
            break;
        }

        if let Err(e) = ws_write
            .send(Message::Text(trimmed.to_string().into()))
            .await
        {
            println!("❌ 发送动作失败: {:?}", e);
            break;
        }
        println!("📤 [你已发言]: {}", trimmed);
    }

    read_task.abort();

    println!("\n💥 === 步骤 3: 发起 HTTP DELETE 请求销毁房间 ===");
    let del_res = http_client
        .delete(format!("{}/rooms/{}", BASE_HTTP_URL, room_id))
        .send()
        .await;

    match del_res {
        Ok(response) => {
            if response.status().is_success() {
                println!(
                    "✅ 服务器响应：房间 {} 内存已成功回收，进程优雅退出。",
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
