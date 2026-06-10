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

/// 请求 AI 发言：发送消息到 LLM API 并返回响应
pub async fn request_speech(
    http: &Client,
    config: &AiConfig,
    messages: String,
) -> Result<String, AiClientError> {
    let messages_json: Value = serde_json::from_str(&messages).map_err(|e| {
        error!(error = %e, "入参 messages 字符串解析为 JSON 失败");
        AiClientError::Parse(format!("入参格式错误: {e}"))
    })?;

    let body = serde_json::json!({
        "model": config.model,
        "messages": messages_json, // ✨ 【核心修复】：传入解析后的 JSON 对象，不再是带有转义的 String
        "temperature": 0.7,
        "max_tokens": config.max_tokens,
    });
    // let body = serde_json::json!({
    //     "model": config.model,
    //     "messages": messages,
    //     "temperature": 0.7,
    //     "max_tokens": config.max_tokens,
    // });
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

    debug!(status=%status,elapsed_ms = %elapsed_ms, "收到 Ai 相应");
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
    // 在声明时指定类型
    let response: serde_json::Value = raw_response.json().await.map_err(|e| {
        error!(error = %e, "AI 响应 JSON 解析失败");
        e
    })?;
    let content = response
        .get("choices")
        .and_then(|c| c.get(0))
        .and_then(|c| c.get("message"))
        .and_then(|m| {
            // 优先取 content，为空则回退到 reasoning_content（推理模型）
            let c = m
                .get("content")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim();
            if !c.is_empty() {
                return Some(c.to_string());
            }
            m.get("reasoning_content")
                .and_then(|v| v.as_str())
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
        })
        .ok_or_else(|| {
            let raw = response.to_string();
            error!(body = %raw, "响应格式异常或 content 为空");
            AiClientError::Parse(format!("响应格式异常或 content 为空: {raw}"))
        })?;
    info!(
        elapsed_ms = %elapsed_ms,
        content_len = content.len(),
        "AI 响应解析成功"
    );
    Ok(content)
}
