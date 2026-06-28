//! OS session-lock listener.
//!
//! On Windows this registers a real WTS session-change notification via a
//! message-only window and locks the vault when the OS session is locked
//! (e.g. Win+L). On macOS/Linux these are best-effort no-op stubs that
//! compile; runtime auto-lock there falls back to the frontend idle timeout
//! (Task 20).

use tauri::{AppHandle, Emitter, Manager};

/// Lock the vault and notify the frontend.
///
/// Safe to call from any thread. Never panics on the lock-notify path even if
/// the mutex is poisoned, so it is safe to invoke from inside a Win32 WNDPROC.
fn do_lock(app: &AppHandle) {
    if let Some(state) = app.try_state::<crate::commands::AppState>() {
        // Recover from a poisoned mutex rather than panicking inside the
        // WNDPROC (a panic across the FFI boundary is undefined behavior).
        match state.vault.lock() {
            Ok(mut guard) => guard.lock(),
            Err(poisoned) => poisoned.into_inner().lock(),
        }
    }
    let _ = app.emit("locked", ());
}

// ---------------------------------------------------------------------------
// Windows: real WTS session-lock listener.
// ---------------------------------------------------------------------------
#[cfg(target_os = "windows")]
pub fn start_session_lock_listener(app: AppHandle) {
    std::thread::spawn(move || {
        windows_session_loop(app);
    });
}

#[cfg(target_os = "windows")]
mod win {
    use super::do_lock;
    use std::sync::Mutex;
    use tauri::AppHandle;

    use windows::core::PCWSTR;
    use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
    use windows::Win32::System::RemoteDesktop::{
        WTSRegisterSessionNotification, WTSUnRegisterSessionNotification, NOTIFY_FOR_THIS_SESSION,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetMessageW,
        RegisterClassW, TranslateMessage, HWND_MESSAGE, MSG, WINDOW_EX_STYLE, WINDOW_STYLE,
        WM_WTSSESSION_CHANGE, WNDCLASSW, WTS_SESSION_LOCK,
    };

    // The WNDPROC is a plain `extern "system" fn` and cannot capture state, so
    // the AppHandle is stashed in a process-global slot before the message
    // loop starts. AppHandle is Send + Sync + Clone (an Arc internally), so a
    // Mutex<Option<AppHandle>> is sound to read from the WNDPROC thread.
    static APP: Mutex<Option<AppHandle>> = Mutex::new(None);

    /// Window procedure. Runs on the message-loop thread. Must not panic
    /// (panicking across the FFI boundary into Win32 is UB), so every branch
    /// is panic-free.
    pub(super) extern "system" fn wndproc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        if msg == WM_WTSSESSION_CHANGE && wparam.0 as u32 == WTS_SESSION_LOCK {
            // Clone the handle out while holding the lock for the minimum
            // time, then release before doing the (potentially slower) work.
            let handle = APP.lock().ok().and_then(|g| g.clone());
            if let Some(app) = handle {
                do_lock(&app);
            }
            return LRESULT(0);
        }
        unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
    }

    pub(super) fn run(app: AppHandle) {
        // Publish the handle for the WNDPROC before creating the window.
        if let Ok(mut slot) = APP.lock() {
            *slot = Some(app);
        }

        // NUL-terminated UTF-16 class name.
        let class_name: Vec<u16> = "VaultOtpSessionLockWindow\0".encode_utf16().collect();
        let class_ptr = PCWSTR(class_name.as_ptr());

        unsafe {
            let wc = WNDCLASSW {
                lpfnWndProc: Some(wndproc),
                lpszClassName: class_ptr,
                ..Default::default()
            };

            // RegisterClassW returns 0 (an ATOM of 0) on failure. A null
            // hInstance ties the class to the calling module, which is fine
            // for an in-process message-only window.
            let atom = RegisterClassW(&wc);
            if atom == 0 {
                // Could not register; fall back to idle timeout. Clear the slot.
                if let Ok(mut slot) = APP.lock() {
                    *slot = None;
                }
                return;
            }

            // Message-only window: HWND_MESSAGE parent, no visible UI.
            let hwnd = match CreateWindowExW(
                WINDOW_EX_STYLE(0),
                class_ptr,
                class_ptr,
                WINDOW_STYLE(0),
                0,
                0,
                0,
                0,
                HWND_MESSAGE,
                None,
                None,
                None,
            ) {
                Ok(h) => h,
                Err(_) => {
                    if let Ok(mut slot) = APP.lock() {
                        *slot = None;
                    }
                    return;
                }
            };

            // Register for session-change notifications for this session.
            if WTSRegisterSessionNotification(hwnd, NOTIFY_FOR_THIS_SESSION).is_err() {
                let _ = DestroyWindow(hwnd);
                if let Ok(mut slot) = APP.lock() {
                    *slot = None;
                }
                return;
            }

            // Classic Win32 message pump. GetMessageW returns:
            //   > 0  normal message
            //   = 0  WM_QUIT  -> exit loop
            //   = -1 error    -> exit loop
            let mut msg = MSG::default();
            loop {
                let res = GetMessageW(&mut msg, None, 0, 0);
                if res.0 <= 0 {
                    break;
                }
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }

            // Cleanup (reached only if the message loop ends, e.g. WM_QUIT).
            let _ = WTSUnRegisterSessionNotification(hwnd);
            let _ = DestroyWindow(hwnd);
        }
    }
}

#[cfg(target_os = "windows")]
fn windows_session_loop(app: AppHandle) {
    win::run(app);
}

// ---------------------------------------------------------------------------
// macOS: best-effort no-op stub (compiles; runtime lock via idle timeout).
// ---------------------------------------------------------------------------
#[cfg(target_os = "macos")]
pub fn start_session_lock_listener(app: AppHandle) {
    // A full implementation would observe "com.apple.screenIsLocked" on the
    // distributed notification center. Not implemented yet; the frontend idle
    // timeout (Task 20) is the fallback.
    let _ = app;
}

// ---------------------------------------------------------------------------
// Linux: best-effort no-op stub (compiles; runtime lock via idle timeout).
// ---------------------------------------------------------------------------
#[cfg(target_os = "linux")]
pub fn start_session_lock_listener(app: AppHandle) {
    // A full implementation would listen on D-Bus for the
    // org.freedesktop.login1 session "Lock" signal or
    // org.freedesktop.ScreenSaver ActiveChanged. Not implemented yet; the
    // frontend idle timeout (Task 20) is the fallback.
    let _ = app;
}

// Keep `do_lock` referenced on every platform (the macOS/Linux stubs do not
// call it), avoiding a dead-code warning on those targets.
#[cfg(not(target_os = "windows"))]
#[allow(dead_code)]
fn _ensure_used(app: &AppHandle) {
    do_lock(app);
}
