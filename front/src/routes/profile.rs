use crate::api::{get_history_rooms, get_username, RoomSnapshotData};
use crate::games::registry::REGISTRY;
use dioxus::prelude::*;

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
        get_username().unwrap_or_else(|| "未登录".to_string())
    });

    let total_games = use_memo(move || rooms.read().len());
    let finished_games = use_memo(move || {
        rooms.read().iter().filter(|r| {
            r.engine_state.get("finished").and_then(|f| f.as_bool()).unwrap_or(false)
        }).count()
    });

    rsx! {
        div { class: "pg-profile animate-fade-in",
            div { class: "page-header",
                h1 { "👤 玩家个人主页" }
                p { "查看您的博弈战绩、历史回顾以及系统统计信息。" }
            }

            // Top Stat Cards Row
            div { class: "pg-profile-header",
                // User Identity Card
                div { class: "pg-profile-user g-card",
                    div { class: "pg-profile-avatar",
                        span { class: "pg-profile-avatar-emoji", "🤵" }
                    }
                    div { class: "pg-profile-info",
                        h2 { "{username}" }
                        span { class: "pg-profile-id", "Turn Craft 玩家" }
                    }
                }

                // Stats Summary Card
                div { class: "pg-profile-stats g-card",
                    h3 { "📊 博弈数据统计" }
                    div { class: "pg-profile-metrics",
                        div { class: "pg-profile-metric",
                            div { class: "pg-profile-metric-num", "{total_games}" }
                            div { class: "pg-profile-metric-label", "参局总数" }
                        }
                        div { class: "pg-profile-metric",
                            div { class: "pg-profile-metric-num", "{finished_games}" }
                            div { class: "pg-profile-metric-label", "已结算对局" }
                        }
                        div { class: "pg-profile-metric",
                            div { class: "pg-profile-metric-num", "{total_games() - finished_games()}" }
                            div { class: "pg-profile-metric-label", "未完成" }
                        }
                    }
                    div { class: "pg-profile-winrate",
                        div { class: "pg-profile-winrate-hdr",
                            span { "胜率" }
                            span { class: "pg-profile-winrate-num", "统计开发中" }
                        }
                        div { class: "pg-profile-winrate-track",
                            div { class: "pg-profile-winrate-bar", style: "width: 0%" }
                        }
                    }
                }
            }

            // Recent match history section
            div { class: "pg-profile-recent g-card",
                h3 { "⏱️ 最近参与对局" }

                if *loading.read() {
                    div { class: "g-skeleton-list",
                        for _ in 0..2 {
                            div { class: "g-skeleton-row" }
                        }
                    }
                } else if rooms.read().is_empty() {
                    div { class: "pg-profile-empty",
                        p { "您最近没有任何对局，快去大厅创建一局对决吧！" }
                    }
                } else {
                    div { class: "pg-profile-matches",
                        for room in rooms.read().iter().take(5) {
                            {
                                let rid = room.room_id.clone();
                                let game_def = REGISTRY.get(&room.game_type);
                                let game_name = game_def.map(|g| g.name).unwrap_or("未知游戏");
                                let game_icon = game_def.map(|g| g.icon).unwrap_or("❓");
                                let time_str = room.created_at.chars().take(16).collect::<String>().replace("T", " ");

                                rsx! {
                                    div { key: "{rid}", class: "pg-profile-match g-card-subtle",
                                        div { class: "pg-profile-match-left",
                                            span { class: "pg-profile-match-icon", "{game_icon}" }
                                            span { class: "pg-profile-match-game", "{game_name}" }
                                        }
                                        div { class: "pg-profile-match-mid",
                                            span { class: "pg-profile-match-id", "ID: {rid}" }
                                        }
                                        div { class: "pg-profile-match-right",
                                            span { class: "pg-profile-match-time", "{time_str}" }
                                            {
                                                let is_done = room.engine_state.get("finished").and_then(|v| v.as_bool()).unwrap_or(false);
                                                rsx! {
                                                    span { class: "pg-profile-status", if is_done { "已结算" } else { "未完成" } }
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
    }
}
