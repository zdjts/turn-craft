use dioxus::prelude::*;
use serde_json::Value;
use tracing::{info, warn};

pub fn submit_litigation(
    mut draft: Signal<String>,
    on_action: Callback<Value>,
    source: &str,
) {
    let content = draft.read().trim().to_string();
    if content.is_empty() {
        warn!(target: "lincoln::action", "用户尝试发送空发言，已忽略");
        return;
    }
    info!(target: "lincoln::action", content_len = content.len(), source, "裁判发射发言");
    on_action.call(serde_json::json!({"content": content}));
    draft.write().clear();
}
