<p align="center">
  <img src="https://raw.githubusercontent.com/zunaidFarouque/VisioFlow-QR/main/assets/logo%20v2.ico" alt="VisioFlow" width="96">
</p>

# VisioFlow

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

---

## Documentation

### Getting started

| Page | Topic |
|------|-------|
| [[Installation]] | Scoop, traditional install, portable zip, build from source |
| [[Quick-Start]] | First scan, shortcuts, `init-defaults`, notify test |

### Usage

| Page | Topic |
|------|-------|
| [[Capture]] | Snip/webcam, flags, mirroring, halts, `--no-notify` |
| [[Routing-and-Auto-Route]] | Auto-route, builtins, mismatch copy fallback |
| [[Default-Rules]] | Stock rule pack table and examples |
| [[Custom-Rules]] | Rule CRUD, regex, captures, priority |
| [[Notifications]] | Windows toasts, Copy button, troubleshooting |

### Reference

| Page | Topic |
|------|-------|
| [[CLI-Reference]] | Every subcommand and flag |
| [[Daemon-and-IPC]] | Background service, socket protocol |
| [[Distribution-and-Release]] | `build-release.ps1`, Scoop manifest, publish checklist |

---

## Quick commands

```powershell
# One-time: install stock routing rules
visioflow rule init-defaults

# Daily use: snip and auto-route
visioflow capture --source snip

# Smoke test notifications
visioflow notify test
```

---

## Repository

- **GitHub:** [zunaidFarouque/VisioFlow-QR](https://github.com/zunaidFarouque/VisioFlow-QR)
- **Release:** [v0.1.5](https://github.com/zunaidFarouque/VisioFlow-QR/releases/tag/v0.1.5)
- **License:** MIT
