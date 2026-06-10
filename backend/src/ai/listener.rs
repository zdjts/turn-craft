
use reqwest::Client;
use tracing::error;

use crate::{
    ai::client::request_speech,
    network::room::{AiTask, RoomCommand},
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
    async fn process_task(http: Client, task: AiTask) -> Result<(), String> {
        let config = task.ai_config;

        let messages_json = build_messages(&config, task.snapshot);

        let ai_reply = match request_speech(&http, &config, messages_json).await {
            Ok(reply) => reply,
            Err(e) => {
                tracing::error!(error = ?e, "请求 AI 接口发生错误");
                let command = RoomCommand::PlayerAction {
                    actor_id: task.actor_id,
                    action: serde_json::json!({"content": "[思考超时，未能发言]"}),
                };
                let _ = task.reply_tx.send(command).await;
                return Err(format!("{:?}", e));
            }
        };

        let command = RoomCommand::PlayerAction {
            actor_id: task.actor_id,
            action: serde_json::json!({"content": ai_reply}),
        };

        if let Err(e) = task.reply_tx.send(command).await {
            tracing::error!(error = ?e, "向房间回传 AI 动作失败，通道可能已关闭");
            return Err(format!("{:?}", e));
        }

        Ok(())
    }
}
