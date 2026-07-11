use dioxus::prelude::*;
use tracing::info;

/// 后端服务地址（固定 8080 端口，不随 dx serve 端口漂移）
const BACKEND_ORIGIN: &str = "http://127.0.0.1:8080";

const FAVICON: Asset = asset!("/assets/favicon.ico");
const MAIN_CSS: Asset = asset!("/assets/main.css");
const LOGIN_CSS: Asset = asset!("/assets/login.css");
const LOBBY_CSS: Asset = asset!("/assets/lobby.css");
const GAME_CSS: Asset = asset!("/assets/game.css");
const SETTINGS_CSS: Asset = asset!("/assets/settings.css");
const REPLAY_CSS: Asset = asset!("/assets/replay.css");
const HISTORY_CSS: Asset = asset!("/assets/history.css");
const PROFILE_CSS: Asset = asset!("/assets/profile.css");
const ABOUT_CSS: Asset = asset!("/assets/about.css");
const PUBLIC_CSS: Asset = asset!("/assets/public.css");
const POKER_CSS: Asset = asset!("/assets/poker.css");

mod api;
mod games;
mod icons;
mod routes;
mod services;

fn main() {
    let mut config = tracing_wasm::WASMLayerConfigBuilder::new();
    config.set_max_level(tracing::Level::INFO);
    tracing_wasm::set_as_global_default_with_config(config.build());
    info!("🏛️ 前端应用正在启动...");
    dioxus::launch(App);
}

/// Root component: Dioxus Router drives page switching.
#[component]
fn App() -> Element {
    rsx! {
        document::Link { rel: "icon", href: FAVICON }
        document::Link { rel: "stylesheet", href: MAIN_CSS }
        document::Link { rel: "stylesheet", href: LOGIN_CSS }
        document::Link { rel: "stylesheet", href: LOBBY_CSS }
        document::Link { rel: "stylesheet", href: GAME_CSS }
        document::Link { rel: "stylesheet", href: SETTINGS_CSS }
        document::Link { rel: "stylesheet", href: REPLAY_CSS }
        document::Link { rel: "stylesheet", href: HISTORY_CSS }
        document::Link { rel: "stylesheet", href: PROFILE_CSS }
        document::Link { rel: "stylesheet", href: ABOUT_CSS }
        document::Link { rel: "stylesheet", href: PUBLIC_CSS }
        document::Link { rel: "stylesheet", href: POKER_CSS }

        Router::<routes::Route> {}
    }
}
