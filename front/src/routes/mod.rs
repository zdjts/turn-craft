pub mod game;
pub mod game_actions;
pub mod lobby;
pub mod lobby_actions;
pub mod settings;
pub mod settings_actions;

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
