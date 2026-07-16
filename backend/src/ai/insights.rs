use reqwest::Client;
use serde_json::{Value, json};
use tokio::time::{timeout, Duration};

use super::env::{AiConfig, AiStyle};

const TIMEOUT_SECS: u64 = 10;

/// LLM 生成的 AI 策略分析
pub async fn generate_insights(
    http: &Client,
    ai_config: &AiConfig,
    engine_state: &Value,
) -> Vec<Value> {
    let actors = extract_ai_actors(engine_state);
    if actors.is_empty() {
        return Vec::new();
    }

    // 从引擎状态提取历史事件日志用于 LLM prompt
    let history = engine_state.get("history")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    if history.is_empty() {
        return Vec::new();
    }

    let mut insights = Vec::new();
    for actor in &actors {
        let actor_id = actor.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let role = actor.get("role").and_then(|v| v.as_str())
            .or_else(|| actor.get("position").and_then(|v| v.as_str()))
            .unwrap_or("").to_string();
        let style = actor.get("style").and_then(|v| v.as_str()).unwrap_or("default").to_string();

        let analysis = analyze_actor(http, ai_config, &actor_id, &role, &style, &history).await;
        insights.push(analysis);
    }

    insights
}

/// 对单个 AI 角色进行 LLM 策略分析
async fn analyze_actor(
    http: &Client,
    ai_config: &AiConfig,
    actor_id: &str,
    role: &str,
    style: &str,
    history: &[Value],
) -> Value {
    // 过滤出该 AI 的行动 + 轮次信息
    let actor_actions: Vec<Value> = history.iter().filter_map(|evt| {
        let eid = evt.get("actor_id").and_then(|v| v.as_str())?;
        if eid != actor_id { return None; }
        let content = evt.get("content").and_then(|v| v.as_str())?;
        if content.is_empty() { return None; }
        Some(json!({
            "round": evt.get("round").and_then(|v| v.as_u64()).unwrap_or(0),
            "content": content
        }))
    }).collect();

    if actor_actions.is_empty() {
        return fallback_insight(actor_id, role, style);
    }

    // 构建 LLM prompt
    let prompt = format!(
        "你是一个策略分析师。请分析以下 AI 玩家在回合制对局中的表现。\n\
        角色: {role}\n风格: {style}\n\n\
        该 AI 的行动历史:\n{actions}\n\n\
        请输出 JSON，格式严格如下：\n\
        {{\n\
          \"overall_assessment\": \"1-2句话总结该AI的整体表现\",\n\
          \"key_actions\": [\n\
            {{\"round\": 数字, \"action\": \"行动描述\", \"reason\": \"为什么这是关键\", \"impact\": \"high|medium\"}}\n\
          ],\n\
          \"highlights\": [\"具体高光时刻描述\"],\n\
          \"mistakes\": [\"具体失误描述\"]\n\
        }}\n\
        如果该AI表现完美没有失误，mistakes可以为空数组。如果平淡无亮点，highlights可以为空数组。\
        只输出JSON，不要多余的说明。",
        role = role,
        style = style,
        actions = serde_json::to_string_pretty(&actor_actions).unwrap_or_default()
    );

    let config = AiConfig {
        api_key: ai_config.api_key.clone(),
        base_url: ai_config.base_url.clone(),
        model: ai_config.model.clone(),
        max_tokens: 1024,
        prompt: prompt.clone(),
        style: AiStyle::Rational,
    };

    let body = json!({
        "model": config.model,
        "messages": [
            {"role": "system", "content": "你是一个策略分析师。输出严格JSON，不要多余文字。"},
            {"role": "user", "content": prompt}
        ],
        "temperature": 0.5,
        "max_tokens": 1024,
    });

    let result = timeout(Duration::from_secs(TIMEOUT_SECS), async {
        let resp = http
            .post(format!("{}/chat/completions", config.base_url.trim_end_matches('/')))
            .header("Authorization", format!("Bearer {}", config.api_key))
            .json(&body)
            .send()
            .await
            .ok()?;

        let data: Value = resp.json().await.ok()?;
        let text = data["choices"][0]["message"]["content"].as_str()?.to_string();
        // 尝试解析为 JSON
        serde_json::from_str::<Value>(&text).ok()
    }).await;

    match result {
        Ok(Some(parsed)) => {
            json!({
                "actor_id": actor_id,
                "role": role,
                "style": style,
                "overall_assessment": parsed.get("overall_assessment").and_then(|v| v.as_str()).unwrap_or(""),
                "key_actions": parsed.get("key_actions").and_then(|v| v.as_array()).cloned().unwrap_or_default(),
                "highlights": parsed.get("highlights").and_then(|v| v.as_array()).cloned().unwrap_or_default(),
                "mistakes": parsed.get("mistakes").and_then(|v| v.as_array()).cloned().unwrap_or_default(),
            })
        }
        _ => fallback_insight(actor_id, role, style),
    }
}

/// LLM 不可用时的回退——提取行动列表但不做深度分析
pub fn fallback_insight(actor_id: &str, role: &str, style: &str) -> Value {
    json!({
        "actor_id": actor_id,
        "role": role,
        "style": style,
        "overall_assessment": "",
        "key_actions": [],
        "highlights": [],
        "mistakes": []
    })
}

/// 提取对局中的 AI 参与者
pub fn extract_ai_actors(engine_state: &Value) -> Vec<Value> {
    let actors = engine_state.get("actors")
        .or_else(|| engine_state.get("players"))
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    actors.into_iter().filter(|a| {
        a.get("kind").and_then(|v| v.as_str())
            .map(|k| k.to_lowercase() == "ai")
            .unwrap_or(false)
    }).collect()
}
