use std::time::Instant;

use reqwest::Client;
use serde_json::Value;
use tracing::{debug, error, info};

use super::env::AiConfig;

/// AI 客户端错误类型
#[derive(Debug)]
pub enum AiClientError {
    Http(reqwest::Error),
    Parse(String),
}
impl From<reqwest::Error> for AiClientError {
    fn from(e: reqwest::Error) -> Self {
        AiClientError::Http(e)
    }
}

/// 请求 AI 发言：发送消息到 LLM API 并返回完整响应
/// 支持可选的 tools（用于 function calling / tool use）
pub async fn request_speech(
    http: &Client,
    config: &AiConfig,
    messages: String,
    tools: Option<&Value>,
) -> Result<(Value, Option<platform_core::traits::TokenUsage>), AiClientError> {
    let messages_json: Value = serde_json::from_str(&messages).map_err(|e| {
        error!(error = %e, "入参 messages 字符串解析为 JSON 失败");
        AiClientError::Parse(format!("入参格式错误: {e}"))
    })?;

    let mut body = serde_json::json!({
        "model": config.model,
        "messages": messages_json,
        "temperature": 0.7,
        "max_tokens": config.max_tokens,
    });

    // 如果传入了 tools，添加到请求体
    if let Some(tools_value) = tools {
        body["tools"] = tools_value.clone();
        body["tool_choice"] = serde_json::json!("required");
        tracing::info!(tools = %tools_value, "发送的 tools 定义");
    }

    let start = Instant::now();

    let raw_response = http
        .post(format!("{}/chat/completions", config.base_url))
        .bearer_auth(&config.api_key)
        .json(&body)
        .send()
        .await
        .map_err(|e| {
            error!(error=%e, "AI Http请求失败");
            e
        })?;
    let status = raw_response.status();
    let elapsed_ms = start.elapsed().as_micros();

    debug!(status=%status, elapsed_ms = %elapsed_ms, "收到 Ai 响应");
    if !status.is_success() {
        let body_text = raw_response.text().await.unwrap_or_default();
        error!(
            status = %status,
            body = %body_text,
            elapsed_ms = elapsed_ms,
            "AI 接口返回非 2xx 状态码"
        );
        return Err(AiClientError::Parse(format!("HTTP {status}: {body_text}")));
    }

    let response: Value = raw_response.json().await.map_err(|e| {
        error!(error = %e, "AI 响应 JSON 解析失败");
        e
    })?;

    // 提取 choices[0].message 作为完整响应
    let message = response
        .get("choices")
        .and_then(|c| c.get(0))
        .and_then(|c| c.get("message"))
        .ok_or_else(|| {
            let raw = response.to_string();
            error!(body = %raw, "响应格式异常");
            AiClientError::Parse(format!("响应格式异常: {raw}"))
        })?
        .clone();

    let usage_val = response.get("usage");
    let token_usage = usage_val.map(|u| {
        let prompt_tokens = u.get("prompt_tokens").and_then(|v| v.as_u64()).unwrap_or(0);
        let completion_tokens = u.get("completion_tokens").and_then(|v| v.as_u64()).unwrap_or(0);
        
        let cached_tokens = u.get("prompt_tokens_details")
            .and_then(|d| d.get("cached_tokens"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        platform_core::traits::TokenUsage {
            prompt_tokens,
            completion_tokens,
            cached_tokens,
        }
    });

    if let Some(usage) = &token_usage {
        info!(
            elapsed_ms = %elapsed_ms,
            has_tool_calls = message.get("tool_calls").is_some(),
            prompt_tokens = usage.prompt_tokens,
            completion_tokens = usage.completion_tokens,
            cached_tokens = usage.cached_tokens,
            "AI 响应解析成功"
        );
    } else {
        info!(
            elapsed_ms = %elapsed_ms,
            has_tool_calls = message.get("tool_calls").is_some(),
            "AI 响应解析成功（未获取到 Token 信息）"
        );
    }

    Ok((message, token_usage))
}

/// 流式输出的增量片段
#[derive(Debug, Clone)]
pub enum StreamDelta {
    /// 普通 content token
    Content(String),
    /// tool_calls 参数增量片段
    ToolCallArgDelta(String),
    /// 流结束
    Done,
}

/// 流式请求 AI 发言：通过 SSE 逐 token 读取，同时将 delta 实时发送到 delta_tx
/// 最终返回拼接后的完整响应（与非流式版本格式一致）
pub async fn request_speech_stream(
    http: &Client,
    config: &AiConfig,
    messages: String,
    tools: Option<&Value>,
    delta_tx: tokio::sync::mpsc::Sender<StreamDelta>,
) -> Result<(Value, Option<platform_core::traits::TokenUsage>), AiClientError> {
    let messages_json: Value = serde_json::from_str(&messages).map_err(|e| {
        error!(error = %e, "入参 messages 字符串解析为 JSON 失败");
        AiClientError::Parse(format!("入参格式错误: {e}"))
    })?;

    let mut body = serde_json::json!({
        "model": config.model,
        "messages": messages_json,
        "temperature": 0.7,
        "max_tokens": config.max_tokens,
        "stream": true,
        "stream_options": { "include_usage": true },
    });

    if let Some(tools_value) = tools {
        body["tools"] = tools_value.clone();
        body["tool_choice"] = serde_json::json!("required");
    }

    let start = std::time::Instant::now();

    let raw_response = http
        .post(format!("{}/chat/completions", config.base_url))
        .bearer_auth(&config.api_key)
        .json(&body)
        .send()
        .await
        .map_err(|e| {
            error!(error=%e, "AI Http请求失败 (stream)");
            e
        })?;
    let status = raw_response.status();

    if !status.is_success() {
        let body_text = raw_response.text().await.unwrap_or_default();
        error!(status = %status, body = %body_text, "AI 接口返回非 2xx 状态码 (stream)");
        return Err(AiClientError::Parse(format!("HTTP {status}: {body_text}")));
    }

    // 逐行读取 SSE 流
    use futures_util::StreamExt;
    let mut byte_stream = raw_response.bytes_stream();
    let mut buffer = String::new();
    let mut content_acc = String::new();
    let mut tool_calls_acc: Vec<Value> = Vec::new();
    let mut role_acc: Option<String> = None;
    let mut token_usage: Option<platform_core::traits::TokenUsage> = None;

    while let Some(chunk_result) = byte_stream.next().await {
        let chunk_bytes = chunk_result.map_err(|e| {
            error!(error = %e, "SSE 流读取失败");
            AiClientError::Http(e)
        })?;
        buffer.push_str(&String::from_utf8_lossy(&chunk_bytes));

        // 按行处理 SSE 事件
        while let Some(line_end) = buffer.find('\n') {
            let line = buffer[..line_end].trim_end_matches('\r').to_string();
            buffer = buffer[line_end + 1..].to_string();

            if line.is_empty() || line.starts_with(':') {
                continue;
            }

            if let Some(data) = line.strip_prefix("data: ") {
                let data = data.trim();
                if data == "[DONE]" {
                    let _ = delta_tx.send(StreamDelta::Done).await;
                    continue;
                }

                if let Ok(event) = serde_json::from_str::<Value>(data) {
                    // 提取 usage（最后一个 chunk 通常包含）
                    if let Some(u) = event.get("usage") {
                        if u.is_object() && u.get("prompt_tokens").is_some() {
                            let prompt_tokens = u.get("prompt_tokens").and_then(|v| v.as_u64()).unwrap_or(0);
                            let completion_tokens = u.get("completion_tokens").and_then(|v| v.as_u64()).unwrap_or(0);
                            let cached_tokens = u.get("prompt_tokens_details")
                                .and_then(|d| d.get("cached_tokens"))
                                .and_then(|v| v.as_u64())
                                .unwrap_or(0);
                            token_usage = Some(platform_core::traits::TokenUsage {
                                prompt_tokens,
                                completion_tokens,
                                cached_tokens,
                            });
                        }
                    }

                    // 提取 delta
                    if let Some(choice) = event.get("choices").and_then(|c| c.get(0)) {
                        if let Some(delta) = choice.get("delta") {
                            // 记录 role
                            if let Some(r) = delta.get("role").and_then(|r| r.as_str()) {
                                role_acc = Some(r.to_string());
                            }

                            // content delta
                            if let Some(content) = delta.get("content").and_then(|c| c.as_str()) {
                                if !content.is_empty() {
                                    content_acc.push_str(content);
                                    let _ = delta_tx.send(StreamDelta::Content(content.to_string())).await;
                                }
                            }

                            // tool_calls delta
                            if let Some(tcs) = delta.get("tool_calls").and_then(|t| t.as_array()) {
                                for tc in tcs {
                                    let idx = tc.get("index").and_then(|i| i.as_u64()).unwrap_or(0) as usize;

                                    // 确保 tool_calls_acc 足够大
                                    while tool_calls_acc.len() <= idx {
                                        tool_calls_acc.push(serde_json::json!({
                                            "id": "",
                                            "type": "function",
                                            "function": { "name": "", "arguments": "" }
                                        }));
                                    }

                                    // 合并 id
                                    if let Some(id) = tc.get("id").and_then(|i| i.as_str()) {
                                        tool_calls_acc[idx]["id"] = Value::String(id.to_string());
                                    }

                                    // 合并 function name
                                    if let Some(func) = tc.get("function") {
                                        if let Some(name) = func.get("name").and_then(|n| n.as_str()) {
                                            tool_calls_acc[idx]["function"]["name"] = Value::String(name.to_string());
                                        }
                                        // 合并 arguments 增量
                                        if let Some(args) = func.get("arguments").and_then(|a| a.as_str()) {
                                            if !args.is_empty() {
                                                let existing = tool_calls_acc[idx]["function"]["arguments"]
                                                    .as_str()
                                                    .unwrap_or("")
                                                    .to_string();
                                                tool_calls_acc[idx]["function"]["arguments"] = Value::String(format!("{}{}", existing, args));
                                                let _ = delta_tx.send(StreamDelta::ToolCallArgDelta(args.to_string())).await;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                } else {
                    debug!(raw = %data, "SSE 行 JSON 解析失败，跳过");
                }
            }
        }
    }

    let elapsed_ms = start.elapsed().as_micros();

    // 构造与非流式版本格式一致的 message Value
    let mut message = serde_json::json!({
        "role": role_acc.unwrap_or_else(|| "assistant".to_string()),
    });
    if !content_acc.is_empty() {
        message["content"] = Value::String(content_acc);
    } else {
        message["content"] = Value::Null;
    }
    if !tool_calls_acc.is_empty() {
        message["tool_calls"] = Value::Array(tool_calls_acc);
    }

    if let Some(usage) = &token_usage {
        info!(
            elapsed_ms = %elapsed_ms,
            has_tool_calls = message.get("tool_calls").is_some(),
            prompt_tokens = usage.prompt_tokens,
            completion_tokens = usage.completion_tokens,
            cached_tokens = usage.cached_tokens,
            "AI 流式响应完成"
        );
    } else {
        info!(
            elapsed_ms = %elapsed_ms,
            has_tool_calls = message.get("tool_calls").is_some(),
            "AI 流式响应完成（未获取到 Token 信息）"
        );
    }

    Ok((message, token_usage))
}
