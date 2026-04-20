//! Win32 HWND helpers for Beacon's jump-to-terminal pipeline.
//!
//! Phase-3a strategy (see docs/ARCHITECTURE.md §9):
//! - On a session's first event, capture whatever window is currently
//!   foreground — in practice that's the terminal the user just typed
//!   `claude` into.
//! - Cache that HWND on the session and reuse it forever (cheap,
//!   stable).
//! - Bringing it back to foreground requires defeating Win11's
//!   focus-stealing guard: attach our input thread to the owner of the
//!   current foreground window for the duration of the SetForegroundWindow
//!   call.

#[cfg(windows)]
use windows::Win32::Foundation::HWND;
#[cfg(windows)]
use windows::Win32::System::Threading::{AttachThreadInput, GetCurrentThreadId};
#[cfg(windows)]
use windows::Win32::UI::Input::KeyboardAndMouse::SetFocus;
#[cfg(windows)]
use windows::Win32::UI::WindowsAndMessaging::{
    AllowSetForegroundWindow, GetForegroundWindow, GetWindowThreadProcessId, IsIconic,
    SetForegroundWindow, ShowWindow, ASFW_ANY, SW_RESTORE,
};

/// Grab whatever window is foreground right now. Returns None if the
/// Win32 API reported no foreground window (rare — only happens during
/// logon / lock screen). Encoded as i64 so the value crosses serde to
/// the frontend / stores in Session without bringing windows types in.
#[cfg(windows)]
pub fn capture_foreground_hwnd() -> Option<i64> {
    let hwnd = unsafe { GetForegroundWindow() };
    if hwnd.0.is_null() {
        None
    } else {
        Some(hwnd.0 as i64)
    }
}

#[cfg(not(windows))]
pub fn capture_foreground_hwnd() -> Option<i64> {
    None
}

/// Bring `hwnd` to the foreground. Does the AttachThreadInput +
/// AllowSetForegroundWindow dance so Win11 doesn't demote us to a
/// taskbar flash.
///
/// Returns Ok(()) even when partial steps fail — the caller should
/// treat the whole jump pipeline as best-effort.
#[cfg(windows)]
pub fn focus_hwnd(raw_hwnd: i64) -> Result<(), String> {
    unsafe {
        let target = HWND(raw_hwnd as *mut core::ffi::c_void);
        let current_fg = GetForegroundWindow();

        // Attach our thread to the input queue of whoever holds focus
        // today so the OS treats our SetForegroundWindow as the user's
        // intent rather than background noise.
        let our_tid = GetCurrentThreadId();
        let mut fg_pid: u32 = 0;
        let fg_tid = GetWindowThreadProcessId(current_fg, Some(&mut fg_pid));
        let attached = fg_tid != 0 && fg_tid != our_tid;
        if attached {
            let _ = AttachThreadInput(our_tid, fg_tid, true);
        }

        // Whitelist any incoming SetForegroundWindow call — belt and
        // braces in case the target process has its own restriction.
        let _ = AllowSetForegroundWindow(ASFW_ANY);

        // If the target is minimized, un-minimize first or the focus
        // call silently no-ops.
        if IsIconic(target).as_bool() {
            let _ = ShowWindow(target, SW_RESTORE);
        }

        let ok = SetForegroundWindow(target).as_bool();

        // Nudge focus into the target's input chain explicitly —
        // otherwise on some systems SetForegroundWindow brings the
        // window up without handing it keyboard focus.
        let _ = SetFocus(Some(target));

        if attached {
            let _ = AttachThreadInput(our_tid, fg_tid, false);
        }

        if ok {
            Ok(())
        } else {
            Err(format!(
                "SetForegroundWindow returned false for hwnd={raw_hwnd}"
            ))
        }
    }
}

#[cfg(not(windows))]
pub fn focus_hwnd(_raw_hwnd: i64) -> Result<(), String> {
    Err("focus_hwnd unsupported outside Windows".into())
}
