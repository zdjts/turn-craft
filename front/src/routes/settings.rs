use crate::api::{get_ai_configs, update_ai_config, AiConfigData};
use crate::routes::layout::use_toast;
use dioxus::prelude::*;

#[component]
pub fn Settings(room_id: String, actor_id: String) -> Element {
    let toast = use_toast();
    let nav = use_navigator();

    let mut api_key = use_signal(|| String::new());
    let mut base_url = use_signal(|| String::new());
    let mut model = use_signal(|| String::new());
    let mut max_tokens = use_signal(|| 2048_u32);
    let mut prompt = use_signal(|| String::new());

    let mut loading = use_signal(|| true);
    let mut saving = use_signal(|| false);

    let rid_for_save = room_id.clone();
    let aid_for_save = actor_id.clone();
    let rid_for_cancel = room_id.clone();
    let aid_for_cancel = actor_id.clone();

    let mut prev_actor = use_signal(|| actor_id.clone());
    let mut prev_room = use_signal(|| room_id.clone());

    // Update signals if props changed (e.g. from tab navigation)
    if *prev_actor.peek() != actor_id || *prev_room.peek() != room_id {
        prev_actor.set(actor_id.clone());
        prev_room.set(room_id.clone());
    }

    let mut all_ai_actors = use_signal(|| Vec::<String>::new());

    // Load AI configurations reactively based on tracking signals
    use_effect(move || {
        let rid = prev_room.read().clone();
        let aid = prev_actor.read().clone();
        spawn(async move {
            loading.set(true);
            match get_ai_configs(&rid).await {
                Ok(configs) => {
                    let mut actors: Vec<String> = configs.keys().cloned().collect();
                    actors.sort();
                    all_ai_actors.set(actors);

                    if let Some(cfg) = configs.get(&aid) {
                        api_key.set(cfg.api_key.clone());
                        base_url.set(cfg.base_url.clone());
                        model.set(cfg.model.clone());
                        max_tokens.set(cfg.max_tokens);
                        prompt.set(cfg.prompt.clone());
                    } else {
                        // Reset if not found
                        api_key.set(String::new());
                        base_url.set(String::new());
                        model.set(String::new());
                        max_tokens.set(2048);
                        prompt.set(String::new());
                        toast.show(
                            format!("未找到角色 {aid} 的 AI 配置，已初始化默认配置。"),
                            crate::routes::layout::ToastType::Info,
                        );
                    }
                }
                Err(e) => {
                    toast.show(
                        format!("加载 AI 配置失败: {e}"),
                        crate::routes::layout::ToastType::Error,
                    );
                }
            }
            loading.set(false);
        });
    });

    let handle_save = move |_| {
        if *saving.read() {
            return;
        }

        saving.set(true);
        let rid = rid_for_save.clone();
        let aid = aid_for_save.clone();
        let config = AiConfigData {
            api_key: api_key.read().clone(),
            base_url: base_url.read().clone(),
            model: model.read().clone(),
            max_tokens: *max_tokens.read(),
            prompt: prompt.read().clone(),
        };

        spawn(async move {
            match update_ai_config(&rid, &aid, &config).await {
                Ok(_) => {
                    toast.show(
                        "AI 参数已成功更新，设置将在下一回合生效。".to_string(),
                        crate::routes::layout::ToastType::Success,
                    );
                    nav.go_back();
                }
                Err(e) => {
                    toast.show(
                        format!("保存 AI 配置失败: {e}"),
                        crate::routes::layout::ToastType::Error,
                    );
                }
            }
            saving.set(false);
        });
    };

    rsx! {
        div { class: "settings-container animate-fade-in",
            div { class: "page-header",
                h1 { "⚙️ 配置 AI 智能参数" }
                p { "为当前房间的 AI 助手配置专属的大语言模型接入端点和个性化 Prompt 提示词。" }
            }

            if *loading.read() {
                div { class: "loading-canvas glass-panel",
                    span { class: "spinner" }
                    p { "正在读取 AI 配置项，请稍候..." }
                }
            } else {
                div { class: "settings-layout glass-panel",
                    // Switch AI tabs if there are multiple AI players in the room
                    if all_ai_actors.read().len() > 1 {
                        div { class: "ai-actor-tabs",
                            for actor in all_ai_actors.read().iter() {
                                {
                                    let act_id = actor.clone();
                                    let is_current = act_id == actor_id;
                                    let rid = room_id.clone();
                                    let nav = nav.clone();
                                    let display_name = match act_id.as_str() {
                                        "ai_pro" => "正方 (Pro)".to_string(),
                                        "ai_con" => "反方 (Con)".to_string(),
                                        "ai_judge" => "裁判 (Judge)".to_string(),
                                        other => {
                                            if other.starts_with("ai_") {
                                                format!("AI {}", &other[3..])
                                            } else {
                                                other.to_string()
                                            }
                                        }
                                    };
                                    rsx! {
                                        button {
                                            key: "{act_id}",
                                            class: if is_current { "ai-tab-btn active" } else { "ai-tab-btn" },
                                            onclick: move |_| {
                                                nav.push(super::Route::Settings { room_id: rid.clone(), actor_id: act_id.clone() });
                                            },
                                            "{display_name}"
                                        }
                                    }
                                }
                            }
                        }
                    }

                    div { class: "settings-section-title",
                        "🤖 正在配置: {actor_id}"
                    }

                    div { class: "settings-form",
                        div { class: "form-grid-two-cols",
                            div { class: "form-field",
                                label { "API 基址 (Base URL)" }
                                input {
                                    r#type: "text",
                                    placeholder: "https://api.openai.com/v1",
                                    value: "{base_url}",
                                    oninput: move |e| base_url.set(e.value()),
                                }
                            }

                            div { class: "form-field",
                                label { "模型名称 (Model)" }
                                input {
                                    r#type: "text",
                                    placeholder: "gpt-4o",
                                    value: "{model}",
                                    oninput: move |e| model.set(e.value()),
                                }
                            }
                        }

                        div { class: "form-grid-two-cols",
                            div { class: "form-field",
                                label { "API 密钥 (API Key)" }
                                input {
                                    r#type: "password",
                                    placeholder: "sk-...",
                                    value: "{api_key}",
                                    oninput: move |e| api_key.set(e.value()),
                                }
                            }

                            div { class: "form-field",
                                label { "最大输出限制 (Max Tokens)" }
                                input {
                                    r#type: "number",
                                    placeholder: "2048",
                                    value: "{max_tokens}",
                                    oninput: move |e| {
                                        if let Ok(val) = e.value().parse::<u32>() {
                                            max_tokens.set(val);
                                        }
                                    },
                                }
                            }
                        }

                        div { class: "form-field",
                            label { "系统提示词 (System Prompt)" }
                            textarea {
                                class: "prompt-textarea",
                                placeholder: "输入赋予该 AI 角色的设定、推理逻辑以及遵守规则...",
                                value: "{prompt}",
                                oninput: move |e| prompt.set(e.value()),
                            }
                        }

                        div { class: "settings-actions",
                            button {
                                class: "cancel-settings-btn glass-panel-subtle",
                                onclick: move |_| {
                                    nav.go_back();
                                },
                                "取消"
                            }
                            button {
                                class: if *saving.read() { "save-settings-btn loading" } else { "save-settings-btn" },
                                onclick: handle_save,
                                disabled: *saving.read(),
                                if *saving.read() {
                                    span { class: "spinner" }
                                } else {
                                    "💾 保存配置"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
