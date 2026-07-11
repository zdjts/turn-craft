use reqwest::Client;
use tracing::error;

use crate::{
    ai::client::{StreamDelta, request_speech, request_speech_stream},
    room::actor::SideEffect,
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

            // 创建流式 delta 通道
            let (delta_tx, mut delta_rx) = tokio::sync::mpsc::channel::<StreamDelta>(256);
            let effect_tx = task.effect_tx.clone();
            let actor_id_for_forwarder = task.actor_id.clone();

            // 启动转发任务: StreamDelta → SideEffect::StreamChunk
            let forwarder = tokio::spawn(async move {
                while let Some(delta) = delta_rx.recv().await {
                    match delta {
                        StreamDelta::Content(text) => {
                            let _ = effect_tx
                                .send(SideEffect::StreamChunk {
                                    actor_id: actor_id_for_forwarder.clone(),
                                    content: text,
                                    is_done: false,
                                })
                                .await;
                        }
                        StreamDelta::ToolCallArgDelta(text) => {
                            let _ = effect_tx
                                .send(SideEffect::StreamChunk {
                                    actor_id: actor_id_for_forwarder.clone(),
                                    content: text,
                                    is_done: false,
                                })
                                .await;
                        }
                        StreamDelta::Done => {
                            let _ = effect_tx
                                .send(SideEffect::StreamChunk {
                                    actor_id: actor_id_for_forwarder.clone(),
                                    content: String::new(),
                                    is_done: true,
                                })
                                .await;
                        }
                    }
                }
            });

            let ai_response = match request_speech_stream(
                &http,
                &config,
                messages_json.to_string(),
                task.tools.as_ref(),
                delta_tx,
            )
            .await
            {
                Ok((mut response, token_usage)) => {
                    // 等待转发任务结束
                    let _ = forwarder.await;

                    tracing::info!(
                        actor_id = %task.actor_id,
                        response = %response,
                        "<<< AI 返回的完整回复 (stream)"
                    );
                    if let Some(usage) = token_usage {
                        if let Some(obj) = response.as_object_mut() {
                            obj.insert(
                                "_token_usage".to_string(),
                                serde_json::to_value(usage).unwrap_or_default(),
                            );
                        }
                    }
                    response
                }
                Err(e) => {
                    // 确保转发任务结束
                    forwarder.abort();
                    // 发送 stream_done 以便前端清理
                    let _ = task
                        .effect_tx
                        .send(SideEffect::StreamChunk {
                            actor_id: task.actor_id.clone(),
                            content: String::new(),
                            is_done: true,
                        })
                        .await;

                    tracing::error!(actor_id = %task.actor_id, error = ?e, "请求 AI 接口发生错误 (stream)");
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
                    if game_error.contains("Not your turn")
                        || game_error.contains("Dead players cannot act")
                        || game_error.contains("游戏还未开始")
                    {
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
