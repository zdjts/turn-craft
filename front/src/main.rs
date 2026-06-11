use dioxus::prelude::*;
use tracing::info;

/// 后端服务地址（固定 8080 端口，不随 dx serve 端口漂移）
const BACKEND_ORIGIN: &str = "http://127.0.0.1:8080";

const FAVICON: Asset = asset!("/assets/favicon.ico");
const MAIN_CSS: Asset = asset!("/assets/main.css");
const LOBBY_CSS: Asset = asset!("/assets/lobby.css");
const SETTINGS_CSS: Asset = asset!("/assets/settings.css");
const POKER_CSS: Asset = asset!("/assets/poker.css");
const TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");

mod api;
mod games;
mod routes;
mod services;

fn main() {
    tracing_wasm::set_as_global_default();
    info!("🏛️ 前端应用正在启动...");
    dioxus::launch(App);
}

/// Root component: Dioxus Router drives page switching.
#[component]
fn App() -> Element {
    rsx! {
        document::Link { rel: "icon", href: FAVICON }
        document::Link { rel: "stylesheet", href: MAIN_CSS }
        document::Link { rel: "stylesheet", href: LOBBY_CSS }
        document::Link { rel: "stylesheet", href: SETTINGS_CSS }
        document::Link { rel: "stylesheet", href: POKER_CSS }
        document::Link { rel: "stylesheet", href: TAILWIND_CSS }

        Router::<routes::Route> {}
    }
}
