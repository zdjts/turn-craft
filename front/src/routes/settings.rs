use dioxus::prelude::*;
use std::collections::HashMap;
use tracing::{error, info};

use crate::api::{AiConfigData, get_ai_configs, update_ai_config};

#[component]
pub fn Settings(room_id: String, actor_id: String) -> Element {
    let mut configs = use_signal(|| HashMap::<String, AiConfigData>::new());
    let mut loading = use_signal(|| true);
    let mut save_msg = use_signal(|| Option::<String>::None);

    let room_id_clone = room_id.clone();
    use_effect(move || {
        let rid = room_id_clone.clone();
        spawn(async move {
            info!(target: "settings", room_id = %rid, "正在加载 AI 配置...");
            match get_ai_configs(&rid).await {
                Ok(cfg) => {
                    info!(target: "settings", count = cfg.len(), "AI 配置加载成功");
                    configs.set(cfg);
                    loading.set(false);
                }
                Err(e) => {
                    error!(target: "settings", error = %e, "加载 AI 配置失败");
                    loading.set(false);
                }
            }
        });
    });

    rsx! {
        div { class: "settings-shell",
            div { class: "settings-container",
                div { class: "settings-header",
                    {
                        let rid = room_id.clone();
                        let aid = actor_id.clone();
                        rsx! {
                            button {
                                class: "back-btn",
                                onclick: move |_| {
                                    use_navigator().push(format!("/game/{}/{}", rid, aid));
                                },
                                "← 返回对局"
                            }
                        }
                    }
                    h1 { class: "settings-title", "⚙️ AI 配置" }
                    p { class: "settings-subtitle",
                        "房间: {room_id}"
                    }
                }

                if *loading.read() {
                    div { class: "settings-loading",
                        div { class: "sync-spinner" }
                        span { "加载配置中..." }
                    }
                } else if configs.read().is_empty() {
                    div { class: "settings-empty",
                        p { "该房间没有 AI 角色配置" }
                    }
                } else {
                    div { class: "settings-cards",
                        for (aid, _cfg) in configs.read().iter() {
                            AiConfigCard {
                                key: "{aid}",
                                actor_id: aid.clone(),
                                room_id: room_id.clone(),
                                initial: _cfg.clone(),
                                on_saved: move |msg: String| {
                                    save_msg.set(Some(msg));
                                },
                            }
                        }
                    }
                }

                if let Some(ref msg) = *save_msg.read() {
                    div { class: "save-toast",
                        "{msg}"
                    }
                }
            }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
struct AiConfigCardProps {
    actor_id: String,
    room_id: String,
    initial: AiConfigData,
    on_saved: Callback<String>,
}

#[component]
fn AiConfigCard(props: AiConfigCardProps) -> Element {
    let mut api_key = use_signal(|| props.initial.api_key.clone());
    let mut base_url = use_signal(|| props.initial.base_url.clone());
    let mut model = use_signal(|| props.initial.model.clone());
    let mut max_tokens = use_signal(|| props.initial.max_tokens.to_string());
    let mut prompt = use_signal(|| props.initial.prompt.clone());
    let mut saving = use_signal(|| false);

    let actor_label = props.actor_id.clone();
    let role_emoji = if actor_label.contains("judge") {
        "👑"
    } else if actor_label.contains("pro") {
        "🟢"
    } else if actor_label.contains("con") {
        "🔴"
    } else {
        "🤖"
    };

    rsx! {
        div { class: "ai-config-card",
            div { class: "ai-config-header",
                span { class: "ai-config-emoji", "{role_emoji}" }
                span { class: "ai-config-name", "{props.actor_id}" }
            }

            div { class: "ai-config-fields",
                div { class: "ai-field",
                    label { "Base URL" }
                    input {
                        value: "{base_url}",
                        placeholder: "https://api.deepseek.com/v1",
                        oninput: move |e| base_url.set(e.value()),
                    }
                }
                div { class: "ai-field",
                    label { "Model" }
                    input {
                        value: "{model}",
                        placeholder: "deepseek-chat",
                        oninput: move |e| model.set(e.value()),
                    }
                }
                div { class: "ai-field",
                    label { "API Key" }
                    input {
                        r#type: "password",
                        value: "{api_key}",
                        placeholder: "sk-...",
                        oninput: move |e| api_key.set(e.value()),
                    }
                }
                div { class: "ai-field",
                    label { "Max Tokens" }
                    input {
                        value: "{max_tokens}",
                        placeholder: "200",
                        oninput: move |e| max_tokens.set(e.value()),
                    }
                }
                div { class: "ai-field full",
                    label { "System Prompt" }
                    textarea {
                        value: "{prompt}",
                        placeholder: "AI 的系统提示词...",
                        oninput: move |e| prompt.set(e.value()),
                    }
                }
            }

            button {
                class: "ai-config-save",
                disabled: *saving.read(),
                onclick: {
                    let rid = props.room_id.clone();
                    let aid = props.actor_id.clone();
                    let on_saved = props.on_saved;
                    move |_| {
                        let mt = max_tokens.read().trim().parse::<u32>().unwrap_or(200);
                        let cfg = AiConfigData {
                            api_key: api_key.read().clone(),
                            base_url: base_url.read().clone(),
                            model: model.read().clone(),
                            max_tokens: mt,
                            prompt: prompt.read().clone(),
                        };
                        let rid = rid.clone();
                        let aid = aid.clone();
                        saving.set(true);
                        spawn(async move {
                            match update_ai_config(&rid, &aid, &cfg).await {
                                Ok(()) => {
                                    on_saved.call(format!("{} 配置已保存", aid));
                                }
                                Err(e) => {
                                    on_saved.call(format!("保存失败: {}", e));
                                }
                            }
                            saving.set(false);
                        });
                    }
                },
                if *saving.read() { "保存中..." } else { "保存" }
            }
        }
    }
}
