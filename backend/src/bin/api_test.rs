use futures_util::{SinkExt, StreamExt};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::error::Error;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

const BASE_HTTP_URL: &str = "http://127.0.0.1:8080";
const BASE_WS_URL: &str = "ws://127.0.0.1:8080";

#[derive(Serialize)]
struct CreateRoomPayload {
    game_type: String,
    max_round: usize,
    player_id: String,
}

#[derive(Deserialize)]
struct CreateRoomResponse {
    status: String,
    room_id: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let player_id = "judge_zeng";
    let http_client = Client::new();

    println!("========== 📑 步骤 1: 正在自动发起 HTTP 请求创建房间 ==========");
    let payload = CreateRoomPayload {
        game_type: "lincoln".to_string(),
        max_round: 16, // 允许激辩 6 个回合
        player_id: player_id.to_string(),
    };

    let res = http_client
        .post(format!("{}/rooms", BASE_HTTP_URL))
        .json(&payload)
        .send()
        .await?;

    if !res.status().is_success() {
        println!("❌ 房间创建失败，HTTP 状态码: {}", res.status());
        return Ok(());
    }

    let res_data: CreateRoomResponse = res.json().await?;
    let room_id = res_data.room_id;
    println!("✅ 房间创建成功！房号 ID: {}\n", room_id);

    println!("========== 🔌 步骤 2: 正在连接 WebSocket 长连接网关 ==========");
    // 严格对应你的最终路由：/ws/:room_id/:actor_id
    let ws_url = format!("{}/ws/{}/{}", BASE_WS_URL, room_id, player_id);
    println!("🔗 正在握手: {}", ws_url);

    let (ws_stream, _) = connect_async(ws_url).await?;
    println!("✅ 长连接成功建立！你已作为【真人裁判】进入房间。");
    println!("💡 提示：输入任意文本并回车即可发言；输入 'exit' 可主动销毁房间退出。\n");

    // 核心重构：将 WebSocket 拆分为【发送端】和【接收端】
    let (mut ws_write, mut ws_read) = ws_stream.split();

    // --------------------------------------------------------
    // 任务 A：异步协程 —— 常驻后台接收服务器广播并打印
    // --------------------------------------------------------
    let _read_room_id = room_id.clone();
    let read_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = ws_read.next().await {
            match msg {
                Message::Text(text) => {
                    // 如果后端游戏结束发送了特定的标志
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

    // --------------------------------------------------------
    // 任务 B：主线程流 —— 死循环捕捉终端输入（stdin）并发送
    // --------------------------------------------------------
    let stdin = tokio::io::stdin();
    let mut reader = BufReader::new(stdin).lines();

    println!("⚖️ 请宣读你的开场白和辩题（例如：“辩题是AI是否取代程序员，请正方开始”）：");

    while let Some(line) = reader.next_line().await? {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // 允许用户输入 exit 退出游戏
        if trimmed == "exit" {
            println!("👋 正在准备退出并清理房间...");
            break;
        }

        // 顺着网关，把终端输入的纯文本直接作为 Action 扔给服务器
        if let Err(e) = ws_write
            .send(Message::Text(trimmed.to_string().into()))
            .await
        {
            println!("❌ 发送动作失败: {:?}", e);
            break;
        }
        println!("📤 [你已发言]: {}", trimmed);
    }

    // --------------------------------------------------------
    // 📑 步骤 3: 优雅收尾，销毁房间 (DELETE /rooms/:room_id)
    // --------------------------------------------------------
    // 取消接收任务
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
