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
) -> Result<Value, AiClientError> {
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

    info!(
        elapsed_ms = %elapsed_ms,
        has_tool_calls = message.get("tool_calls").is_some(),
        "AI 响应解析成功"
    );

    Ok(message)
}
