//! Screen work-area bounds for preview window placement.
//!
//! On Windows, `SetWindowPos` and `SPI_GETWORKAREA` use the same logical coordinate
//! space (respecting display scaling). `xcap` monitor sizes are physical pixels and
//! must not be mixed with minifb positioning.

use minifb::Window;

use crate::commands::capture::PreviewPosition;

/// Usable desktop rectangle (excludes taskbar when reported by the OS).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScreenBounds {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

/// Ensure the current thread is attached to the interactive display desktop (`Default`).
///
/// In sandboxed terminal environments (e.g. Cursor, Antigravity IDE, SSH, or containerized runners),
/// threads are often spawned into an isolated virtual desktop (`exebox-...`). Windows created
/// in those sandboxes do not render to the physical monitor because DWM only composites the `Default`
/// input desktop.
///
/// On Windows, this function queries the thread desktop and, if it is not `"default"`, dynamically
/// binds the thread to the active user display desktop. On non-Windows platforms, this is a no-op.
pub fn ensure_interactive_desktop() -> Option<String> {
    #[cfg(windows)]
    {
        ensure_interactive_desktop_windows()
    }
    #[cfg(not(windows))]
    {
        None
    }
}

#[cfg(windows)]
fn ensure_interactive_desktop_windows() -> Option<String> {
    use windows_sys::Win32::System::StationsAndDesktops::{
        GetThreadDesktop, GetUserObjectInformationW, OpenDesktopW, OpenInputDesktop,
        SetThreadDesktop, UOI_NAME,
    };
    use windows_sys::Win32::System::Threading::GetCurrentThreadId;

    unsafe {
        let current_desk = GetThreadDesktop(GetCurrentThreadId());
        if current_desk.is_null() {
            return None;
        }

        let mut buf = [0u16; 256];
        let mut needed = 0u32;
        let ok = GetUserObjectInformationW(
            current_desk as _,
            UOI_NAME,
            buf.as_mut_ptr() as *mut _,
            (buf.len() * 2) as u32,
            &mut needed,
        );

        let current_name = if ok != 0 && needed > 2 {
            let len = (needed as usize / 2).saturating_sub(1);
            String::from_utf16_lossy(&buf[..len])
        } else {
            String::new()
        };

        if current_name.eq_ignore_ascii_case("default") {
            return None;
        }

        // Try OpenInputDesktop first with GENERIC_ALL access
        const GENERIC_ALL: u32 = 0x1000_0000;
        const MAXIMUM_ALLOWED: u32 = 0x0200_0000;

        let input_desk = OpenInputDesktop(0, 0, GENERIC_ALL);
        if !input_desk.is_null() {
            if SetThreadDesktop(input_desk) != 0 {
                return Some("Default".to_owned());
            }
        }

        // Fall back to opening the standard "default" desktop on WinSta0
        let default_name: Vec<u16> = "default\0".encode_utf16().collect();
        let default_desk = OpenDesktopW(default_name.as_ptr(), 0, 0, MAXIMUM_ALLOWED);
        if !default_desk.is_null() {
            if SetThreadDesktop(default_desk) != 0 {
                return Some("default".to_owned());
            }
        }

        None
    }
}

/// Primary monitor work area in the same coordinate space minifb uses for positioning.
pub fn primary_work_area() -> Option<ScreenBounds> {
    #[cfg(windows)]
    {
        return primary_work_area_windows();
    }

    #[cfg(not(windows))]
    {
        let monitor = xcap::Monitor::all().ok()?.into_iter().next()?;
        let width = u32::try_from(monitor.width()).ok()?.max(1);
        let height = u32::try_from(monitor.height()).ok()?.max(1);
        Some(ScreenBounds {
            x: 0,
            y: 0,
            width,
            height,
        })
    }
}

#[cfg(windows)]
fn primary_work_area_windows() -> Option<ScreenBounds> {
    use windows_sys::Win32::Foundation::RECT;
    use windows_sys::Win32::UI::WindowsAndMessaging::{SystemParametersInfoW, SPI_GETWORKAREA};

    let mut rect = RECT {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    };
    let ok =
        unsafe { SystemParametersInfoW(SPI_GETWORKAREA, 0, (&mut rect as *mut RECT).cast(), 0) };
    if ok == 0 {
        return None;
    }

    let width = (rect.right - rect.left).max(1) as u32;
    let height = (rect.bottom - rect.top).max(1) as u32;
    Some(ScreenBounds {
        x: rect.left,
        y: rect.top,
        width,
        height,
    })
}

