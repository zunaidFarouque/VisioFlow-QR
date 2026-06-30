# Getting Started

VisioFlow captures QR payloads from the screen or webcam, matches them against routing rules, and runs the appropriate desktop action. This guide covers install, first scan, and optional shortcuts on **Windows**.

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

### 2. Traditional machine-local install

Copies binaries and `share/` to `%LOCALAPPDATA%\Programs\VisioFlow`, seeds `%APPDATA%\visioflow\rules.json`, creates Desktop and Start Menu shortcuts.

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

---

## Shortcuts (Windows)

Generate double-click launchers and Desktop / Start Menu shortcuts:

```powershell
.\scripts\install-shortcuts.ps1
```

Creates under `%APPDATA%\VisioFlow\launchers\`:

| Launcher | Command |
|----------|---------|
| `scan-auto.cmd` | `capture --source snip` (auto-route) |
| `scan-copy.cmd` | `capture --source snip --trigger copy` |
| `scan-plain.cmd` | `capture --source snip --trigger plain --action stdout` |

Bind hotkeys in AutoHotkey or PowerToys to these `.cmd` files.

```powershell
.\scripts\install-shortcuts.ps1 -BinPath .\target\release\visioflow.exe -Force
```

---

## Build from source

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
| Auto-routing and stock rules | [Routing-And-Default-Rules.md](Routing-And-Default-Rules.md) |
| Custom rules | [Rules-CLI.md](Rules-CLI.md) |
| Windows toasts | [Notifications-Windows.md](Notifications-Windows.md) |
| Background daemon | [Daemon-and-IPC.md](Daemon-and-IPC.md) |
| Parent shell variables | [Rules-CLI.md](Rules-CLI.md) § Export |
