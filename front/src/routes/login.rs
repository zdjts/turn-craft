use crate::api::{login, register, set_token, AuthRequest};
use dioxus::prelude::*;

#[component]
pub fn Login() -> Element {
    let mut username = use_signal(|| String::new());
    let mut password = use_signal(|| String::new());
    let mut is_register = use_signal(|| false);
    let mut error_msg = use_signal(|| Option::<String>::None);
    let mut loading = use_signal(|| false);
    let nav = use_navigator();

    let mut handle_submit = move |_| {
        let u = username.read().trim().to_string();
        let p = password.read().trim().to_string();

        if u.is_empty() || p.is_empty() {
            error_msg.set(Some("用户名和密码不能为空".to_string()));
            return;
        }

        loading.set(true);
        error_msg.set(None);

        spawn(async move {
            let req = AuthRequest {
                username: u,
                password: p,
            };
            let result = if *is_register.read() {
                register(&req).await
            } else {
                login(&req).await
            };

            loading.set(false);
            match result {
                Ok(resp) => {
                    if resp.status == "success" {
                        if let Some(token) = resp.token {
                            set_token(&token);
                            nav.push(super::Route::Lobby {});
                        } else {
                            // If register succeeded, we might toggle back to login or auto-login if token provided
                            if *is_register.read() {
                                is_register.set(false);
                                error_msg.set(Some("注册成功，请登录".to_string()));
                            }
                        }
                    } else {
                        error_msg.set(Some(resp.message.unwrap_or_else(|| "请求失败".to_string())));
                    }
                }
                Err(err) => {
                    error_msg.set(Some(err));
                }
            }
        });
    };

    rsx! {
        div { class: "pg-login-container",
            // Dynamic glow background for login
            div { class: "bg-glow bg-glow-1" }
            div { class: "bg-glow bg-glow-2" }

            div { class: "pg-login-card g-card animate-fade-in",
                div { class: "pg-login-header",
                    span { class: "pg-login-icon", "⚔️" }
                    h1 { class: "pg-login-title", "Turn Craft" }
                    p { class: "pg-login-subtitle", "回合制 AI 与真人协作对局平台" }
                }

                div { class: "pg-login-form",
                    div { class: "g-field",
                        label { "用户名" }
                        input {
                            r#type: "text",
                            placeholder: "输入您的用户名",
                            value: "{username}",
                            oninput: move |e| username.set(e.value()),
                            disabled: *loading.read(),
                        }
                    }

                    div { class: "g-field",
                        label { "密码" }
                        input {
                            r#type: "password",
                            placeholder: "输入您的密码",
                            value: "{password}",
                            oninput: move |e| password.set(e.value()),
                            disabled: *loading.read(),
                            onkeydown: move |e| {
                                if e.key() == Key::Enter {
                                    handle_submit(());
                                }
                            }
                        }
                    }

                        if let Some(msg) = error_msg.read().clone() {
                            {
                                let is_success = msg.contains("注册成功");
                                rsx! {
                                    div {
                                        class: if is_success { "pg-login-success" } else { "pg-login-error" },
                                        span { class: if is_success { "success-icon" } else { "pg-login-error-icon" }, if is_success { "✅" } else { "⚠️" } }
                                        span { "{msg}" }
                                    }
                                }
                            }
                        }

                    button {
                        class: if *loading.read() { "pg-login-submit is-loading" } else { "pg-login-submit" },
                        onclick: move |_| handle_submit(()),
                        disabled: *loading.read(),
                        if *loading.read() {
                            span { class: "g-spinner" }
                        } else if *is_register.read() {
                            "立即注册"
                        } else {
                            "安全登录"
                        }
                    }
                }

                div { class: "pg-login-toggle",
                    if *is_register.read() {
                        span { "已有账号？" }
                        button {
                            class: "pg-login-link",
                            onclick: move |_| {
                                is_register.set(false);
                                error_msg.set(None);
                            },
                            "立即登录"
                        }
                    } else {
                        span { "还没有账号？" }
                        button {
                            class: "pg-login-link",
                            onclick: move |_| {
                                is_register.set(true);
                                error_msg.set(None);
                            },
                            "创建账号"
                        }
                    }
                }
            }
        }
    }
}
