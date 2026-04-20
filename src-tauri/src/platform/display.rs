//! Multi-monitor helpers. Used to position the notch on the monitor
//! under the user's cursor rather than always landing on the primary.

/// Physical-pixel working area of a monitor (excludes the taskbar).
#[derive(Debug, Clone, Copy)]
pub struct WorkArea {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

#[cfg(windows)]
pub fn cursor_monitor_work_area() -> Option<WorkArea> {
    use windows::Win32::Foundation::POINT;
    use windows::Win32::Graphics::Gdi::{
        GetMonitorInfoW, MonitorFromPoint, MONITORINFO, MONITOR_DEFAULTTONEAREST,
    };
    use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;

    unsafe {
        let mut pt = POINT::default();
        if GetCursorPos(&mut pt).is_err() {
            return None;
        }
        let hmon = MonitorFromPoint(pt, MONITOR_DEFAULTTONEAREST);
        if hmon.is_invalid() {
            return None;
        }
        let mut info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        if !GetMonitorInfoW(hmon, &mut info).as_bool() {
            return None;
        }
        let rc = info.rcWork;
        Some(WorkArea {
            x: rc.left,
            y: rc.top,
            width: rc.right - rc.left,
            height: rc.bottom - rc.top,
        })
    }
}

#[cfg(not(windows))]
pub fn cursor_monitor_work_area() -> Option<WorkArea> {
    None
}
