pub mod game;
pub mod lobby;
pub mod settings;

use dioxus::prelude::*;
use game::Game;
use lobby::Lobby;
use settings::Settings;

#[derive(Routable, Clone, PartialEq)]
pub enum Route {
    #[route("/")]
    Lobby {},

    #[route("/game/:room_id/:actor_id")]
    Game { room_id: String, actor_id: String },

    #[route("/settings/:room_id/:actor_id")]
    Settings { room_id: String, actor_id: String },
}
