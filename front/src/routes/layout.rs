use crate::api::{get_room, get_token, get_username};
use crate::icons::{self, IconSize};
use crate::services::connection;
use dioxus::prelude::*;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ToastType {
    Success,
    Info,
    #[allow(dead_code)]
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

    // Initialize global ConnectionManager once at app level
    let _conn = connection::use_connection_manager();

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

    // ── Feedback context: capture room_id + route for issue body ──
    let (feedback_room_id, feedback_path) = match &current_route {
        super::Route::Game { room_id, .. } => (room_id.clone(), current_route.to_string()),
        super::Route::Settings { room_id, .. } => (room_id.clone(), current_route.to_string()),
        super::Route::Replay { room_id } => (room_id.clone(), current_route.to_string()),
        _ => ("N/A".to_string(), current_route.to_string()),
    };
    let mut feedback_game_type: Signal<String> = use_signal(|| {
        if feedback_room_id == "N/A" {
            "N/A".to_string()
        } else {
            "加载中...".to_string()
        }
    });
    {
        let rid = feedback_room_id.clone();
        use_effect(move || {
            if rid != "N/A" {
                let rid = rid.clone();
                spawn(async move {
                    if let Ok(room) = get_room(&rid).await {
                        feedback_game_type.set(room.game_type);
                    }
                });
            }
        });
    }
    let feedback_url = format!(
        "https://github.com/anomalyco/turn-craft/issues/new?body={}",
        js_sys::encode_uri_component(&format!(
            "## 反馈上下文\n\n- **页面路由**: {}\n- **房间 ID**: {}\n- **游戏类型**: {}\n\n## 反馈内容\n\n(请在此描述您的问题或建议)",
            feedback_path, feedback_room_id, feedback_game_type.read()
        ))
    );

    let logout = move |_| {
        crate::api::remove_token();
        crate::api::remove_username();
        nav.push(super::Route::Login {});
    };

    let username = use_memo(move || {
        get_username().unwrap_or_else(|| "未登录".to_string())
    });

    rsx! {
        div { class: "app-shell",
            // ── Sidebar ──
            div { class: "sidebar g-card",
                div { class: "sidebar-logo",
                    icons::GameIcon { size: IconSize::Lg }
                    span { class: "logo-text", "Turn Craft" }
                }

                div { class: "sidebar-menu",
                    Link {
                        to: super::Route::Lobby {},
                        class: if matches!(current_route, super::Route::Lobby {}) { "menu-item is-active" } else { "menu-item" },
                        span { class: "menu-icon", icons::ArenaIcon { size: IconSize::Md } }
                        span { class: "menu-label", "游戏大厅" }
                    }
                    Link {
                        to: super::Route::PublicRooms {},
                        class: if matches!(current_route, super::Route::PublicRooms {}) { "menu-item is-active" } else { "menu-item" },
                        span { class: "menu-icon", icons::GlobeIcon { size: IconSize::Md } }
                        span { class: "menu-label", "公开房间" }
                    }
                    Link {
                        to: super::Route::History {},
                        class: if matches!(current_route, super::Route::History {}) { "menu-item is-active" } else { "menu-item" },
                        span { class: "menu-icon", icons::ScrollIcon { size: IconSize::Md } }
                        span { class: "menu-label", "历史房间" }
                    }
                    Link {
                        to: super::Route::Profile {},
                        class: if matches!(current_route, super::Route::Profile {}) { "menu-item is-active" } else { "menu-item" },
                        span { class: "menu-icon", icons::UserIcon { size: IconSize::Md } }
                        span { class: "menu-label", "个人主页" }
                    }
                    Link {
                        to: super::Route::About {},
                        class: if matches!(current_route, super::Route::About {}) { "menu-item is-active" } else { "menu-item" },
                        span { class: "menu-icon", icons::InfoIcon { size: IconSize::Md } }
                        span { class: "menu-label", "关于项目" }
                    }
                }

                div { class: "sidebar-footer",
                    // 反馈入口（自动携带上下文）
                    div {
                        class: "sidebar-feedback",
                        style: "margin-bottom: 12px; text-align: center;",
                        a {
                            href: "{feedback_url}",
                            target: "_blank",
                            style: "color: var(--text-muted); font-size: 0.85em; text-decoration: none;",
                            "💬 反馈建议"
                        }
                    }
                    // Theme toggler
                    button {
                        class: "theme-toggle-btn",
                        onclick: toggle_theme,
                        if theme.read().as_str() == "dark" {
                            span { class: "toggle-icon", icons::SunIcon { size: IconSize::Md } }
                            span { class: "toggle-label", "浅色模式" }
                        } else {
                            span { class: "toggle-icon", icons::MoonIcon { size: IconSize::Md } }
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
                            icons::LogoutIcon { size: IconSize::Md }
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
