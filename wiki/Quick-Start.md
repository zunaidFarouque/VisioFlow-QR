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

## 5. Clipboard to QR (`encode`)

```powershell
# Copy text or a URL (or a .url shortcut file), then:
visioflow encode --source clipboard
```

Default `--deliver preview` opens a square, non-resizable QR preview (Escape to dismiss). Other modes: `--deliver preview-copy` or `copy`. Details: [[Encode]].

Start Menu: **VisioFlow Clipboard to QR**.

---

## 6. Test notifications

```powershell
visioflow notify test
visioflow notify test --title "VisioFlow" --body "Hello" --verbose
```

See [[Notifications]] for troubleshooting.

---

## Shortcuts (Windows)

Launchers are **`.vbs`** scripts (run via `wscript`, window style hidden) so Start Menu / hotkey launches do not flash a console. Start Menu `.lnk` files point at those VBS files — not at `visioflow.exe` directly.

**Scoop:** five Start Menu entries under **Scoop Apps → VisioFlow** (no desktop shortcuts). Targets are bundled `$dir\launchers\*.vbs`.

**Traditional / portable / dev:**

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

Start Menu shortcuts: `%APPDATA%\Microsoft\Windows\Start Menu\Programs\VisioFlow\` (including **VisioFlow Clipboard to QR**).

A separate registration shortcut **`VisioFlow.lnk`** (same folder) may appear after toast use — that is for AppUserModelID / toast Copy activation, not a user launcher. See [[Notifications]].

Bind hotkeys in AutoHotkey or PowerToys to the **`.vbs`** launchers.

```powershell
.\scripts\install-shortcuts.ps1 -BinPath .\target\release\visioflow.exe -Force
```

---

## Learn more

| Topic | Page |
|-------|------|
| All capture flags | [[Capture]] |
| Clipboard to QR | [[Encode]] |
| Auto-routing | [[Routing-and-Auto-Route]] |
| Custom rules | [[Custom-Rules]] |
| Windows toasts | [[Notifications]] |
| Background daemon | [[Daemon-and-IPC]] |
