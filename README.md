# VisioFlow

[![Build](https://github.com/zunaidFarouque/VisioFlow-QR/actions/workflows/build.yml/badge.svg)](https://github.com/zunaidFarouque/VisioFlow-QR/actions/workflows/build.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://github.com/zunaidFarouque/VisioFlow-QR/blob/main/Cargo.toml)

**Optical automation engine** — capture QR payloads from your screen or webcam, route them through rules, and trigger desktop actions (open URLs, WiFi handoff, run scripts, copy to clipboard).

VisioFlow is a **visual payload router**, not just a QR scanner. Scan once; the right action runs automatically.

| Platform | Capture | Routing / daemon |
|----------|---------|------------------|
| **Windows** | Snip + webcam (full build) | Full feature set |
| **Linux** | Snip (router-only build) | Rules, export, daemon |

macOS is out of scope.

---

## Features

- **Screen snip** and **webcam** QR decode (OpenCV + WeChat CNN on Windows full builds)
- **Auto-routing** — stock rules match URLs, WiFi, mailto, tel, geo, vCard, MATMSG, clipboard prefixes, and more
- **Rule engine** — regex capture groups, native protocol parsers (`QR_NATIVE_*`), child-process exec scripts
- **Windows toasts** — routing feedback on by default; optional **Copy** button via `visioflow-toast.exe`
- **WiFi handoff** — opens Settings with SSID/password helpers (not silent auto-join by default)
- **Daemon + IPC** — background rule server over named pipes (Windows) or Unix sockets (Linux)
- **Shell export** — `--export bash|ps1` for parent-session variable injection

<!-- Screenshots: add captures of snip flow, webcam preview, and a routing toast here -->

---

## Quick start (Windows)

### Install

Pick one path (details in [Getting Started](DOCs/Getting-Started.md) and [Distribution](DOCs/Distribution-Windows.md)):

1. **Scoop** — [Zunaid-Scoop-Bucket](https://github.com/zunaidFarouque/Zunaid-Scoop-Bucket): `scoop bucket add zunaid-scoop-bucket https://github.com/zunaidFarouque/Zunaid-Scoop-Bucket` then `scoop install zunaid-scoop-bucket/visioflow` (bootstrap is automatic)
2. **Traditional** — `scripts/install-traditional.ps1`
3. **Portable zip** — extract release zip, run `bootstrap-portable.ps1`

### First scan

```powershell
# Seed stock routing rules (one time)
visioflow rule init-defaults

# Snip a QR — auto-routes (URL opens browser, unknown text copies to clipboard)
visioflow capture --source snip
```

```powershell
# Webcam (full build; source dev-env first)
. .\scripts\dev-env.ps1
visioflow capture --source webcam --timeout 30
```

### Build from source

```powershell
# Router-only (snip + rules, no OpenCV)
cargo build --release -p visioflow-cli --no-default-features

# Full Windows build (webcam)
. .\scripts\dev-env.ps1
cargo build --release -p visioflow-cli
```

Release zip: `.\scripts\build-release.ps1` → `dist/visioflow-win-x64.zip`

---

## Documentation

Full docs live in **[`DOCs/`](DOCs/README.md)**:

| Guide | Topic |
|-------|--------|
| [Getting Started](DOCs/Getting-Started.md) | Install, first scan, shortcuts |
| [Capture](DOCs/Capture.md) | Snip/webcam, flags, mirroring, halts |
| [Routing & Default Rules](DOCs/Routing-And-Default-Rules.md) | Auto-route, builtins, stock rules |
| [Rules CLI](DOCs/Rules-CLI.md) | Rule CRUD, `init-defaults`, execute |
| [Notifications (Windows)](DOCs/Notifications-Windows.md) | Toasts, Copy button, troubleshooting |
| [Daemon & IPC](DOCs/Daemon-and-IPC.md) | Background service, socket protocol |
| [Distribution (Windows)](DOCs/Distribution-Windows.md) | Scoop, zip, publish checklist |
| [Architecture](DOCs/Architecture.md) | CLI shape, TDD, engine design |

Legacy index: [USER_GUIDE.md](DOCs/USER_GUIDE.md) (links to the pages above).

---

## Repository

- **GitHub:** [zunaidFarouque/VisioFlow-QR](https://github.com/zunaidFarouque/VisioFlow-QR)
- **License:** MIT (see workspace `Cargo.toml`)

## Development

```powershell
cargo test
cargo clippy -- -D warnings
.\scripts\smoke-router.ps1
.\scripts\smoke-distribution.ps1
```
