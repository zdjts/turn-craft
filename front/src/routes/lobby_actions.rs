use std::collections::HashMap;

use dioxus::prelude::*;

pub fn select_lincoln_game(
    mut selected_game: Signal<String>,
    mut selected_role: Signal<String>,
    mut role_modes: Signal<HashMap<String, String>>,
) {
    selected_game.set("lincoln".to_string());
    selected_role.set("Judge".to_string());
    role_modes.set(HashMap::from([
        ("Judge".to_string(), "human".to_string()),
        ("Pro".to_string(), "ai".to_string()),
        ("Con".to_string(), "ai".to_string()),
    ]));
}

pub fn select_texas_game(
    mut selected_game: Signal<String>,
    mut selected_role: Signal<String>,
    mut role_modes: Signal<HashMap<String, String>>,
    player_count: Signal<usize>,
) {
    selected_game.set("texas_holdem".to_string());
    selected_role.set("player1".to_string());
    role_modes.set(HashMap::new());
    role_modes.write().insert("player1".to_string(), "human".to_string());
    for i in 2..=*player_count.read() {
        role_modes
            .write()
            .insert(format!("player{}", i), "ai".to_string());
    }
}

pub fn select_lobby_role(
    mut selected_role: Signal<String>,
    mut role_modes: Signal<HashMap<String, String>>,
    role_name: String,
    roles: &[(&str, &str)],
) {
    selected_role.set(role_name.clone());
    let mut modes = HashMap::new();
    for (name, _) in roles {
        let n = name.to_string();
        if n == role_name {
            modes.insert(n, "human".to_string());
        } else {
            modes.insert(n, "ai".to_string());
        }
    }
    role_modes.set(modes);
}

pub fn set_player_count(
    mut player_count: Signal<usize>,
    mut role_modes: Signal<HashMap<String, String>>,
    count: usize,
) {
    player_count.set(count);
    let mut modes = HashMap::new();
    modes.insert("player1".to_string(), "human".to_string());
    for i in 2..=count {
        modes.insert(format!("player{}", i), "ai".to_string());
    }
    role_modes.set(modes);
}

pub fn set_spectator_mode(mut spectator_mode: Signal<bool>, enabled: bool) {
    spectator_mode.set(enabled);
}
