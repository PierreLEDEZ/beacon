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
use windows::core::PWSTR;
#[cfg(windows)]
use windows::Win32::Foundation::{CloseHandle, HWND};
#[cfg(windows)]
use windows::Win32::System::Threading::{
    AttachThreadInput, GetCurrentThreadId, OpenProcess, QueryFullProcessImageNameW,
    PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
};
#[cfg(windows)]
use windows::Win32::UI::Input::KeyboardAndMouse::SetFocus;
#[cfg(windows)]
use windows::Win32::UI::WindowsAndMessaging::{
    AllowSetForegroundWindow, GetForegroundWindow, GetWindowThreadProcessId, IsIconic,
    SetForegroundWindow, ShowWindow, ASFW_ANY, SW_RESTORE,
};

/// Exe basenames (no extension, case-insensitive) that are definitely
/// NOT terminal emulators. Used to reject the foreground window if the
/// user happened to have Claude Desktop / the Start menu / etc. up when
/// the session's first event fired.
const NON_TERMINAL_EXES: &[&str] = &[
    "claude",                   // Claude Desktop (can be always-on-top, easy to steal focus)
    "beacon",                   // ourselves (WS_EX_NOACTIVATE means this shouldn't happen,
                                // but belt + braces)
    "explorer",                 // Windows shell
    "searchapp",                // Win11 search popup
    "shellexperiencehost",
    "startmenuexperiencehost",
    "textinputhost",
    "lockapp",
    "dwm",                      // compositor
    "applicationframehost",     // host for UWP windows that have nothing else on top
    "chrome",
    "msedge",
    "firefox",
    "brave",
    "discord",
    "slack",
    "spotify",
    "notepad",
    "notepad++",
    "word",
    "excel",
    "outlook",
];

/// Context-aware filter: given the hook-reported `host_terminal_kind`
/// and the exe we resolved from the HWND, decide whether the HWND is
/// a plausible host for this session.
///
/// Special-cases `claude-desktop` events (emitted by the Windows-native
/// `beacon.exe hook`): there the claude.exe window IS the right jump
/// target, so we must not blacklist it.
pub fn is_plausible_host(exe: &str, host_terminal_kind: &str) -> bool {
    if host_terminal_kind == "claude-desktop" {
        // Claude Desktop's own HWND is the expected target. Don't filter.
        return true;
    }
    !NON_TERMINAL_EXES
        .iter()
        .any(|b| b.eq_ignore_ascii_case(exe))
}

/// Grab whatever window is foreground right now. Returns None if the
/// Win32 API reported no foreground window. Encoded as i64 so the
/// value crosses serde to the frontend / stores in Session without
/// bringing windows types into public API.
///
/// Caller is responsible for deciding whether the returned HWND is
/// plausible (see `is_plausible_host`) — for WSL sessions we typically
/// reject claude.exe, for Claude Desktop sessions we accept it.
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

/// Resolve the base name (without `.exe`) of the process that owns
/// `raw_hwnd`. Used when the hook's `$TERM_PROGRAM` / `$WT_SESSION` /
/// etc. detection returned "unknown" — at least we can surface
/// "WindowsTerminal", "powershell", "Code", … in the session card.
#[cfg(windows)]
pub fn process_name_of_hwnd(raw_hwnd: i64) -> Option<String> {
    unsafe {
        let hwnd = HWND(raw_hwnd as *mut core::ffi::c_void);
        let mut pid: u32 = 0;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        if pid == 0 {
            return None;
        }

        let handle =
            OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;

        let mut buf = [0u16; 512];
        let mut len = buf.len() as u32;
        let res = QueryFullProcessImageNameW(
            handle,
            PROCESS_NAME_WIN32,
            PWSTR(buf.as_mut_ptr()),
            &mut len,
        );
        let _ = CloseHandle(handle);

        if res.is_err() {
            return None;
        }

        let path = String::from_utf16_lossy(&buf[..len as usize]);
        let file = path
            .rsplit(|c| c == '\\' || c == '/')
            .next()
            .filter(|s| !s.is_empty())?;
        let stripped = file
            .strip_suffix(".exe")
            .or_else(|| file.strip_suffix(".EXE"))
            .unwrap_or(file);
        Some(stripped.to_string())
    }
}

#[cfg(not(windows))]
pub fn process_name_of_hwnd(_raw_hwnd: i64) -> Option<String> {
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
