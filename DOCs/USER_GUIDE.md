# VisioFlow User Guide (index)

> **This page is an index.** Detailed guides moved to topic pages in this folder — see [README.md](README.md).

Practical kickstart for developers and sysadmins on **Windows** and **Linux**. macOS is out of scope.

---

## What VisioFlow does

```
capture (snip / webcam)
    → decode payload
    → match rule (auto or --trigger)
    → QR_RAW / QR_NATIVE_* / QR_VAR_* env vars
    → optional exec script (child process)
    → optional --export bash|ps1 (parent shell)
```

Typical uses: URLs, WiFi provisioning, mailto/MATMSG, asset tags, custom automation.

---

## Documentation map

| I want to… | Read |
|------------|------|
| Install and first scan | [Getting-Started.md](Getting-Started.md) |
| Capture flags, webcam, mirroring | [Capture.md](Capture.md) |
| Auto-routing and stock rules | [Routing-And-Default-Rules.md](Routing-And-Default-Rules.md) |
| Rule commands and `rules.json` | [Rules-CLI.md](Rules-CLI.md) |
| Windows toasts and Copy button | [Notifications-Windows.md](Notifications-Windows.md) |
| Daemon and IPC | [Daemon-and-IPC.md](Daemon-and-IPC.md) |
| Scoop, zip, releases | [Distribution-Windows.md](Distribution-Windows.md) |
| Architecture and TDD | [Architecture.md](Architecture.md) |
| Engine variables and security | [ENGINE_RULES.md](ENGINE_RULES.md) |
| IPC wire format | [IPC_PROTOCOL.md](IPC_PROTOCOL.md) |
| Implementation status | [Handoff-Router-Phase.md](Handoff-Router-Phase.md) |
| Webcam / OpenCV pipeline | [Rust OpenCV QR Scanning Architecture.md](Rust%20OpenCV%20QR%20Scanning%20Architecture.md) |
| CI conventions | [PLATFORM_CI.md](PLATFORM_CI.md) |

---

## Quick reference

### Install rules and scan

```powershell
visioflow rule init-defaults
visioflow capture --source snip
```

### Disable toasts

```powershell
visioflow capture --source snip --no-notify
```

### Parent shell (PowerShell)

```powershell
Invoke-Expression (visioflow --export ps1 rule execute asset --payload "ASSET:42")
```

### Daemon

```powershell
visioflow daemon start --hidden
visioflow --ipc-socket "\\.\pipe\visioflow.sock" capture --source snip
visioflow daemon reload
visioflow daemon stop
```

### Config paths

| OS | Rules |
|----|-------|
| Windows | `%APPDATA%\visioflow\rules.json` |
| Linux | `~/.config/visioflow/rules.json` |

---

## Smoke scripts

```powershell
.\scripts\smoke-router.ps1
.\scripts\smoke-default-rules.ps1
.\scripts\smoke-shortcuts.ps1
.\scripts\smoke-notify.ps1
.\scripts\smoke-distribution.ps1
```

---

## Repo entry point

[README.md](../README.md) — project overview, badges, links to this folder.
