# Installation

Windows-first install guide. For Linux router-only usage, build with `--no-default-features` (see [Build from source](#build-from-source)).

---

## Prerequisites

| Goal | Windows | Linux |
|------|---------|-------|
| Snip + rules + daemon | [Rust toolchain](https://rustup.rs/) | Rust toolchain |
| Webcam capture | Release zip / Scoop install includes bundled `models/` (WeChat CNN). Dev builds: LLVM + [vcpkg](https://vcpkg.io/) OpenCV (`scripts/dev-env.ps1`) | `libopencv-contrib-dev`, `clang`, WeChat models in `models/` |

---

## Option 1: Scoop (recommended)

Bucket: [Zunaid-Scoop-Bucket](https://github.com/zunaidFarouque/Zunaid-Scoop-Bucket). Manifest source in app repo: `scripts/packaging/scoop/visioflow.json`.

`scoop install` runs bootstrap automatically — shortcuts, rules seed, and sync to `%APPDATA%\visioflow\rules.json`. The release zip includes **`models/`** beside `visioflow.exe` for webcam (no extra download). Scoop sets `VISIOFLOW_MODELS_DIR` to `$dir\models`.

```powershell
scoop bucket add Zuanid-Scoop https://github.com/zunaidFarouque/Zunaid-Scoop-Bucket
scoop install Zuanid-Scoop/visioflow
```

**Uninstall:** removes Desktop/Start Menu shortcuts and `%APPDATA%\VisioFlow\launchers`. Rules stay in Scoop persist unless you run `scoop uninstall -p visioflow`. The `visioflow:` toast protocol registry in HKCU is left in place (refreshed on the next `visioflow notify test`).

---

## Option 2: Traditional machine-local install

Copies binaries and `share/` to `%LOCALAPPDATA%\Programs\VisioFlow`, seeds `%APPDATA%\visioflow\rules.json`, and creates Desktop and Start Menu shortcuts.

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\install-traditional.ps1 -DistRoot .\dist\visioflow-win-x64 -Force
```

---

## Option 3: Portable zip (no package manager)

Download or build `visioflow-win-x64.zip`, extract anywhere, then bootstrap:

```powershell
cd D:\tools\visioflow-win-x64
powershell -ExecutionPolicy Bypass -File .\bootstrap-portable.ps1 -DistRoot . -Force
```

**Official release:** [v0.1.2 — visioflow-win-x64.zip](https://github.com/zunaidFarouque/VisioFlow-QR/releases/download/v0.1.2/visioflow-win-x64.zip)

See [[Distribution-and-Release]] for building and publishing release zips.

---

## Smoke checks

After install, verify the CLI:

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

## Build from source

### Router-only (no webcam)

```powershell
cargo build --release -p visioflow-cli --no-default-features
```

### Full Windows build (webcam)

```powershell
. .\scripts\dev-env.ps1
cargo build --release -p visioflow-cli
```

### Release zip

```powershell
.\scripts\build-release.ps1
```

Output: `dist/visioflow-win-x64.zip`

### Verify

```powershell
.\scripts\smoke-router.ps1
.\scripts\smoke-default-rules.ps1
.\scripts\smoke-distribution.ps1
```

---

## Next steps

→ [[Quick-Start]] — seed rules and run your first scan
