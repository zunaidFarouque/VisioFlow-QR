# VisioFlow: Cross-Platform & CI/CD Doctrine

## 1. The Platform Abstraction Pattern
VisioFlow targets Windows and Linux, but the developer is building exclusively on Windows. Cursor MUST strictly isolate all OS-specific API calls using Rust's conditional compilation attributes.

### The `OsCommandRunner` Trait
Never call OS-specific binaries (like `netsh` or `nmcli`) directly in the core logic. 
1. Define a platform-agnostic trait in `src/sys/mod.rs` (e.g., `pub trait SystemExecutor { fn connect_wifi(&self, ssid: &str, pass: &str) -> Result<()>; }`).
2. Implement the Windows logic in `src/sys/windows.rs` protected by `#[cfg(target_os = "windows")]`.
3. Implement the Linux logic in `src/sys/linux.rs` protected by `#[cfg(target_os = "linux")]`.

### Cargo.toml Target Dependencies
Do not bloat the Windows binary with Linux dependencies, and vice versa. Use target-specific dependencies in `Cargo.toml`:
```toml
[target.'cfg(windows)'.dependencies]
winapi = "0.3"

[target.'cfg(unix)'.dependencies]
nix = "0.27"

```

## 2. IPC Implementation (The Nervous System)

The CLI and the Background Daemon must communicate instantly via local sockets. Do not write custom raw socket implementations; use the `interprocess` crate to handle the cross-platform heavy lifting.

* **Implementation Rule:** Use `interprocess::local_socket`.
* **Windows Behavior:** This crate will automatically map to Named Pipes (`\\.\pipe\visioflow.sock`).
* **Linux Behavior:** This crate will automatically map to Unix Domain Sockets (`/tmp/visioflow.sock`).
* **Protocol:** Communication over the socket MUST be strictly framed. Use newline-delimited JSON or a length-prefixed binary format so the daemon knows exactly when a payload ends.

## 3. The Global Keyboard Hook

Capturing hotkeys while the app is in the background is highly OS-dependent.

* **Windows:** Utilize `RegisterHotKey` via the `windows-rs` or `winapi` crate.
* **Linux:** Linux desktop environments (Wayland vs. X11) are notoriously fragmented for global hotkeys. For the initial implementation, rely on the desktop environment's native shortcut manager to trigger the ephemeral CLI (e.g., binding `Ctrl+Shift+Q` to run a VisioFlow **`.vbs`** launcher or `visioflow capture --source snip` in the OS settings), rather than trying to build a universal Wayland hook, which is a massive scope creep.

## 4. CI/CD & Testing Pipeline

Since the host environment is strictly Windows, the following pipeline must be adhered to:

### Phase 1: Local TDD (Windows)

All unit tests and core engine logic (regex, image processing math, pipeline routing) must pass natively on Windows. Full webcam / OpenCV builds are **local or release packaging** (`scripts/dev-env.ps1`, `scripts/build-release.ps1`) — not required on CI.

### Phase 2: WSL2 Integration (Linux)

Cursor may provide Linux-specific integration tests that the developer can run manually via the WSL2 terminal.

* Command to test Linux build from Windows: `cargo test --target x86_64-unknown-linux-gnu` (Requires setting up the correct cross-compilation toolchain, or simply running `cargo test` inside the WSL2 bash prompt).

### Phase 3: GitHub Actions (current)

Do not merge any PR or finalize a feature until it compiles cleanly in CI. The live workflow is [`.github/workflows/build.yml`](../.github/workflows/build.yml):

1. **Windows only** (`windows-latest`) — Linux/Ubuntu GA runners are **deferred** (OpenCV/contrib setup on hosted runners is unreliable).
2. Runs `cargo clippy --workspace --no-default-features -- -D warnings`.
3. Runs `cargo test --workspace --no-default-features` (router-only; no OpenCV).
4. Builds release binaries with `--no-default-features` (`visioflow` + `visioflow-toast`).

Full webcam builds ship via local release packaging, not CI artifacts.
