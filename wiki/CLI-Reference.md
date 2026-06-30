# CLI Reference

Complete reference for `visioflow` and all subcommands. Run `visioflow --help` or `visioflow <subcommand> --help` for the installed version.

```text
visioflow [GLOBAL FLAGS] <command>
```

---

## Global flags

| Flag | Short | Default | Description |
|------|-------|---------|-------------|
| `--output <plain\|json>` | | `plain` | Output format for resolved variables and structured data |
| `--verbose` | `-v` | off | Diagnostics on stderr |
| `--silent` | | off | Suppress stdout |
| `--export <bash\|ps1>` | | — | Print parent-shell assignment lines |
| `--ipc-socket <PATH>` | | — | Delegate rule execution to daemon |

Environment variable: `VISIOFLOW_IPC_SOCKET` (same as `--ipc-socket`).

---

## capture

Capture and decode visual payloads.

```text
visioflow capture --source <snip|webcam> [options]
```

### Required

| Flag | Values | Description |
|------|--------|-------------|
| `--source` | `snip`, `webcam` | Capture source |

### Decode and output

| Flag | Default | Description |
|------|---------|-------------|
| `--filter` | `otsu` | `otsu` \| `median` — binarization pipeline |
| `--action` | — | `stdout` \| `copy` — bypass routing (scripting) |

### Webcam only

| Flag | Default | Description |
|------|---------|-------------|
| `--timeout` | `20` | Seconds to scan with live preview |
| `--preview-position` | `bottom-center` | `top-left`, `top-center`, `top-right`, `center-left`, `center`, `center-right`, `bottom-left`, `bottom-center`, `bottom-right` |
| `--preview-scale` | `0.12` | Preview size as fraction of screen height |
| `--exposure-step-ms` | `100` | Hold time per exposure bracket step |
| `--exposure-flush-grabs` | `2` | Frames to discard after exposure change |
| `--decode-interval-ms` | `100` | Interval between QR decode attempts |
| `--exposure-bracket` | `auto` | `auto` \| `on` \| `off` |
| `--no-mirror` | off | Disable horizontal mirroring (mirrored by default) |

### Routing

| Flag | Default | Description |
|------|---------|-------------|
| `--trigger <NAME>` | — | Explicit rule or builtin `copy` / `plain`; omit for auto-route |
| `--except <NAME>` | — | Exclude rule(s) from auto scan (repeatable) |
| `--only <NAME>` | — | Whitelist for auto scan (repeatable) |
| `--on-mismatch` | `copy` | `copy` \| `none` — fallback on routing failure |
| `--wifi-handoff` | `open-settings` | `open-settings` \| `print` — WiFi QR action mode |
| `--no-notify` | off | Disable desktop toasts (**on by default**) |

### Halts

| Flag | Description |
|------|-------------|
| `--select` | Pick one payload when multiple decoded |
| `--interactive` | Confirm payload on stdin before routing |

See [[Capture]] for behavior details.

---

## rule

Manage routing rules.

```text
visioflow rule <subcommand>
```

### rule create

```text
visioflow rule create <NAME>
```

Create an empty rule. Reserved names: `copy`, `plain`, `auto`.

### rule config

```text
visioflow rule config <NAME> [--regex <PAT>] [--map GROUP:VAR ...]
```

Set regex and capture group mappings. `--map` is repeatable.

### rule set-action

```text
visioflow rule set-action <NAME> [--exec <PATH>] [--wifi-connect]
```

Attach exec script path and/or enable OS WiFi connect from `QR_NATIVE_WIFI_*`.

### rule execute

```text
visioflow rule execute <NAME> --payload <STR> [--no-exec]
```

Apply rule to payload. Prints `QR_RAW`, `QR_NATIVE_*`, `QR_VAR_*`. Spawns exec unless `--no-exec`.

### rule list

```text
visioflow rule list
```

List all rules. Use global `--output json` for full rule objects.

### rule delete

```text
visioflow rule delete <NAME>
```

Remove a rule from the store.

### rule init-defaults

```text
visioflow rule init-defaults [--merge] [--force]
```

| Flag | Description |
|------|-------------|
| `--merge` | Add missing stock rules only |
| `--force` | Replace entire store with stock defaults |

See [[Custom-Rules]] and [[Default-Rules]].

---

## notify

Desktop notification utilities (Windows).

```text
visioflow notify <subcommand>
```

### notify test

```text
visioflow notify test [--title <STR>] [--body <STR>] [--backend <BACKEND>]
```

| Flag | Default | Description |
|------|---------|-------------|
| `--title` | `VisioFlow` | Toast title |
| `--body` | `Toast delivery smoke test` | Toast body |
| `--backend` | — | Force `winrt`, `powershell`, or `burnttoast` |

### notify copy (hidden)

```text
visioflow notify copy --from-toast <PATH>
```

Internal entry used by toast Copy button protocol activation.

See [[Notifications]].

---

## daemon

Background routing daemon.

```text
visioflow daemon [--socket <PATH>] <subcommand>
```

| Flag | Description |
|------|-------------|
| `--socket` | IPC socket path (default: `\\.\pipe\visioflow.sock` on Windows, `/tmp/visioflow.sock` on Linux) |

### daemon start

```text
visioflow daemon start [--hidden]
```

| Flag | Description |
|------|-------------|
| `--hidden` | Run detached in background (default: foreground) |

### daemon stop

```text
visioflow daemon stop
```

Stop the running daemon.

### daemon status

```text
visioflow daemon status
```

Show PID and socket health.

### daemon reload

```text
visioflow daemon reload
```

Re-read `rules.json` from disk into the daemon.

See [[Daemon-and-IPC]].

---

## Exit codes (capture)

| Code | Meaning |
|------|---------|
| `0` | Success — routing completed or copy fallback |
| `1` | Routing failed with `--on-mismatch none`, or `--interactive` cancelled |
| `2` | Decode failure (no payload) |

---

## Config paths

| Item | Windows | Linux |
|------|---------|-------|
| Rules store | `%APPDATA%\visioflow\rules.json` | `~/.config/visioflow/rules.json` |
| Daemon PID | `daemon.pid` next to rules file | same |
| Default IPC socket | `\\.\pipe\visioflow.sock` | `/tmp/visioflow.sock` |
