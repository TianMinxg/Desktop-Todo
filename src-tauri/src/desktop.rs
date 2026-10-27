use std::sync::atomic::{AtomicBool, AtomicIsize, Ordering};

/// When true, hide/minimize from our tray is allowed.
pub static USER_HIDDEN: AtomicBool = AtomicBool::new(false);

/// When true, stay under every normal app window. Pinning turns this off.
static DESKTOP_LAYER: AtomicBool = AtomicBool::new(true);

/// True while Explorer's desktop is the foreground window (for example after Win+D).
static DESKTOP_SHOWN: AtomicBool = AtomicBool::new(false);

static WIDGET_HWND: AtomicIsize = AtomicIsize::new(0);

pub fn set_desktop_layer(enabled: bool) {
    DESKTOP_LAYER.store(enabled, Ordering::SeqCst);
}

#[cfg(windows)]
pub fn prepare_desktop_widget(window: &tauri::WebviewWindow) {
    let Ok(raw) = window.hwnd() else {
        return;
    };
    let hwnd = windows::Win32::Foundation::HWND(raw.0 as *mut _);
    WIDGET_HWND.store(hwnd.0 as isize, Ordering::SeqCst);

    unsafe {
        strip_chrome(hwnd);
        hide_single_instance_window();
        install_subclass(hwnd);
        install_foreground_hook();
        show_tree(hwnd);
        strip_chrome(hwnd);
    }
}

#[cfg(not(windows))]
pub fn prepare_desktop_widget(_window: &tauri::WebviewWindow) {}

#[cfg(windows)]
pub fn refresh_desktop_widget(window: &tauri::WebviewWindow) {
    let Ok(raw) = window.hwnd() else {
        return;
    };
    let hwnd = windows::Win32::Foundation::HWND(raw.0 as *mut _);
    unsafe {
        strip_chrome(hwnd);
        show_tree(hwnd);
        sink_below_apps(hwnd);
    }
}

#[cfg(not(windows))]
pub fn refresh_desktop_widget(_window: &tauri::WebviewWindow) {}

#[cfg(windows)]
static STRIPPING: AtomicBool = AtomicBool::new(false);

#[cfg(windows)]
unsafe fn strip_chrome(hwnd: windows::Win32::Foundation::HWND) {
    use windows::Win32::Graphics::Dwm::{
        DwmSetWindowAttribute, DWMNCRP_DISABLED, DWMWA_NCRENDERING_POLICY,
        DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_DONOTROUND,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        GetParent, GetWindowLongPtrW, SetWindowLongPtrW, SetWindowPos, GWL_EXSTYLE, GWL_STYLE,
        SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, WS_BORDER,
        WS_CAPTION, WS_CHILD, WS_DLGFRAME, WS_EX_APPWINDOW, WS_EX_CLIENTEDGE, WS_EX_DLGMODALFRAME,
        WS_EX_STATICEDGE, WS_EX_TOOLWINDOW, WS_EX_TRANSPARENT, WS_EX_WINDOWEDGE, WS_MAXIMIZEBOX,
        WS_MINIMIZEBOX, WS_POPUP, WS_SYSMENU, WS_THICKFRAME, WS_VISIBLE,
    };

    // SetWindowPos(SWP_FRAMECHANGED) can re-enter via WINDOWPOSCHANGED; bail out.
    if STRIPPING.swap(true, Ordering::SeqCst) {
        return;
    }

    let style = GetWindowLongPtrW(hwnd, GWL_STYLE) as u32;
    let parent = GetParent(hwnd).unwrap_or_default();
    let is_child = !parent.0.is_null() || (style & WS_CHILD.0) != 0;

    // Drop every bit that paints a system titlebar / frame.
    let mut new_style = style
        & !(WS_CAPTION.0
            | WS_THICKFRAME.0
            | WS_SYSMENU.0
            | WS_MINIMIZEBOX.0
            | WS_MAXIMIZEBOX.0
            | WS_BORDER.0
            | WS_DLGFRAME.0);

    if is_child {
        new_style |= WS_CHILD.0 | WS_VISIBLE.0;
        new_style &= !WS_POPUP.0;
    } else {
        new_style |= WS_POPUP.0 | WS_VISIBLE.0;
        new_style &= !WS_CHILD.0;
    }

    let ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
    // WS_EX_TRANSPARENT would paint the window but send every click to whatever is behind it.
    let new_ex = (ex | WS_EX_TOOLWINDOW.0)
        & !(WS_EX_APPWINDOW.0
            | WS_EX_TRANSPARENT.0
            | WS_EX_DLGMODALFRAME.0
            | WS_EX_WINDOWEDGE.0
            | WS_EX_CLIENTEDGE.0
            | WS_EX_STATICEDGE.0);

    let style_changed = new_style != style;
    let ex_changed = new_ex != ex;

    if style_changed {
        let _ = SetWindowLongPtrW(hwnd, GWL_STYLE, new_style as isize);
    }
    if ex_changed {
        let _ = SetWindowLongPtrW(hwnd, GWL_EXSTYLE, new_ex as isize);
    }

    // Ask DWM not to paint a non-client frame (focus/unfocus shell).
    let policy = DWMNCRP_DISABLED.0;
    let _ = DwmSetWindowAttribute(
        hwnd,
        DWMWA_NCRENDERING_POLICY,
        &policy as *const _ as *const _,
        std::mem::size_of_val(&policy) as u32,
    );
    let corner = DWMWCP_DONOTROUND.0;
    let _ = DwmSetWindowAttribute(
        hwnd,
        DWMWA_WINDOW_CORNER_PREFERENCE,
        &corner as *const _ as *const _,
        std::mem::size_of_val(&corner) as u32,
    );

    if style_changed || ex_changed {
        let _ = SetWindowPos(
            hwnd,
            None,
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
        );
    }

    STRIPPING.store(false, Ordering::SeqCst);
}

