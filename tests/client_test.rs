use std::time::Instant;

use reqwest::Client;
use tracing::{debug, error, info};

use crate::ai::env::build_messages;

use super::env::AiConfig;

pub enum AiClientError {
    Http(reqwest::Error),
    Parse(String),
}

impl From<reqwest::Error> for AiClientError {
    fn from(e: reqwest::Error) -> Self {
        AiClientError::Http(e)
    }
}

pub async fn request_speech(
    http: &Client,
    config: &AiConfig,
    messages: String,
) -> Result<String, AiClientError> {
    let message_count = messages.len();
    let body = serde_json::json!({
        "model": config.model,
        "messages": messages,
        "temperature": 0.7,
        "max_tokens": config.max_tokens,
    });
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
        .and_then(|m| m.get("content"))
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string())
        .ok_or_else(|| {
            let raw = response.to_string();
            error!(body = %raw, "响应格式异常，无法提取 content");
            AiClientError::Parse(format!("响应格式异常: {raw}"))
        })?;
    info!(
        elapsed_ms = %elapsed_ms,
        content_len = content.len(),
        "AI 响应解析成功"
    );
    Ok(content)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::env::AiConfig;
    use std::io::{BufRead, BufReader, Read, Write};
    use std::net::TcpListener;
    use std::thread;
    use std::time::Duration;

    async fn start_mock_server(response_body: &str) -> u16 {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind mock server");
        let port = listener.local_addr().unwrap().port();
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
            response_body.len(),
            response_body
        );
        thread::spawn(move || {
            for stream in listener.incoming() {
                if let Ok(mut stream) = stream {
                    let mut reader = BufReader::new(&stream);
                    // read request line
                    let mut request_line = String::new();
                    if reader.read_line(&mut request_line).is_err() {
                        break;
                    }
                    // read headers
                    let mut headers = Vec::new();
                    loop {
                        let mut line = String::new();
                        if reader.read_line(&mut line).is_err() {
                            break;
                        }
                        if line == "\r\n" || line.is_empty() {
                            break;
                        }
                        headers.push(line);
                    }
                    // determine content-length
                    let mut content_length = 0usize;
                    for h in &headers {
                        if h.to_lowercase().starts_with("content-length:") {
                            if let Some(val) = h.split(':').nth(1) {
                                content_length = val.trim().parse().unwrap_or(0);
                            }
                        }
                    }
                    if content_length > 0 {
                        let mut body = vec![0u8; content_length];
                        let _ = stream.read_exact(&mut body);
                    }
                    // send response
                    let _ = stream.write_all(response.as_bytes());
                    let _ = stream.flush();
                    break; // only handle one connection
                }
            }
        });
        // give the server a moment to start
        tokio::time::sleep(Duration::from_millis(50)).await;
        port
    }

    #[tokio::test]
    async fn test_request_speech_success() {
        let expected_content = "Hello from AI";
        let response_json = serde_json::json!({
            "choices": [{
                "message": {
                    "content": expected_content
                }
            }]
        })
        .to_string();
        let port = start_mock_server(&response_json).await;
        let config = AiConfig {
            api_key: "test-key".to_string(),
            base_url: format!("http://127.0.0.1:{}", port),
            model: "test-model".to_string(),
            max_tokens: 100,
        };
        let http = reqwest::Client::new();
        let messages = "[]".to_string();
        let result = request_speech(&http, &config, messages).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), expected_content);
    }

    #[tokio::test]
    async fn test_request_speech_invalid_url() {
        let config = AiConfig {
            api_key: "test-key".to_string(),
            base_url: "http://127.0.0.1:1".to_string(),
            model: "test-model".to_string(),
            max_tokens: 100,
        };
        let http = reqwest::Client::new();
        let messages = "[]".to_string();
        let result = request_speech(&http, &config, messages).await;
        assert!(result.is_err());
    }
}
