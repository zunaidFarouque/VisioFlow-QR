use std::path::PathBuf;

/// Read plain text from the clipboard, if available.
pub fn read_clipboard_text() -> Option<String> {
    let mut clipboard = arboard::Clipboard::new().ok()?;
    clipboard.get_text().ok()
}

/// Read file paths from the clipboard when the user copied files in Explorer.
#[must_use]
pub fn read_clipboard_file_paths() -> Vec<PathBuf> {
    read_clipboard_file_paths_impl()
}

#[cfg(windows)]
const CF_HDROP: u32 = 15;

#[cfg(windows)]
fn read_clipboard_file_paths_impl() -> Vec<PathBuf> {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::System::DataExchange::{
        CloseClipboard, GetClipboardData, IsClipboardFormatAvailable, OpenClipboard,
    };
    use windows::Win32::UI::Shell::{DragQueryFileW, HDROP};

    unsafe {
        if IsClipboardFormatAvailable(CF_HDROP).is_err() {
            return Vec::new();
        }
        if OpenClipboard(HWND::default()).is_err() {
            return Vec::new();
        }

        let paths = (|| {
            let handle = GetClipboardData(CF_HDROP).ok()?;
            let hdrop = HDROP(handle.0);
            let count = DragQueryFileW(hdrop, u32::MAX, None);
            let mut paths = Vec::with_capacity(count as usize);
            for index in 0..count {
                let char_count = DragQueryFileW(hdrop, index, None);
                let mut buffer = vec![0u16; char_count as usize + 1];
                DragQueryFileW(hdrop, index, Some(&mut buffer));
                let path = String::from_utf16_lossy(&buffer[..char_count as usize]);
                paths.push(PathBuf::from(path));
            }
            Some(paths)
        })();

        let _ = CloseClipboard();
        paths.unwrap_or_default()
    }
}

#[cfg(not(windows))]
fn read_clipboard_file_paths_impl() -> Vec<PathBuf> {
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_clipboard_file_paths_returns_vec_on_all_platforms() {
        let _paths = read_clipboard_file_paths();
    }
}
