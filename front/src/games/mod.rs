pub mod lincoln;
pub mod registry;
pub mod texas_holdem;
pub mod werewolf;

// pub use registry::{GameConfigProps, REGISTRY}; // consumers import from registry directly

use dioxus::prelude::*;
use serde_json::Value;

/// 插件契约：外壳与插件之间的纯净接口
#[derive(Props, Clone, PartialEq)]
pub struct GamePluginProps {
    pub state: Signal<Value>,
    pub on_action: Callback<Value>,
    pub actor_id: String,
}

/// 类型擦除后的动态分发器
///
/// 不生产数据，不渲染具体游戏画面，唯一的宿命就是根据 game_type 分发到对应插件
#[component]
pub fn GamePluginManager(game_type: String, props: GamePluginProps) -> Element {
    if let Some(def) = registry::REGISTRY.get(&game_type) {
        let Comp = def.game_component;
        rsx! {
            Comp {
                state: props.state,
                on_action: props.on_action,
                actor_id: props.actor_id,
            }
        }
    } else {
        rsx! {
            div { class: "unknown-game",
                div { class: "unknown-game-icon", "🎮" }
                h2 { "未知游戏类型" }
                p { "game_type: \"{game_type}\"" }
            }
        }
    }
}