#[cfg(windows)]
unsafe fn hide_single_instance_window() {
    use windows::core::PCWSTR;
    use windows::Win32::UI::WindowsAndMessaging::{FindWindowW, ShowWindow, SW_HIDE};
    // tauri-plugin-single-instance names its mutex window `{identifier}-siw`
    let title = windows::core::w!("com.local.desktop-todo-tauri-siw");
    if let Ok(siw) = FindWindowW(PCWSTR::null(), title) {
        if !siw.0.is_null() {
            let _ = ShowWindow(siw, SW_HIDE);
        }
    }
}

#[cfg(windows)]
unsafe fn show_tree(hwnd: windows::Win32::Foundation::HWND) {
    use windows::Win32::Foundation::{HWND, LPARAM};
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumChildWindows, ShowWindow, SW_RESTORE, SW_SHOW,
    };

    let _ = ShowWindow(hwnd, SW_SHOW);
    let _ = ShowWindow(hwnd, SW_RESTORE);

    unsafe extern "system" fn enum_child(child: HWND, _: LPARAM) -> windows::core::BOOL {
        unsafe {
            let _ = ShowWindow(child, SW_SHOW);
            let _ = ShowWindow(child, SW_RESTORE);
        }
        windows::core::BOOL(1)
    }
    let _ = EnumChildWindows(Some(hwnd), Some(enum_child), LPARAM(0));
}

#[cfg(windows)]
unsafe fn sink_below_apps(hwnd: windows::Win32::Foundation::HWND) {
    use windows::Win32::UI::WindowsAndMessaging::{
        SetWindowPos, HWND_BOTTOM, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE,
    };

    if USER_HIDDEN.load(Ordering::SeqCst)
        || !DESKTOP_LAYER.load(Ordering::SeqCst)
        || DESKTOP_SHOWN.load(Ordering::SeqCst)
    {
        return;
    }
    // HWND_BOTTOM is the back of the top-level stack: above the desktop, under apps.
    let _ = SetWindowPos(
        hwnd,
        Some(HWND_BOTTOM),
        0,
        0,
        0,
        0,
        SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
    );
}

#[cfg(windows)]
unsafe fn is_desktop_window(hwnd: windows::Win32::Foundation::HWND) -> bool {
    use windows::Win32::UI::WindowsAndMessaging::GetClassNameW;

    if hwnd.0.is_null() {
        return false;
    }
    let mut buf = [0u16; 32];
    let len = GetClassNameW(hwnd, &mut buf);
    let class = String::from_utf16_lossy(&buf[..len.max(0) as usize]);
    class == "Progman" || class == "WorkerW"
}

/// Show-desktop raises Explorer's desktop above every app window. While it is in
/// front, lift the widget above it; once another app takes over, sink it again.
#[cfg(windows)]
unsafe fn on_foreground_changed(foreground: windows::Win32::Foundation::HWND) {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{
        SetWindowPos, HWND_NOTOPMOST, HWND_TOPMOST, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE,
        SWP_SHOWWINDOW,
    };

    let raw = WIDGET_HWND.load(Ordering::SeqCst);
    if raw == 0 {
        return;
    }
    let hwnd = HWND(raw as *mut _);
    if foreground == hwnd {
        return;
    }
    if USER_HIDDEN.load(Ordering::SeqCst) || !DESKTOP_LAYER.load(Ordering::SeqCst) {
        DESKTOP_SHOWN.store(false, Ordering::SeqCst);
        return;
    }

    if is_desktop_window(foreground) {
        DESKTOP_SHOWN.store(true, Ordering::SeqCst);
        // Explorer raises the desktop after the foreground event fires, so lift
        // again a few times until it has finished.
        std::thread::spawn(move || {
            let hwnd = HWND(raw as *mut _);
            for delay in [0u64, 80, 200, 450, 900] {
                std::thread::sleep(std::time::Duration::from_millis(delay));
                if !DESKTOP_SHOWN.load(Ordering::SeqCst) {
                    return;
                }
                // TOPMOST then NOTOPMOST lands at the top of the normal band without
                // staying always-on-top, and without stealing focus from the desktop.
                let flags = SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_SHOWWINDOW;
                unsafe {
                    let _ = SetWindowPos(hwnd, Some(HWND_TOPMOST), 0, 0, 0, 0, flags);
                    let _ = SetWindowPos(hwnd, Some(HWND_NOTOPMOST), 0, 0, 0, 0, flags);
                }
            }
        });
    } else if DESKTOP_SHOWN.swap(false, Ordering::SeqCst) {
        sink_below_apps(hwnd);
    }
}

