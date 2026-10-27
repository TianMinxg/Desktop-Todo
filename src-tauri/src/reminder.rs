use crate::store;
use chrono::Utc;
use std::thread;
use std::time::Duration;
use tauri::AppHandle;
use tauri_plugin_notification::NotificationExt;

fn check_once(app: &AppHandle) {
    let due = store::take_due_reminders(Utc::now().timestamp_millis());
    for todo in &due {
        let _ = app
            .notification()
            .builder()
            .title("待办提醒")
            .body(&todo.title)
            .show();
    }
}

pub fn start_reminder_loop(app: AppHandle) {
    thread::spawn(move || loop {
        check_once(&app);
        thread::sleep(Duration::from_secs(30));
    });
}
