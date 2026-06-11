use crate::dioxus_router::Navigator;

pub fn go_back(navigator: Navigator, room_id: String, actor_id: String) {
    navigator.push(format!("/game/{}/{}", room_id, actor_id));
}
