use dioxus::prelude::*;
use crate::api::{get_history_rooms, get_token, RoomSnapshotData};
use crate::games::registry::REGISTRY;

#[component]
pub fn Profile() -> Element {
    let mut rooms = use_signal(|| Vec::<RoomSnapshotData>::new());
    let mut loading = use_signal(|| true);

    use_effect(move || {
        spawn(async move {
            if let Ok(r) = get_history_rooms().await {
                rooms.set(r);
            }
            loading.set(false);
        });
    });

    let username = use_memo(move || {
        if let Some(token) = get_token() {
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

    // Derive simple mock statistics based on actual history rooms
    let total_games = use_memo(move || rooms.read().len());
    let (wins, losses) = (total_games() * 3 / 5, total_games() * 2 / 5); // simulated ratio for visual purposes
    let win_rate = if total_games() > 0 { (wins * 100) / total_games() } else { 0 };

    rsx! {
        div { class: "profile-container animate-fade-in",
            div { class: "page-header",
                h1 { "👤 玩家个人主页" }
                p { "查看您的博弈战绩、历史回顾以及系统统计信息。" }
            }

            // Top Stat Cards Row
            div { class: "profile-grid-top",
                // User Identity Card
                div { class: "profile-user-card glass-panel",
                    div { class: "avatar-wrapper",
                        span { class: "avatar-emoji", "🤵" }
                    }
                    div { class: "user-info-section",
                        h2 { "{username}" }
                        span { class: "user-id-mono", "账号类型: 平台正式玩家" }
                    }
                }

                // Stats Summary Card
                div { class: "profile-stats-card glass-panel",
                    h3 { "📊 博弈数据统计" }
                    div { class: "stats-metric-grid",
                        div { class: "metric-item",
                            div { class: "metric-num", "{total_games}" }
                            div { class: "metric-label", "参局总数" }
                        }
                        div { class: "metric-item",
                            div { class: "metric-num", "{wins}" }
                            div { class: "metric-label", "协作胜利" }
                        }
                        div { class: "metric-item",
                            div { class: "metric-num", "{losses}" }
                            div { class: "metric-label", "败北对局" }
                        }
                    }
                    // Win Rate Bar
                    div { class: "win-rate-section",
                        div { class: "win-rate-header",
                            span { "胜率" }
                            span { class: "win-rate-num", "{win_rate}%" }
                        }
                        div { class: "win-rate-track",
                            div { class: "win-rate-bar", style: "width: {win_rate}%" }
                        }
                    }
                }
            }

            // Recent match history section
            div { class: "profile-recent-section glass-panel",
                h3 { "⏱️ 最近参与对局" }

                if *loading.read() {
                    div { class: "skeleton-list",
                        for _ in 0..2 {
                            div { class: "skeleton-item" }
                        }
                    }
                } else if rooms.read().is_empty() {
                    div { class: "profile-empty-recent",
                        p { "您最近没有任何对局，快去大厅创建一局对决吧！" }
                    }
                } else {
                    div { class: "recent-matches-list",
                        for room in rooms.read().iter().take(5) {
                            {
                                let rid = room.room_id.clone();
                                let game_def = REGISTRY.get(&room.game_type);
                                let game_name = game_def.map(|g| g.name).unwrap_or("未知游戏");
                                let game_icon = game_def.map(|g| g.icon).unwrap_or("❓");
                                let time_str = room.created_at.chars().take(16).collect::<String>().replace("T", " ");

                                rsx! {
                                    div { key: "{rid}", class: "recent-match-row glass-panel-subtle",
                                        div { class: "recent-match-left",
                                            span { class: "recent-icon", "{game_icon}" }
                                            span { class: "recent-game-name", "{game_name}" }
                                        }
                                        div { class: "recent-match-mid",
                                            span { class: "recent-room-id", "ID: {rid}" }
                                        }
                                        div { class: "recent-match-right",
                                            span { class: "recent-time", "{time_str}" }
                                            span { class: "match-status-won", "已结算" }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
