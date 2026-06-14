use dioxus::prelude::*;

use super::GamePluginProps;
use crate::games::registry::GameConfigProps;

#[allow(unused_variables)]
#[component]
pub fn WerewolfGame(props: GamePluginProps) -> Element {
    rsx! {
        div { class: "werewolf-game",
            div { class: "werewolf-placeholder",
                span { class: "werewolf-icon", "🐺" }
                h2 { "狼人杀 — 尚未实现" }
                p { "在此插入狼人杀游戏组件" }
            }
        }
    }
}

pub fn WerewolfLobbyCard(props: GameConfigProps) -> Element {
    rsx! {
        div { class: "form-field",
            p { style: "color: var(--text-muted); font-size: 0.95rem; text-align: center; padding: 20px;",
                "🐺 狼人杀模式正在开发中，暂无配置项。"
            }
        }
    }
}
