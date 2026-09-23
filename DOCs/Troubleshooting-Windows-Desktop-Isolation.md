# Troubleshooting: Preview Window Invisible on Windows (Virtual Desktop Isolation)

## Overview & Symptoms

When running `visioflow capture --webcam` or `visioflow encode --preview` in certain environments on Windows 11 (including integrated terminal sessions within Cursor / Antigravity IDE, background subshells, and process sandboxes):

- OpenCV initializes the camera stream and decodes QR codes normally.
- Win32 API calls (`CreateWindowExW`, `ShowWindow`, `GetWindowRect`) succeed without error.
- A valid window handle (`HWND`) is allocated at valid screen coordinates (e.g. `[182, 182, 598, 521]`).
- The rendering loop calls `window.update_with_buffer(...)` at 60 FPS without throwing any error.
- **Yet no window ever appears on the user's monitor.**

---

## Technical Root Cause Analysis

### Windows WindowStation and Desktop Architecture

In the Windows NT architecture, GUI subsystems are structured hierarchically:

1. **WindowStation**: A security container that contains a clipboard, atom table, and one or more desktops. The only interactive WindowStation capable of displaying UI to the logged-in user is `WinSta0`.
2. **Desktop**: A display surface contained inside a WindowStation. The desktop that is actively displayed on the physical monitor and connected to physical user input (keyboard/mouse) is named **`Default`** (or the Winlogon/UAC secure desktop when prompted).
3. **Desktop Window Manager (DWM)**: The Windows compositing engine only composites and presents the currently active input desktop (`Default`) to the GPU monitor.

```
Session 1 / 2 / 3 (Interactive User Session)
└── WindowStation: WinSta0 (Interactive)
    ├── Desktop: "Default"   <--- Composited by DWM to physical monitor!
    ├── Desktop: "Winlogon"  <--- Secure login/UAC screen
    └── Desktop: "exebox-*"  <--- Invisible sandbox virtual desktop!
```

### The Sandboxing Mechanism (`exebox-...`)

Modern agentic IDEs, containers, and development sandboxes (such as Cursor and Antigravity) execute terminal processes and background tasks inside an isolated virtual desktop named `exebox-<hash>` (e.g. `exebox-ARD5FUVDAD6I5ZLBWOG66NORNU`).

When `minifb` creates a window:
```c
CreateWindowExW(..., class_name, window_name, flags, ...)
```
Win32 assigns the new window handle to the **calling thread's assigned desktop** (`GetThreadDesktop(GetCurrentThreadId())`).

Because the calling thread was assigned to `exebox-...`:
- The window was created on `exebox-...`.
- GDI drew the frame buffer to `exebox-...`.
- The window was physically invisible to the user sitting in front of their monitor, because the monitor was displaying the `Default` desktop.

---

## The Solution: Dynamic Interactive Desktop Attachment

Before initializing any GUI window on Windows, VisioFlow checks the thread's current desktop affinity using the Win32 API.

If the calling thread is assigned to a non-interactive or sandboxed desktop (anything other than `Default`), VisioFlow dynamically switches the thread's desktop affinity to the active user display desktop:

```rust
// 1. Attempt to open the desktop currently receiving user input
let input_desk = OpenInputDesktop(0, 0, MAXIMUM_ALLOWED);
if !input_desk.is_null() {
    SetThreadDesktop(input_desk);
    CloseDesktop(input_desk);
    return;
}

// 2. Fall back to opening the standard "default" desktop on WinSta0
let default_name = to_wstring("default");
let default_desk = OpenDesktopW(default_name.as_ptr(), 0, 0, MAXIMUM_ALLOWED);
if !default_desk.is_null() {
    SetThreadDesktop(default_desk);
    CloseDesktop(default_desk);
}
```

### Implementation Details
- Handled automatically by `visioflow_cli::screen_bounds::ensure_interactive_desktop()`.
- Called prior to `minifb::Window::new` in both `webcam_session.rs` (live camera feed) and `qr_preview.rs` (QR encode preview).
- Non-Windows platforms (Linux, macOS) compile this function as a zero-overhead no-op.
- Fails gracefully without panics: if security policies disallow desktop switching, it logs the condition under `--verbose` and continues.

---

## Verifying the Fix

You can verify that a window can successfully bind to the interactive desktop by running the included diagnostic example:

```powershell
cargo run --example minifb_test -p visioflow-cli
```

Expected output:
```text
Process WindowStation: 'WinSta0'
[DESKTOP] Current Thread Desktop before switch: 'exebox-...'
[DESKTOP] Successfully switched via OpenInputDesktop to: 'Default'
[OK] Window::new with topmost: true succeeded.
HWND: 0x...
IsWindowVisible: 1
```
A 500×400 green preview window will immediately appear floating on your primary display.
