# Installation

Windows-first install guide. For Linux router-only usage, build with `--no-default-features` (see [Build from source](#build-from-source)).

---

## Prerequisites

| Goal | Windows | Linux |
|------|---------|-------|
| Snip + rules + daemon | [Rust toolchain](https://rustup.rs/) | Rust toolchain |
| Webcam capture | Above + LLVM + [vcpkg](https://vcpkg.io/) OpenCV (`scripts/dev-env.ps1`) | `libopencv-contrib-dev`, `clang`, WeChat models in `models/` |

---

## Option 1: Scoop (recommended)

Manifest: `scripts/packaging/scoop/visioflow.json` in the [main repository](https://github.com/zunaidFarouque/VisioFlow-QR).

```powershell
scoop bucket add visioflow-bucket <your-bucket-url>
scoop install visioflow

# One-time bootstrap (shortcuts, rules seed)
powershell -ExecutionPolicy Bypass -File "$env:USERPROFILE\scoop\apps\visioflow\current\bootstrap-portable.ps1" -DistRoot "$env:USERPROFILE\scoop\apps\visioflow\current" -Force
```

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

**Official release:** [v0.1.0 — visioflow-win-x64.zip](https://github.com/zunaidFarouque/VisioFlow-QR/releases/download/v0.1.0/visioflow-win-x64.zip)

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
