pub mod about;
pub mod game;
pub mod history;
pub mod layout;
pub mod lobby;
pub mod login;
pub mod profile;
pub mod public;
pub mod replay;
pub mod settings;

use about::About;
use dioxus::prelude::*;
use game::Game;
use history::History;
use layout::AppLayout;
use lobby::Lobby;
use login::Login;
use profile::Profile;
use public::PublicRooms;
use replay::Replay;
use settings::Settings;

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
