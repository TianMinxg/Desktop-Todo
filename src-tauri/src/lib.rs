mod desktop;
mod reminder;
mod state;
mod store;

use desktop::{prepare_desktop_widget, refresh_desktop_widget, USER_HIDDEN};
use reminder::start_reminder_loop;
use state::AppState;
use std::sync::atomic::Ordering;
use std::sync::Mutex;
use store::{load_todos, merge_and_save, DataDirUpdate, Todo};
use tauri::{
    image::Image,
    menu::{CheckMenuItem, Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager, Runtime, WebviewWindow,
};
use tauri_plugin_autostart::MacosLauncher;
use tauri_plugin_notification::NotificationExt;

fn main_window<R: Runtime>(app: &AppHandle<R>) -> Option<WebviewWindow<R>> {
    app.get_webview_window("main")
}

fn is_user_hidden(app: &AppHandle) -> bool {
    app.state::<AppState>()
        .user_hidden
        .lock()
        .map(|v| *v)
        .unwrap_or(false)
}

fn set_user_hidden(app: &AppHandle, hidden: bool) {
    USER_HIDDEN.store(hidden, Ordering::SeqCst);
    if let Ok(mut v) = app.state::<AppState>().user_hidden.lock() {
        *v = hidden;
    }
}

fn is_pinned(app: &AppHandle) -> bool {
    app.state::<AppState>()
        .pinned
        .lock()
        .map(|v| *v)
        .unwrap_or(false)
}

fn place_window(window: &WebviewWindow) {
    let _ = window.set_title("");
    let _ = window.set_decorations(false);
    let _ = window.set_shadow(false);
    // Tauri may briefly re-apply native chrome; strip Win32 shell immediately after.
    refresh_desktop_widget(window);

    if let Ok(Some(monitor)) = window.current_monitor() {
        let size = monitor.size();
        let scale = monitor.scale_factor();
        // Size comes from tauri.conf.json. Only use it to pin the window to the top-right.
        let width = window
            .outer_size()
            .map(|outer| outer.width as f64 / scale)
            .unwrap_or(0.0);
        let x = (size.width as f64 / scale) - width - 24.0;
        let y = 24.0;
        let _ = window.set_position(tauri::LogicalPosition::new(x.max(0.0), y));
    }
}

fn apply_layer(app: &AppHandle, window: &WebviewWindow) {
    let pinned = is_pinned(app);
    // Flag first so the z-order hook does not pull a pinned window back down.
    desktop::set_desktop_layer(!pinned);
    let _ = window.set_always_on_top(pinned);
    if !pinned {
        refresh_desktop_widget(window);
    }
}

fn reveal_window(app: &AppHandle, focus: bool) {
    if let Some(window) = main_window(app) {
        set_user_hidden(app, false);
        place_window(&window);
        let _ = window.unminimize();
        let _ = window.show();
        refresh_desktop_widget(&window);
        apply_layer(app, &window);
        if focus {
            let _ = window.set_focus();
        }
    }
}

fn hide_window(app: &AppHandle) {
    if let Some(window) = main_window(app) {
        set_user_hidden(app, true);
        let _ = window.hide();
    }
}

fn toggle_window(app: &AppHandle) {
    if is_user_hidden(app) {
        reveal_window(app, true);
        return;
    }
    if let Some(window) = main_window(app) {
        let visible = window.is_visible().unwrap_or(false);
        if visible {
            hide_window(app);
        } else {
            reveal_window(app, true);
        }
    }
}

fn set_pinned_state(app: &AppHandle, pinned: bool) {
    if let Ok(mut state) = app.state::<AppState>().pinned.lock() {
        *state = pinned;
    }
    if let Some(window) = main_window(app) {
        apply_layer(app, &window);
        if pinned {
            let _ = window.set_focus();
        }
    }
}

#[tauri::command]
fn get_todos() -> Vec<Todo> {
    load_todos()
}

#[tauri::command]
fn save_todos(todos: Vec<Todo>) -> bool {
    merge_and_save(todos);
    true
}

#[tauri::command]
fn get_autolaunch(app: AppHandle) -> Result<bool, String> {
    use tauri_plugin_autostart::ManagerExt;
    app.autolaunch().is_enabled().map_err(|e| e.to_string())
}

#[tauri::command]
fn get_data_dir() -> String {
    store::current_data_dir()
}

#[tauri::command]
fn set_data_dir(path: String) -> Result<DataDirUpdate, String> {
    store::set_data_dir(&path)
}

#[tauri::command]
fn set_autolaunch(app: AppHandle, enabled: bool) -> Result<bool, String> {
    use tauri_plugin_autostart::ManagerExt;
    let launcher = app.autolaunch();
    if enabled {
        launcher.enable().map_err(|e| e.to_string())?;
    } else {
        launcher.disable().map_err(|e| e.to_string())?;
    }
    launcher.is_enabled().map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            reveal_window(app, true);
        }))
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            Some(vec!["--opened-at-login"]),
        ))
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState {
            pinned: Mutex::new(false),
            user_hidden: Mutex::new(false),
        })
        .invoke_handler(tauri::generate_handler![
            get_todos,
            save_todos,
            get_autolaunch,
            set_autolaunch,
            get_data_dir,
            set_data_dir
        ])
        .setup(|app| {
            let handle = app.handle().clone();

            let _ = handle.notification().request_permission();

            {
                use tauri_plugin_autostart::ManagerExt;
                let launcher = handle.autolaunch();
                if !launcher.is_enabled().unwrap_or(false) {
                    let _ = launcher.enable();
                }
            }

            if let Some(window) = main_window(&handle) {
                let _ = window.set_skip_taskbar(true);
                place_window(&window);
                apply_layer(&handle, &window);
                let _ = window.show();

                // Stay a top-level window so clicks and typing still reach WebView2.
                // The z-order hook keeps it under other apps; a foreground hook lifts
                // it above the desktop when Win+D shows the desktop.
                prepare_desktop_widget(&window);
                let _ = window.set_focus();

                // Re-strip chrome / clear native title after WebView2 spins up
                let w = window.clone();
                std::thread::spawn(move || {
                    for ms in [300u64, 1000, 2500] {
                        std::thread::sleep(std::time::Duration::from_millis(ms));
                        let _ = w.set_title("");
                        let _ = w.set_decorations(false);
                        refresh_desktop_widget(&w);
                    }
                });
            }

            let show_item = MenuItem::with_id(app, "show", "显示 / 隐藏", true, None::<&str>)?;
            let pin_item = CheckMenuItem::with_id(app, "pin", "置顶", true, false, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_item, &pin_item, &quit_item])?;

            let tray_icon = Image::from_bytes(include_bytes!("../icons/tray.png"))
                .or_else(|_| Image::from_bytes(include_bytes!("../icons/32x32.png")))?;

            let _tray = TrayIconBuilder::new()
                .icon(tray_icon)
                .tooltip("Desktop Todo")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => toggle_window(app),
                    "pin" => {
                        let next = !is_pinned(app);
                        set_pinned_state(app, next);
                    }
                    "quit" => {
                        app.exit(0);
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        toggle_window(tray.app_handle());
                    }
                })
                .build(app)?;

            start_reminder_loop(handle);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