/// Outer window size (frame + title bar). `set_position` anchors this rect, not the client area.
pub fn window_outer_size(window: &Window) -> Option<(u32, u32)> {
    #[cfg(windows)]
    {
        return window_outer_size_windows(window);
    }

    #[cfg(not(windows))]
    {
        let (width, height) = window.get_size();
        Some((width.max(1) as u32, height.max(1) as u32))
    }
}

#[cfg(windows)]
fn window_outer_size_windows(window: &Window) -> Option<(u32, u32)> {
    use windows_sys::Win32::Foundation::RECT;
    use windows_sys::Win32::UI::WindowsAndMessaging::GetWindowRect;

    let hwnd = window.get_window_handle();
    if hwnd.is_null() {
        return None;
    }

    let mut rect = RECT {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    };
    let ok = unsafe { GetWindowRect(hwnd as _, &mut rect) };
    if ok == 0 {
        return None;
    }

    let width = (rect.right - rect.left).max(1) as u32;
    let height = (rect.bottom - rect.top).max(1) as u32;
    Some((width, height))
}

/// Compute top-left position for a window of `window_width` x `window_height` (outer frame).
#[must_use]
pub fn anchored_window_position(
    bounds: &ScreenBounds,
    window_width: u32,
    window_height: u32,
    anchor: PreviewPosition,
) -> (isize, isize) {
    let free_x = bounds.width.saturating_sub(window_width) as i32;
    let free_y = bounds.height.saturating_sub(window_height) as i32;

    let (rel_x, rel_y) = match anchor {
        PreviewPosition::TopLeft => (0, 0),
        PreviewPosition::TopCenter => (free_x / 2, 0),
        PreviewPosition::TopRight => (free_x, 0),
        PreviewPosition::CenterLeft => (0, free_y / 2),
        PreviewPosition::Center => (free_x / 2, free_y / 2),
        PreviewPosition::CenterRight => (free_x, free_y / 2),
        PreviewPosition::BottomLeft => (0, free_y),
        PreviewPosition::BottomCenter => (free_x / 2, free_y),
        PreviewPosition::BottomRight => (free_x, free_y),
    };

    let x = bounds.x + rel_x.max(0);
    let y = bounds.y + rel_y.max(0);

    let max_x = bounds.x + bounds.width as i32 - window_width as i32;
    let max_y = bounds.y + bounds.height as i32 - window_height as i32;

    (
        x.min(max_x).max(bounds.x) as isize,
        y.min(max_y).max(bounds.y) as isize,
    )
}

/// Place the preview window at `anchor`, using outer frame dimensions and work-area clamping.
pub fn apply_anchored_preview_position(
    window: &mut Window,
    bounds: &ScreenBounds,
    anchor: PreviewPosition,
    client_width: u32,
    client_height: u32,
) {
    let (outer_width, outer_height) =
        window_outer_size(window).unwrap_or((client_width.max(1), client_height.max(1)));
    let (x, y) = anchored_window_position(bounds, outer_width, outer_height, anchor);
    window.set_position(x, y);
    window.topmost(true);

    #[cfg(windows)]
    unsafe {
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            BringWindowToTop, SetForegroundWindow, ShowWindow, SW_SHOW,
        };
        let hwnd = window.get_window_handle() as windows_sys::Win32::Foundation::HWND;
        if !hwnd.is_null() {
            ShowWindow(hwnd, SW_SHOW);
            BringWindowToTop(hwnd);
            SetForegroundWindow(hwnd);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::capture::PreviewPosition;

    #[test]
    fn screen_bounds_stores_work_area_offset() {
        let bounds = ScreenBounds {
            x: 0,
            y: 48,
            width: 1536,
            height: 816,
        };
        assert_eq!(bounds.width, 1536);
        assert_eq!(bounds.height, 816);
    }

    #[test]
    fn bottom_center_anchor_maps_to_expected_position() {
        let bounds = ScreenBounds {
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
        };
        let (x, y) = anchored_window_position(&bounds, 640, 360, PreviewPosition::BottomCenter);
        assert_eq!(x, 640);
        assert_eq!(y, 720);
    }

    #[test]
    fn anchored_position_clamps_when_window_exceeds_work_area() {
        let bounds = ScreenBounds {
            x: 0,
            y: 48,
            width: 800,
            height: 600,
        };
        let (x, y) = anchored_window_position(&bounds, 900, 700, PreviewPosition::BottomRight);
        assert_eq!(x, 0);
        assert_eq!(y, 48);
    }

    #[test]
    fn anchored_position_honors_work_area_offset() {
        let bounds = ScreenBounds {
            x: 0,
            y: 48,
            width: 1536,
            height: 816,
        };
        let (x, y) = anchored_window_position(&bounds, 640, 400, PreviewPosition::TopLeft);
        assert_eq!(x, 0);
        assert_eq!(y, 48);
    }
}
