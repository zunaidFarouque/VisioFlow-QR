# Getting Started

VisioFlow captures QR payloads from the screen or webcam, matches them against routing rules, and runs the appropriate desktop action. It can also encode clipboard text into a QR (**Clipboard to QR**). This guide covers install, first scan, encode, and optional shortcuts on **Windows**.

For Linux router-only usage, skip webcam sections and build with `--no-default-features`.

---

## Prerequisites

| Goal | Windows | Linux |
|------|---------|-------|
| Snip + rules + daemon | [Rust toolchain](https://rustup.rs/) | Rust toolchain |
| Webcam capture | Release zip / Scoop install includes bundled `models/` (WeChat CNN). Dev builds: LLVM + [vcpkg](https://vcpkg.io/) OpenCV (`scripts/dev-env.ps1`) | `libopencv-contrib-dev`, `clang`, WeChat models in `models/` |

---

## Install (Windows)

Three supported paths — use one:

### 1. Scoop portable (recommended)

Bucket repo: [Zunaid-Scoop-Bucket](https://github.com/zunaidFarouque/Zunaid-Scoop-Bucket). Local Scoop name: **`Zuanid-Scoop`**. Manifest source: `scripts/packaging/scoop/visioflow.json`. Install runs bootstrap automatically.

```powershell
scoop bucket add Zuanid-Scoop https://github.com/zunaidFarouque/Zunaid-Scoop-Bucket
scoop install Zuanid-Scoop/visioflow
```

Webcam works out of the box: release zips bundle `models/` beside `visioflow.exe`. Scoop sets `VISIOFLOW_MODELS_DIR` to `$dir\models`.

**Start Menu:** five shortcuts under **Scoop Apps → VisioFlow** (no desktop shortcuts): Camera/Snip × auto/copy, plus **Clipboard to QR**. Each shortcut targets a hidden **`.vbs`** launcher under `$dir\launchers\` (no console flash).

### 2. Traditional machine-local install

Copies binaries and `share/` to `%LOCALAPPDATA%\Programs\VisioFlow`, seeds `%APPDATA%\visioflow\rules.json`, and creates **Start Menu** shortcuts under `Programs\VisioFlow` (no desktop shortcuts). Shortcuts target hidden **`.vbs`** launchers under `%APPDATA%\VisioFlow\launchers\`.

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\install-traditional.ps1 -DistRoot .\dist\visioflow-win-x64 -Force
```

### 3. Portable zip (no package manager)

Build or download `visioflow-win-x64.zip`, extract anywhere, then:

```powershell
cd D:\tools\visioflow-win-x64
powershell -ExecutionPolicy Bypass -File .\bootstrap-portable.ps1 -DistRoot . -Force
```

See [Distribution-Windows.md](Distribution-Windows.md) for `build-release.ps1` and publish steps.

### Smoke checks

```powershell
visioflow --help
visioflow rule list
visioflow notify test
```

---

## Config locations

| Item | Windows | Linux |
|------|---------|-------|
| Rules store | `%APPDATA%\visioflow\rules.json` | `~/.config/visioflow/rules.json` |
| Daemon PID | `daemon.pid` next to rules file | same |

---

## First scan

### 1. Install stock rules

```powershell
visioflow rule init-defaults
```

This loads URL, WiFi, mailto, tel, geo, vCard, clipboard, MATMSG, and catch-all rules from `assets/default-rules.json`. See [Routing-And-Default-Rules.md](Routing-And-Default-Rules.md) for the full table.

| Flag | Behavior |
|------|----------|
| *(none)* | Upsert all stock rules (overwrites same names) |
| `--merge` | Add missing rules only; keep your edits |
| `--force` | Replace entire store with stock defaults |

### 2. Snip and auto-route

```powershell
visioflow capture --source snip
```

- Select a screen region containing a QR code.
- VisioFlow decodes the payload and **auto-routes** (no `--trigger` needed).
- On match: runs the rule action (e.g. open browser for `https://…`).
- On no match: **copies** the raw text to the clipboard and shows a toast (notifications on by default).

### 3. Explicit rule or copy-only

```powershell
# Corporate asset tag (explicit-only rule)
visioflow capture --source snip --trigger asset

# Never run rules — copy only
visioflow capture --source snip --trigger copy

# Debug: print payload to stdout
visioflow capture --source snip --trigger plain --action stdout
```

### 4. Webcam (full build)

```powershell
. .\scripts\dev-env.ps1
visioflow capture --source webcam --timeout 30
```

Preview is **mirrored by default** (selfie-style). Use `--no-mirror` for raw camera orientation. Details: [Capture.md](Capture.md).

### 5. Clipboard to QR (`encode`)

```powershell
# Copy text or a URL (or a .url shortcut file), then:
visioflow encode --source clipboard
```

Default `--deliver preview` opens a square, non-resizable QR preview (Escape to dismiss). Other modes: `--deliver preview-copy` or `copy`. Details: [Encode.md](Encode.md).

Start Menu: **VisioFlow Clipboard to QR**.

---

## Shortcuts (Windows)

Launchers are **`.vbs`** scripts (run via `wscript`, window style hidden) so Start Menu / hotkey launches do not flash a console. Start Menu `.lnk` files point at those VBS files — not at `visioflow.exe` directly.

**Scoop:** five Start Menu entries under **Scoop Apps → VisioFlow** (no desktop shortcuts). Targets are bundled `$dir\launchers\*.vbs`.

**Traditional / portable / dev:** run `install-shortcuts.ps1` to create Start Menu shortcuts and launchers:

```powershell
.\scripts\install-shortcuts.ps1
```

Creates under `%APPDATA%\VisioFlow\launchers\`:

| Launcher | Command |
|----------|---------|
| `camera-auto.vbs` | `capture --source webcam` (auto-route) |
| `camera-copy.vbs` | `capture --source webcam --trigger copy` |
| `snip-auto.vbs` | `capture --source snip` (auto-route) |
| `snip-copy.vbs` | `capture --source snip --trigger copy` |
| `clipboard-qr.vbs` | `encode --source clipboard` (preview) |

Start Menu shortcuts live in `%APPDATA%\Microsoft\Windows\Start Menu\Programs\VisioFlow\` (including **VisioFlow Clipboard to QR**).

A separate registration shortcut **`VisioFlow.lnk`** (same folder) may appear after toast use — that is for AppUserModelID / toast Copy activation, not a user launcher. See [Notifications-Windows.md](Notifications-Windows.md).

Bind hotkeys in AutoHotkey or PowerToys to the **`.vbs`** launchers.

```powershell
.\scripts\install-shortcuts.ps1 -BinPath .\target\release\visioflow.exe -Force
```

---

## Build from source

Setting up a **new development machine**? See [Dev-Setup-New-PC.md](Dev-Setup-New-PC.md) for the full checklist (Rust, vcpkg, LLVM, smoke tests, migration).

### Router-only (no webcam)

```powershell
cargo build --release -p visioflow-cli --no-default-features
```

### Full Windows build

```powershell
. .\scripts\dev-env.ps1
cargo build --release -p visioflow-cli
```

### Verify

```powershell
.\scripts\smoke-router.ps1
.\scripts\smoke-default-rules.ps1
.\scripts\smoke-distribution.ps1
```

---

## Next steps

| Topic | Document |
|-------|----------|
| All capture flags | [Capture.md](Capture.md) |
| Clipboard to QR | [Encode.md](Encode.md) |
| Auto-routing and stock rules | [Routing-And-Default-Rules.md](Routing-And-Default-Rules.md) |
| Custom rules | [Rules-CLI.md](Rules-CLI.md) |
| Windows toasts | [Notifications-Windows.md](Notifications-Windows.md) |
| Background daemon | [Daemon-and-IPC.md](Daemon-and-IPC.md) |
| Parent shell variables | [Rules-CLI.md](Rules-CLI.md) § Export |
