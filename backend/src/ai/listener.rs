use reqwest::Client;
use tracing::error;

use crate::{
    ai::client::request_speech,
    room::model::{AiTask, RoomCommand},
};

use super::env::build_messages;

/// AI 后台工作者：消费任务队列，调用 LLM API
pub struct AiWorker {
    http_client: Client,
}
impl AiWorker {
    pub fn new() -> Self {
        Self {
            http_client: Client::new(),
        }
    }
    /// 启动消费循环，持续处理 AI 任务
    pub async fn start_consuming(self, mut ai_rx: tokio::sync::mpsc::Receiver<AiTask>) {
        while let Some(task) = ai_rx.recv().await {
            let client_clone = self.http_client.clone();
            tokio::spawn(async move {
                if let Err(e) = Self::process_task(client_clone, task).await {
                    error!(" [AI Worker] 异步处理工单失败: {e}");
                }
            });
        }
    }
    /// 处理单个 AI 任务：调用 LLM 并回传结果
    async fn process_task(http: Client, mut task: AiTask) -> Result<(), String> {
        let max_retries = 3;
        let config = task.ai_config.clone();

        let mut messages_json: serde_json::Value =
            serde_json::from_str(&build_messages(&config, task.snapshot.clone())).unwrap();

        loop {
            tracing::info!(
                actor_id = %task.actor_id,
                retries = task.retries,
                messages = %messages_json.to_string(),
                ">>> 发送给 AI 的完整内容"
            );

            let ai_response = match request_speech(
                &http,
                &config,
                messages_json.to_string(),
                task.tools.as_ref(),
            )
            .await
            {
                Ok(response) => {
                    tracing::info!(
                        actor_id = %task.actor_id,
                        response = %response,
                        "<<< AI 返回的完整回复"
                    );
                    response
                }
                Err(e) => {
                    tracing::error!(actor_id = %task.actor_id, error = ?e, "请求 AI 接口发生错误");
                    if task.retries < max_retries {
                        task.retries += 1;
                        if let Some(arr) = messages_json.as_array_mut() {
                            arr.push(serde_json::json!({
                                    "role": "user",
                                    "content": format!("请求发生网络或调用错误: {:?}。请尝试重新输出。", e)
                                }));
                        }
                        continue;
                    } else {
                        let command = RoomCommand::PlayerAction {
                            actor_id: task.actor_id.clone(),
                            action: serde_json::json!({"content": "[思考超时，未能发言]"}),
                            feedback_tx: None,
                        };
                        let _ = task.reply_tx.send(command).await;
                        return Err(format!("{:?}", e));
                    }
                }
            };

            let (tx, rx) = tokio::sync::oneshot::channel();

            // 完整响应直接传给游戏引擎解析
            let command = RoomCommand::PlayerAction {
                actor_id: task.actor_id.clone(),
                action: ai_response,
                feedback_tx: Some(tx),
            };

            if let Err(e) = task.reply_tx.send(command).await {
                tracing::error!(error = ?e, "向房间回传 AI 动作失败，通道可能已关闭");
                return Err(format!("{:?}", e));
            }

            match rx.await {
                Ok(Ok(())) => {
                    return Ok(());
                }
                Ok(Err(game_error)) => {
                    tracing::warn!(actor_id = %task.actor_id, error = %game_error, "AI 动作被游戏引擎拒绝");
                    if game_error == "Game is over" {
                        tracing::error!("游戏已结束，放弃 AI 动作");
                        return Err("Game is over".into());
                    }
                    if game_error.contains("Not your turn") || game_error.contains("Dead players cannot act") || game_error.contains("游戏还未开始") {
                        tracing::warn!("引擎状态已变化，不再重试: {}", game_error);
                        return Err(game_error);
                    }
                    if task.retries < max_retries {
                        task.retries += 1;
                        if let Some(arr) = messages_json.as_array_mut() {
                            arr.push(serde_json::json!({
                                "role": "user",
                                "content": format!("你上一次的动作执行失败被系统拒绝，原因是：{}。请认真检查你的 action_type 和 target 是否合法，修正格式后重试。", game_error)
                            }));
                        }
                        continue;
                    } else {
                        tracing::error!("达到最大重试次数，放弃 AI 动作");
                        let command = RoomCommand::PlayerAction {
                            actor_id: task.actor_id.clone(),
                            action: serde_json::json!({"content": "[反复违规，操作失败]"}),
                            feedback_tx: None,
                        };
                        let _ = task.reply_tx.send(command).await;
                        return Err("Max retries reached".into());
                    }
                }
                Err(_) => {
                    tracing::error!("等待游戏引擎反馈通道断开");
                    return Err("Feedback channel closed".into());
                }
            }
        }
    }
}
