use std::sync::Mutex;

pub struct AppState {
    pub pinned: Mutex<bool>,
    /// True only when user hid the window via tray (not Win+D / minimize).
    pub user_hidden: Mutex<bool>,
}
