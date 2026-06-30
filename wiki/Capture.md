# Capture

The `capture` command decodes visual payloads from a **screen snip** or **webcam**, then optionally routes them through rules.

```text
visioflow capture --source <snip|webcam> [options]
```

When `--trigger` is omitted and `--action` is omitted, capture **auto-routes** after decode. See [[Routing-and-Auto-Route]].

---

## Sources

| `--source` | Description | Build |
|------------|-------------|-------|
| `snip` | Interactive screen region selection | Router-only OK |
| `webcam` | Live preview + decode loop | Requires `opencv-webcam` feature (default on Windows full build) |

### Snip examples

```powershell
# Auto-route (default UX after init-defaults)
visioflow capture --source snip

# Pipe raw payload (scripting)
visioflow capture --source snip --action stdout

# Copy without routing
visioflow capture --source snip --action copy
```

### Webcam examples

```powershell
. .\scripts\dev-env.ps1
visioflow capture --source webcam --timeout 30 --verbose
```

| Flag | Default | Purpose |
|------|---------|---------|
| `--timeout` | `20` | Seconds to scan with live preview |
| `--preview-position` | `bottom-center` | Preview window anchor |
| `--preview-scale` | `0.12` | Preview size as fraction of screen height |
| `--exposure-step-ms` | `100` | Hold time per exposure bracket step |
| `--exposure-flush-grabs` | `2` | Frames to discard after exposure change |
| `--decode-interval-ms` | `100` | Interval between QR decode attempts |
| `--exposure-bracket` | `auto` | `auto` \| `on` \| `off` — temporal exposure bracketing |

**Manual exposure:** focus the preview window and use **↑ / ↓** for ±0.5 EV sensor exposure. Useful for bright phone screens.

**Mirroring:** preview and decode are **horizontally mirrored by default** (selfie-style). Pass `--no-mirror` to use the raw camera orientation.

---

## Filters

| `--filter` | Pipeline |
|------------|----------|
| `otsu` | Otsu binarization (default) |
| `median` | Median blur then Otsu |

```powershell
visioflow capture --source snip --filter median
```

---

## Routing flags

Used when auto-routing is active (`--trigger` omitted, or `--action` omitted).

| Flag | Default | Purpose |
|------|---------|---------|
| `--trigger <NAME>` | *(omit = auto)* | Explicit rule, or builtin `copy` / `plain` |
| `--except <NAME>` | — | Exclude rule(s) from auto scan (repeatable) |
| `--only <NAME>` | — | Whitelist for auto scan (repeatable) |
| `--on-mismatch <copy\|none>` | `copy` | On routing failure: copy payload or exit strict |
| `--wifi-handoff <open-settings\|print>` | `open-settings` | WiFi QR: open Settings UI or print credentials |
| `--no-notify` | off | Disable Windows desktop toasts (**enabled by default**) |

```powershell
# Auto-route but never match the WiFi rule
visioflow capture --source snip --except wifi

# Strict: no clipboard fallback on mismatch
visioflow capture --source snip --trigger asset --on-mismatch none

# Quiet desktop toasts
visioflow capture --source snip --no-notify
```

Notifications: [[Notifications]].

---

## Output actions

| `--action` | Behavior |
|------------|----------|
| *(omit)* | Auto-route when defaults seeded; otherwise no stdout |
| `stdout` | Print decoded payload(s) |
| `copy` | Copy to clipboard (legacy; prefer `--trigger copy` with routing) |

With routing active, successful matches run rule actions; mismatch uses `--on-mismatch` (default: copy).

---

## Halts (multi-payload and confirm)

| Flag | When |
|------|------|
| `--select` | Multiple decoded payloads → numbered terminal picker |
| `--interactive` | Print payload and wait for `[y/N]` before routing |

```powershell
visioflow capture --source snip --select
visioflow capture --source snip --interactive --trigger wifi
```

Compose with auto-route or explicit `--trigger`.

---

## Global flags (with capture)

| Flag | Purpose |
|------|---------|
| `--verbose` | Decoded payloads and diagnostics on stderr |
| `--silent` | Suppress stdout |
| `--output plain\|json` | Format for resolved variables after routing |
| `--export bash\|ps1` | Parent-shell assignment lines |
| `--ipc-socket <PATH>` | Route via daemon (see [[Daemon-and-IPC]]) |

---

## Typical flows

### Daily snip (human)

```powershell
visioflow capture --source snip
```

Decode → auto-route → action or clipboard + toast.

### Scripting

```powershell
$payload = visioflow capture --source snip --action stdout
```

### Webcam + explicit WiFi rule

```powershell
visioflow capture --source webcam --trigger wifi --timeout 30 --verbose
```

WiFi stock rule runs `share/actions/wifi-handoff.ps1` (Settings handoff, not silent netsh join). Override with `--wifi-handoff print` to emit credentials on stdout.

---

## Troubleshooting

| Symptom | Fix |
|---------|-----|
| Webcam unavailable | Rebuild without `--no-default-features`; run `dev-env.ps1` on Windows |
| No auto-route | Run `visioflow rule init-defaults` |
| Regex mismatch | `visioflow capture --source snip --verbose`; test with `rule execute` |
| Toast missing | [[Notifications]] |

---

See also: [[CLI-Reference]] · [[Routing-and-Auto-Route]]
