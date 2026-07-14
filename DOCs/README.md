# VisioFlow documentation

Windows-first guides for install, capture, routing, and distribution. Linux is supported for **router-only** builds (snip, rules, daemon, export).

## Start here

| Document | What you'll learn |
|----------|-------------------|
| [Getting Started](Getting-Started.md) | Install paths, `rule init-defaults`, first scan, encode, shortcuts |
| [Capture](Capture.md) | Snip/webcam, WeChat models, exposure bracket, filters (snip-only), halts |
| [Encode](Encode.md) | Clipboard to QR, `.url` shortcut extraction, preview window, image copy |
| [Routing & Default Rules](Routing-And-Default-Rules.md) | Auto-route, builtins, stock rule pack, mismatch copy fallback |

## Reference

| Document | What you'll learn |
|----------|-------------------|
| [Rules CLI](Rules-CLI.md) | `rule create/config/set-action/execute/list/delete/init-defaults` |
| [Notifications (Windows)](Notifications-Windows.md) | Toasts, `visioflow-toast.exe`, Copy button, `notify test` |
| [Daemon & IPC](Daemon-and-IPC.md) | `daemon start/stop/status/reload`, `--ipc-socket`, wire protocol |
| [Distribution (Windows)](Distribution-Windows.md) | `build-release.ps1`, Scoop manifest, release checklist |

## Engineering

| Document | What you'll learn |
|----------|-------------------|
| [Dev setup (new PC)](Dev-Setup-New-PC.md) | Clone repo, Rust/vcpkg/LLVM, verify builds, migrate from another machine |
| [Architecture](Architecture.md) | CLI noun-verb design, TDD protocol, constraints |
| [ENGINE_RULES.md](ENGINE_RULES.md) | `QR_RAW` / `QR_NATIVE_*` / `QR_VAR_*`, sandbox, redaction |
| [IPC_PROTOCOL.md](IPC_PROTOCOL.md) | NDJSON message shapes (technical reference) |
| [Handoff-Router-Phase.md](Handoff-Router-Phase.md) | Implementation status and handoff context |
| [Rust OpenCV QR Scanning Architecture.md](Rust%20OpenCV%20QR%20Scanning%20Architecture.md) | Webcam / OpenCV pipeline |
| [PLATFORM_CI.md](PLATFORM_CI.md) | Cross-platform CI and IPC conventions |

## Legacy

[USER_GUIDE.md](USER_GUIDE.md) — slim index; detailed content moved to the topic pages above.
