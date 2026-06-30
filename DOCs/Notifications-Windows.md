# Notifications (Windows)

VisioFlow shows **native Windows toasts** for routing outcomes. Notifications are **enabled by default** on capture; use `--no-notify` to disable.

Linux desktop notifications are not implemented in this release.

---

## Default behavior

| Setting | CLI | Effect |
|---------|-----|--------|
| On (default) | *(no flag)* | Toast on successful matches, mismatches, no auto match, and copy-only mode |
| Off | `--no-notify` | Stderr messages only (unless `--silent`) |

```powershell
# Default — toasts on
visioflow capture --source snip

# Disable toasts
visioflow capture --source snip --no-notify
```

Stderr always receives human-readable routing lines when not `--silent`, for example:

```text
visioflow: matched rule "url"
visioflow: rule "asset" did not match; copied payload to clipboard
visioflow: no auto rule matched; copied payload to clipboard
```

If the OS notification channel is unavailable, capture continues. Use `--verbose` for a one-line stderr diagnostic.

---

## Toast smoke test

No capture or webcam required:

```powershell
visioflow notify test
visioflow notify test --title "VisioFlow" --body "Hello" --verbose
```

Force a backend (diagnostics):

```powershell
visioflow notify test --backend winrt --verbose
visioflow notify test --backend powershell --verbose
visioflow notify test --backend burnttoast --verbose   # requires Install-Module BurntToast
```

Automated check:

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\smoke-notify.ps1
```

---

## Copy button and visioflow-toast.exe

Routing toasts and `notify test` include a **Copy** button (or **Copy again** when routing already copied the payload).

Clicking Copy:

1. Launches headless **`visioflow-toast.exe`** (Windows GUI subsystem — no console flash).
2. Invoked via the registered `visioflow:` protocol URL.
3. Reads the **full raw payload** from a temp file in `%TEMP%` (not the truncated toast body).
4. Copies it to the clipboard.

**Layout:** `visioflow-toast.exe` must sit beside `visioflow.exe` in release zips and installs. See [Distribution-Windows.md](Distribution-Windows.md).

On first toast send, VisioFlow registers:

- `visioflow:` protocol handler
- Toast activator CLSID on the Start Menu shortcut

AppUserModelID: `VisioFlow.VisioFlowQR` (Start Menu shortcut at `%APPDATA%\Microsoft\Windows\Start Menu\Programs\VisioFlow\VisioFlow.lnk`).

Hidden CLI entry (used by protocol activation):

```text
visioflow notify copy --from-toast <path>
```

---

## Troubleshooting

### Toast test exits 0 but no popup

1. **Settings → System → Notifications** — enabled globally and for **VisioFlow**.
2. **Focus Assist / Do Not Disturb** — can suppress toasts.
3. Re-run `visioflow notify test --verbose` for delivery hints.
4. Confirm Start Menu shortcut exists with correct AppUserModelID.

### Copy button does nothing

1. Verify `visioflow-toast.exe` next to `visioflow.exe`.
2. Rebuild or reinstall from a current release zip.
3. Run `notify test` once to register protocol handler.

### Notifications in shortcuts

Default launchers from `install-shortcuts.ps1` use auto-route capture **with** toasts. Add `--no-notify` to launcher `.cmd` files if you prefer silent desktop feedback.

---

## Related

- Capture flag reference: [Capture.md](Capture.md)
- Routing events that drive toast text: [Routing-And-Default-Rules.md](Routing-And-Default-Rules.md) § Notifications
