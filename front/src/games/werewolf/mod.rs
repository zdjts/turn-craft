use dioxus::prelude::*;

use super::GamePluginProps;

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
