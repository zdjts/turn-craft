pub mod lobby;
pub mod login;
pub mod history;
pub mod public;
pub mod profile;
pub mod about;
pub mod game;
pub mod settings;
pub mod replay;
pub mod layout;

use dioxus::prelude::*;
use lobby::Lobby;
use login::Login;
use history::History;
use public::PublicRooms;
use profile::Profile;
use about::About;
use game::Game;
use settings::Settings;
use replay::Replay;
use layout::AppLayout;

#[derive(Routable, Clone, PartialEq, Debug)]
#[rustfmt::skip]
pub enum Route {
    #[route("/login")]
    Login {},

    #[layout(AppLayout)]
        #[route("/")]
        Lobby {},

        #[route("/history")]
        History {},

        #[route("/public")]
        PublicRooms {},

        #[route("/profile")]
        Profile {},

        #[route("/about")]
        About {},

        #[route("/game/:room_id/:actor_id")]
        Game { room_id: String, actor_id: String },

        #[route("/settings/:room_id/:actor_id")]
        Settings { room_id: String, actor_id: String },

        #[route("/replay/:room_id")]
        Replay { room_id: String },
}