#[cfg(windows)]
unsafe fn install_foreground_hook() {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::Accessibility::{SetWinEventHook, HWINEVENTHOOK};
    use windows::Win32::UI::WindowsAndMessaging::{EVENT_SYSTEM_FOREGROUND, WINEVENT_OUTOFCONTEXT};

    static INSTALLED: AtomicBool = AtomicBool::new(false);
    if INSTALLED.swap(true, Ordering::SeqCst) {
        return;
    }

    unsafe extern "system" fn on_event(
        _hook: HWINEVENTHOOK,
        _event: u32,
        hwnd: HWND,
        _id_object: i32,
        _id_child: i32,
        _thread: u32,
        _time: u32,
    ) {
        unsafe { on_foreground_changed(hwnd) }
    }

    // Out-of-context callbacks run on this (UI) thread through its message loop.
    let _ = SetWinEventHook(
        EVENT_SYSTEM_FOREGROUND,
        EVENT_SYSTEM_FOREGROUND,
        None,
        Some(on_event),
        0,
        0,
        WINEVENT_OUTOFCONTEXT,
    );
}

#[cfg(windows)]
unsafe fn install_subclass(hwnd: windows::Win32::Foundation::HWND) {
    use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
    use windows::Win32::UI::Shell::{DefSubclassProc, SetWindowSubclass};
    use windows::Win32::UI::WindowsAndMessaging::{
        ShowWindow, HWND_BOTTOM, SC_MINIMIZE, SWP_HIDEWINDOW, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER,
        SW_RESTORE, SW_SHOW, WINDOWPOS, WM_ACTIVATE, WM_NCACTIVATE, WM_NCCALCSIZE, WM_NCPAINT,
        WM_SIZE, WM_STYLECHANGED, WM_SYSCOMMAND, WM_WINDOWPOSCHANGING,
    };

    static INSTALLED: AtomicBool = AtomicBool::new(false);
    if INSTALLED.swap(true, Ordering::SeqCst) {
        return;
    }

    const SIZE_MINIMIZED: usize = 1;

    unsafe extern "system" fn subclass_proc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
        _id: usize,
        _data: usize,
    ) -> LRESULT {
        let user_hidden = USER_HIDDEN.load(Ordering::SeqCst);

        // Entire window is client area — kills the outer system shell permanently.
        if msg == WM_NCCALCSIZE && wparam.0 != 0 {
            return LRESULT(0);
        }
        // Skip default non-client painting (titlebar / focus ring).
        if msg == WM_NCPAINT {
            return LRESULT(0);
        }
        if msg == WM_NCACTIVATE {
            // Acknowledge activate state without letting DWM draw the caption shell.
            return LRESULT(1);
        }

        // Styles / focus changes can restore chrome; strip it again.
        if msg == WM_STYLECHANGED || msg == WM_ACTIVATE {
            strip_chrome(hwnd);
        }

        if !user_hidden {
            if msg == WM_SYSCOMMAND {
                let cmd = (wparam.0 & 0xFFF0) as u32;
                if cmd == SC_MINIMIZE {
                    return LRESULT(0);
                }
            }
            if msg == WM_SIZE && wparam.0 == SIZE_MINIMIZED {
                let _ = ShowWindow(hwnd, SW_RESTORE);
                let _ = ShowWindow(hwnd, SW_SHOW);
                sink_below_apps(hwnd);
                return LRESULT(0);
            }
            if msg == WM_WINDOWPOSCHANGING {
                let pos = lparam.0 as *mut WINDOWPOS;
                if !pos.is_null() {
                    // Win+D hides or parks windows at (-32000, -32000). Keep ours put.
                    (*pos).flags &= !SWP_HIDEWINDOW;
                    if (*pos).x <= -32000 || (*pos).y <= -32000 {
                        (*pos).flags |= SWP_NOMOVE | SWP_NOSIZE;
                    }
                    // Stay at the bottom of the top-level z-order, above the desktop
                    // and below every other application. Pinning and show-desktop opt out.
                    if DESKTOP_LAYER.load(Ordering::SeqCst) && !DESKTOP_SHOWN.load(Ordering::SeqCst)
                    {
                        (*pos).hwndInsertAfter = HWND_BOTTOM;
                        (*pos).flags &= !SWP_NOZORDER;
                    }
                }
            }
        }

        DefSubclassProc(hwnd, msg, wparam, lparam)
    }

    let _ = SetWindowSubclass(hwnd, Some(subclass_proc), 0xD070, 0);
}
