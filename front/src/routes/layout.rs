use dioxus::prelude::*;
use crate::api::get_token;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ToastType {
    Success,
    Info,
    Warning,
    Error,
}

#[derive(Clone, Copy)]
pub struct ToastService {
    show: Signal<Option<(String, ToastType)>>,
}

impl ToastService {
    pub fn show(&self, message: String, toast_type: ToastType) {
        let mut signal = self.show;
        signal.set(Some((message, toast_type)));
    }
}

pub fn use_toast() -> ToastService {
    use_context::<ToastService>()
}

#[component]
pub fn AppLayout() -> Element {
    let nav = use_navigator();
    
    // Auth redirect check
    use_effect(move || {
        if get_token().is_none() {
            nav.push(super::Route::Login {});
        }
    });

    // Theme state: "dark" or "light"
    let mut theme = use_signal(|| {
        if let Some(win) = web_sys::window() {
            if let Ok(Some(storage)) = win.local_storage() {
                if let Ok(Some(saved)) = storage.get_item("theme") {
                    return saved;
                }
            }
        }
        "dark".to_string() // default fallback
    });

    // Sync theme with HTML attribute
    use_effect(move || {
        if let Some(win) = web_sys::window() {
            if let Some(doc) = win.document() {
                if let Some(el) = doc.document_element() {
                    let t = theme.read().clone();
                    let _ = el.set_attribute("data-theme", &t);
                    if let Ok(Some(storage)) = win.local_storage() {
                        let _ = storage.set_item("theme", &t);
                    }
                }
            }
        }
    });

    let toggle_theme = move |_| {
        if theme.read().as_str() == "dark" {
            theme.set("light".to_string());
        } else {
            theme.set("dark".to_string());
        }
    };

    // Global Toast System
    let toast_show = use_signal(|| None::<(String, ToastType)>);
    use_context_provider(|| ToastService { show: toast_show });

    // Auto-dismiss toast after 3 seconds
    let mut toast_show_handle = toast_show.clone();
    use_effect(move || {
        if toast_show_handle.read().is_some() {
            // Spawn a task to clear it
            spawn(async move {
                gloo_timers::future::TimeoutFuture::new(3000).await;
                toast_show_handle.set(None);
            });
        }
    });

    // Current route to highlight active sidebar item
    let current_route = use_route::<super::Route>();

    let logout = move |_| {
        crate::api::remove_token();
        nav.push(super::Route::Login {});
    };

    // User info fallback: decode username from token or use generic name
    let username = use_memo(move || {
        if let Some(token) = get_token() {
            // A simple decode helper or default to a readable name
            if token.contains(":") {
                let parts: Vec<&str> = token.split(':').collect();
                if parts.len() > 1 {
                    return parts[0].to_string();
                }
            }
            token.chars().take(8).collect::<String>()
        } else {
            "未登录".to_string()
        }
    });

    rsx! {
        div { class: "app-shell",
            // ── Sidebar ──
            div { class: "sidebar glass-panel",
                div { class: "sidebar-logo",
                    span { class: "logo-icon", "⚔️" }
                    span { class: "logo-text", "Turn Craft" }
                }

                div { class: "sidebar-menu",
                    Link {
                        to: super::Route::Lobby {},
                        class: if matches!(current_route, super::Route::Lobby {}) { "menu-item active" } else { "menu-item" },
                        span { class: "menu-icon", "🏟️" }
                        span { class: "menu-label", "游戏大厅" }
                    }
                    Link {
                        to: super::Route::PublicRooms {},
                        class: if matches!(current_route, super::Route::PublicRooms {}) { "menu-item active" } else { "menu-item" },
                        span { class: "menu-icon", "🌐" }
                        span { class: "menu-label", "公开房间" }
                    }
                    Link {
                        to: super::Route::History {},
                        class: if matches!(current_route, super::Route::History {}) { "menu-item active" } else { "menu-item" },
                        span { class: "menu-icon", "📜" }
                        span { class: "menu-label", "历史房间" }
                    }
                    Link {
                        to: super::Route::Profile {},
                        class: if matches!(current_route, super::Route::Profile {}) { "menu-item active" } else { "menu-item" },
                        span { class: "menu-icon", "👤" }
                        span { class: "menu-label", "个人主页" }
                    }
                    Link {
                        to: super::Route::About {},
                        class: if matches!(current_route, super::Route::About {}) { "menu-item active" } else { "menu-item" },
                        span { class: "menu-icon", "ℹ️" }
                        span { class: "menu-label", "关于项目" }
                    }
                }

                div { class: "sidebar-footer",
                    // Theme toggler
                    button {
                        class: "theme-toggle-btn",
                        onclick: toggle_theme,
                        if theme.read().as_str() == "dark" {
                            span { class: "toggle-icon", "☀️" }
                            span { class: "toggle-label", "浅色模式" }
                        } else {
                            span { class: "toggle-icon", "🌙" }
                            span { class: "toggle-label", "深色模式" }
                        }
                    }

                    // User Info Card
                    div { class: "user-profile-summary",
                        div { class: "user-avatar", "🤵" }
                        div { class: "user-details",
                            div { class: "user-name", "{username}" }
                            div { class: "user-role-badge", "玩家" }
                        }
                        button {
                            class: "logout-btn",
                            onclick: logout,
                            title: "退出登录",
                            "🚪"
                        }
                    }
                }
            }

            // ── Main viewport with dynamic background glow ──
            div { class: "viewport",
                // Dynamic ambient glows
                div { class: "bg-glow bg-glow-1" }
                div { class: "bg-glow bg-glow-2" }
                
                div { class: "viewport-content",
                    Outlet::<super::Route> {}
                }
            }

            // ── Toast Notifications ──
            if let Some((msg, ttype)) = toast_show.read().clone() {
                {
                    let class_name = match ttype {
                        ToastType::Success => "toast success",
                        ToastType::Info => "toast info",
                        ToastType::Warning => "toast warning",
                        ToastType::Error => "toast error",
                    };
                    let icon = match ttype {
                        ToastType::Success => "✅",
                        ToastType::Info => "ℹ️",
                        ToastType::Warning => "⚠️",
                        ToastType::Error => "❌",
                    };
                    rsx! {
                        div { class: "{class_name}",
                            span { class: "toast-icon", "{icon}" }
                            span { class: "toast-message", "{msg}" }
                        }
                    }
                }
            }
        }
    }
}
