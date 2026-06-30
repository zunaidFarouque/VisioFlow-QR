# Quick Start

Get from install to a working snip scan in minutes on **Windows**.

---

## 1. Install stock rules

```powershell
visioflow rule init-defaults
```

This loads URL, WiFi, mailto, tel, geo, vCard, clipboard, MATMSG, and catch-all rules from the stock pack. See [[Default-Rules]] for the full table.

| Flag | Behavior |
|------|----------|
| *(none)* | Upsert all stock rules (overwrites same names) |
| `--merge` | Add missing rules only; keep your edits |
| `--force` | Replace entire store with stock defaults |

---

## 2. Snip and auto-route

```powershell
visioflow capture --source snip
```

1. Select a screen region containing a QR code.
2. VisioFlow decodes the payload and **auto-routes** (no `--trigger` needed).
3. On match: runs the rule action (e.g. open browser for `https://…`).
4. On no match: **copies** the raw text to the clipboard and shows a toast (notifications on by default).

---

## 3. Explicit rule or copy-only

```powershell
# Corporate asset tag (explicit-only rule)
visioflow capture --source snip --trigger asset

# Never run rules — copy only
visioflow capture --source snip --trigger copy

# Debug: print payload to stdout
visioflow capture --source snip --trigger plain --action stdout
```

---

## 4. Webcam (full build)

```powershell
. .\scripts\dev-env.ps1
visioflow capture --source webcam --timeout 30
```

Preview is **mirrored by default** (selfie-style). Use `--no-mirror` for raw camera orientation. Details: [[Capture]].

---

## 5. Test notifications

```powershell
visioflow notify test
visioflow notify test --title "VisioFlow" --body "Hello" --verbose
```

See [[Notifications]] for troubleshooting.

---

## Shortcuts (Windows)

**Scoop:** Start Menu shortcuts under **Scoop Apps → VisioFlow** (four entries — no desktop shortcuts).

**Traditional / portable / dev:**

```powershell
.\scripts\install-shortcuts.ps1
```

Creates under `%APPDATA%\VisioFlow\launchers\`:

| Launcher | Command |
|----------|---------|
| `camera-auto.cmd` | `capture --source webcam` (auto-route) |
| `camera-copy.cmd` | `capture --source webcam --trigger copy` |
| `snip-auto.cmd` | `capture --source snip` (auto-route) |
| `snip-copy.cmd` | `capture --source snip --trigger copy` |

Start Menu shortcuts: `%APPDATA%\Microsoft\Windows\Start Menu\Programs\VisioFlow\`.

Bind hotkeys in AutoHotkey or PowerToys to the `.cmd` launchers.

```powershell
.\scripts\install-shortcuts.ps1 -BinPath .\target\release\visioflow.exe -Force
```

---

## Learn more

| Topic | Page |
|-------|------|
| All capture flags | [[Capture]] |
| Auto-routing | [[Routing-and-Auto-Route]] |
| Custom rules | [[Custom-Rules]] |
| Windows toasts | [[Notifications]] |
| Background daemon | [[Daemon-and-IPC]] |
