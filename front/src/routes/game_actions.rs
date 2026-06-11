use crate::dioxus_router::Navigator;
use tracing::debug;

pub fn copy_room_id(room_id: String) {
    if let Some(win) = web_sys::window() {
        let _ = win.navigator().clipboard().write_text(&room_id);
        debug!(target: "game", room_id = %room_id, "房间号已复制到剪贴板");
    }
}

pub fn open_settings(navigator: Navigator, room_id: String, actor_id: String) {
    navigator.push(format!("/settings/{}/{}", room_id, actor_id));
}
