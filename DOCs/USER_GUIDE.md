# VisioFlow User Guide

Practical kickstart for developers and sysadmins on **Windows** and **Linux**.  
macOS is **out of scope** for this project.

---

## What VisioFlow does

VisioFlow is a **visual payload router**: it reads QR (and related) data from the screen or a webcam, routes that text through named **rules**, and exposes the result as ephemeral environment variables. Optionally, it runs a script or executable with those variables set in the **child process only**.

```
capture (snip / webcam)
    → decode payload string
    → match rule (regex + native parsers)
    → QR_RAW / QR_NATIVE_* / QR_VAR_* env vars
    → optional --exec script (child process)
    → optional --export bash|ps1 (parent shell injection)
```

Typical uses: asset tags, WiFi provisioning payloads, URI deep-links, and custom automation keyed off QR content.

---

## Install and build

### Prerequisites

| Platform | Router-only (snip + rules) | Full build (webcam / OpenCV) |
|---|---|---|
| **Windows** | [Rust toolchain](https://rustup.rs/) | Above + LLVM + [vcpkg](https://vcpkg.io/) OpenCV (see `scripts/dev-env.ps1`) |
| **Linux** | Rust toolchain | Above + `libopencv-contrib-dev`, `clang`, WeChat CNN models in `models/` |

### Router-only build (no webcam)

Use this when you only need snip capture, rules, export, and the daemon. No OpenCV or vcpkg required.

```powershell
# Windows
cargo build --release -p visioflow-cli --no-default-features
```

```bash
# Linux
cargo build --release -p visioflow-cli --no-default-features
```

Binary: `target/release/visioflow` (`.exe` on Windows).

### Full build with webcam (Windows)

Source the dev environment **in each new terminal** before building or running webcam capture:

```powershell
. .\scripts\dev-env.ps1
cargo build --release -p visioflow-cli
```

`dev-env.ps1` sets `VCPKG_ROOT`, `VCPKGRS_TRIPLET`, and adds LLVM to `PATH`. Adjust paths inside the script if your vcpkg or LLVM install differs.

### Full build with webcam (Linux)

```bash
# Example on Ubuntu — package names may vary by distro
sudo apt install libopencv-contrib-dev clang
cargo build --release -p visioflow-cli
```

Download WeChat CNN model files into `models/` (see `models/README.md`).

### Verify the router stack

```powershell
.\scripts\smoke-router.ps1
```

This runs core/CLI tests, builds with `--no-default-features`, and exercises rule create/config/execute plus `--export bash`.

### Config file locations

Rules persist to a JSON store:

| OS | Default path |
|---|---|
| Windows | `%APPDATA%\visioflow\rules.json` |
| Linux | `~/.config/visioflow/rules.json` |

Daemon PID file: `daemon.pid` in the same directory as `rules.json`.

---

## Quick start

### 1. Decode a QR from a screen snip

```powershell
# Windows — plain text to stdout
cargo run -p visioflow-cli --no-default-features -- capture --source snip --action stdout
```

```bash
# Linux
cargo run -p visioflow-cli --no-default-features -- capture --source snip --action stdout
```

Select a screen region containing a QR code. The decoded string is printed (or copied with `--action copy`).

### 2. Create and test a routing rule

```powershell
visioflow rule create asset
visioflow rule config asset --regex "ASSET:(?P<asset>\d+)" --map asset:ASSET
visioflow rule execute asset --payload "ASSET:42"
```

Expected output:

```
QR_RAW=ASSET:42
QR_VAR_ASSET=42
```

- `--regex` must use **named capture groups** (`(?P<name>…)`).
- `--map asset:ASSET` maps capture group `asset` to env suffix `ASSET` → `QR_VAR_ASSET`.
- Omit `--map` to auto-uppercase the group name (`(?P<asset>…)` → `QR_VAR_ASSET`).

### 3. Inject variables into your **parent shell** (`--export`)

`--export` prints eval-safe assignment lines. Run VisioFlow in a **subshell** and apply the output in the parent.

**PowerShell**

```powershell
# Set vars in the current session
Invoke-Expression (visioflow --export ps1 rule execute asset --payload "ASSET:42")
$env:QR_VAR_ASSET   # 42
```

**bash**

```bash
eval "$(visioflow --export bash rule execute asset --payload 'ASSET:42')"
echo "$QR_VAR_ASSET"   # 42
```

After capture + trigger:

```powershell
Invoke-Expression (visioflow --export ps1 capture --source snip --action stdout --trigger asset)
```

```bash
eval "$(visioflow --export bash capture --source snip --action stdout --trigger asset)"
```

`--export ps1` emits lines like `$env:QR_VAR_ASSET = '42'`.  
`--export bash` emits `export QR_VAR_ASSET='42'`.

### 4. Attach a script to a rule

```powershell
visioflow rule set-action asset --exec C:\scripts\handle-asset.ps1
```

```bash
visioflow rule set-action asset --exec /usr/local/bin/handle-asset.sh
```

When you use `capture --trigger asset` (without `--ipc-socket`), VisioFlow runs that executable **after** routing, with all resolved variables in the child environment. The script file is never modified.

---

## Capture with `--trigger`

Apply a named rule immediately after decode:

```powershell
visioflow capture --source snip --action stdout --trigger asset --verbose
```

Flow:

1. Snip (or webcam) decodes one or more payloads.
2. The **first** payload is routed through rule `asset`.
3. Resolved variables are printed (or exported with `--export`).
4. If the rule has `--exec`, the script runs in a child process with env vars set.

Use `--verbose` to see decoded payload(s) on stderr before routing — essential when debugging regex mismatches.

**Webcam example (full build)**

```powershell
. .\scripts\dev-env.ps1
visioflow capture --source webcam --action stdout --trigger wifi --timeout 30 --verbose
```

### Multi-payload selection (`--select`)

When a frame decodes **more than one** payload, pass `--select` to pick which string to route:

```powershell
visioflow capture --source snip --action stdout --select --trigger asset
```

A numbered list appears in the terminal; enter the index of the payload you want.

### Confirmation gate (`--interactive`)

Require explicit approval before stdout/copy/trigger/exec:

```powershell
visioflow capture --source snip --action stdout --trigger asset --interactive
```

VisioFlow prints the payload and waits for `[y/N]` on stdin.

---

## Daemon and IPC

The background daemon keeps rules in memory and serves routing over a local socket. The CLI talks to it when you pass `--ipc-socket` (or set `VISIOFLOW_IPC_SOCKET`).

| Platform | Default socket |
|---|---|
| Windows | `\\.\pipe\visioflow.sock` |
| Linux | `/tmp/visioflow.sock` |

### Start, status, stop, reload

**Foreground** (blocks the terminal; good for debugging):

```powershell
visioflow daemon start
```

**Background**

```powershell
visioflow daemon start --hidden
```

```powershell
visioflow daemon status
visioflow daemon reload    # re-read rules.json from disk
visioflow daemon stop
```

Custom socket:

```powershell
visioflow daemon start --socket "\\.\pipe\my-visioflow.sock"
visioflow --ipc-socket "\\.\pipe\my-visioflow.sock" rule execute asset --payload "ASSET:1"
```

```bash
visioflow daemon start --socket /tmp/my-visioflow.sock
visioflow --ipc-socket /tmp/my-visioflow.sock rule execute asset --payload 'ASSET:1'
```

### CLI via daemon

When `--ipc-socket` is set, `rule execute` and `capture --trigger` delegate to the daemon (same semantics as local routing, including native parsers and optional exec). Reload the daemon after editing `rules.json` on disk:

```powershell
visioflow daemon reload
```

Wire format details: [`IPC_PROTOCOL.md`](IPC_PROTOCOL.md).

---

## Rule management reference

| Command | Purpose |
|---|---|
| `rule create <NAME>` | Create an empty rule |
| `rule config <NAME> --regex <PAT> [--map G:VAR ...]` | Set regex and optional capture mappings |
| `rule set-action <NAME> --exec <PATH>` | Script to run after successful route |
| `rule execute <NAME> --payload <STR> [--no-exec]` | Apply rule; spawns `--exec` unless `--no-exec` |

**Example: WiFi QR (native parsing on trigger / daemon)**

```powershell
visioflow rule create wifi
# No regex required — native parsers fill QR_NATIVE_WIFI_* on full route
visioflow capture --source snip --action stdout --trigger wifi
```

**Example: URI rule with regex**

```powershell
visioflow rule create uri
visioflow rule config uri --regex "^https://(?P<host>[^/]+)"
visioflow rule execute uri --payload "https://example.com/path"
```

Integration tests can pass `--store <path>` to use a temporary rules file (hidden in normal use).

---

## Variable namespaces

| Prefix | Source | Example |
|---|---|---|
| `QR_RAW` | Full decoded string | `ASSET:42` |
| `QR_NATIVE_*` | Built-in protocol parsers (WiFi, HTTP/HTTPS/FTP URI, …) | `QR_NATIVE_WIFI_SSID`, `QR_NATIVE_URI_HOST` |
| `QR_VAR_*` | Regex named capture groups via `--map` | `(?P<asset>\d+)` + `--map asset:ASSET` → `QR_VAR_ASSET` |

### Common `QR_NATIVE_*` keys

| Key | When set |
|---|---|
| `QR_NATIVE_WIFI_SSID` | Payload starts with `WIFI:` |
| `QR_NATIVE_WIFI_PASSWORD` | WiFi QR with `P:` field |
| `QR_NATIVE_WIFI_ENCRYPTION` | WiFi QR `T:` field (e.g. WPA) |
| `QR_NATIVE_WIFI_HIDDEN` | WiFi QR `H:` field |
| `QR_NATIVE_URI_SCHEME` | `http://`, `https://`, or `ftp://` |
| `QR_NATIVE_URI_HOST` | URI authority host |
| `QR_NATIVE_URI_PORT` | Explicit port in URI |
| `QR_NATIVE_URI_PATH` | Path component |
| `QR_NATIVE_MAIL_TO` | `mailto:` address |
| `QR_NATIVE_MAIL_SUBJECT` | `mailto:` subject query param |
| `QR_NATIVE_TEL_NUMBER` | `tel:` number |
| `QR_NATIVE_GEO_LAT` / `QR_NATIVE_GEO_LON` | `geo:` coordinates |
| `QR_NATIVE_VCARD_FN` | vCard full name (`FN:`) |
| `QR_NATIVE_VCARD_TEL` | Simple vCard `TEL:` line |

Native parsers run on **`rule execute`**, **`capture --trigger`**, and **daemon** routes.

---

## Global flags (short reference)

| Flag | Description |
|---|---|
| `--output plain\|json` | Format for resolved variables (default: `plain`) |
| `--verbose` | Diagnostics on stderr |
| `--silent` | Suppress stdout |
| `--export bash\|ps1` | Emit parent-shell assignments instead of plain output |
| `--ipc-socket <PATH>` | Talk to daemon (or `VISIOFLOW_IPC_SOCKET`) |

---

## Troubleshooting

### Regex did not match

The decoded QR text must match the rule’s `--regex` exactly (Rust regex syntax).

**Symptom**

```
regex did not match decoded payload "https://example.com"; rule 'asset' expects pattern: ASSET:(?P<asset>\d+)
```

**Fixes**

1. Run with `--verbose` and compare the decoded string to your pattern.
2. Ensure the QR encodes the format your rule expects (e.g. `ASSET:42`, not a bare URL).
3. Loosen or fix the regex; test offline:

   ```powershell
   visioflow rule execute asset --payload "ASSET:42"
   ```

4. For rules without regex, only `QR_RAW` (and native vars on trigger/daemon) are produced.

### Webcam build fails on Windows

Run `. .\scripts\dev-env.ps1` first. Confirm `VCPKG_ROOT` points at a vcpkg tree with OpenCV installed for `x64-windows-static-md`.

### Webcam unavailable in router-only binary

Rebuild **without** `--no-default-features`, or use `--source snip`.

### Daemon not running / stale PID

```powershell
visioflow daemon status
visioflow daemon stop
visioflow daemon start --hidden
```

### Automated smoke check

```powershell
.\scripts\smoke-router.ps1
```

Confirms tests, build, rule workflow, and `--export bash` in one pass.

---

## Security notes

1. **Child-process env only** — Variables are passed via `Command::env(...)`. They exist only for the spawned process and are reclaimed when it exits. VisioFlow does **not** modify your script files (no string substitution into `.ps1` / `.sh` sources).

2. **Parent shell injection** — Use `--export bash` or `--export ps1` only from trusted capture/rule output. Treat eval like running arbitrary code.

3. **Redaction** — Daemon logging redacts sensitive keys such as `QR_NATIVE_WIFI_PASSWORD` as `[REDACTED]`. Do not rely on `--verbose` in production for payloads containing secrets.

4. **Local IPC** — Sockets are local-only. On multi-user Linux hosts, avoid world-writable socket paths.

5. **Air-gapped hosts** — Set `VISIOFLOW_AIRGAP=1` or pass hidden `--disable-telemetry` to refuse startup (future OTLP guard). Unset before normal use.

---

## Further reading

| Document | Contents |
|---|---|
| [`Architecture.md`](Architecture.md) | CLI shape, TDD protocol, daemon design |
| [`ENGINE_RULES.md`](ENGINE_RULES.md) | Variable hierarchy, sandbox rules, optical pipeline |
| [`IPC_PROTOCOL.md`](IPC_PROTOCOL.md) | NDJSON messages, request/response types |
| [`Handoff-Router-Phase.md`](Handoff-Router-Phase.md) | Phase context and implementation status |
| [`Rust OpenCV QR Scanning Architecture.md`](Rust%20OpenCV%20QR%20Scanning%20Architecture.md) | Webcam / OpenCV pipeline (webcam only) |
| [`PLATFORM_CI.md`](PLATFORM_CI.md) | Cross-platform CI and IPC conventions |
| [`README.md`](../README.md) | Repo overview and webcam exposure keys |

---

## Command cheat sheet

```powershell
# Snip → stdout
visioflow capture --source snip --action stdout

# Rule lifecycle
visioflow rule create myrule
visioflow rule config myrule --regex "(?P<id>\w+)" --map id:ID
visioflow rule set-action myrule --exec C:\path\to\script.ps1
visioflow rule execute myrule --payload "test"

# Capture + route + export (PowerShell parent session)
Invoke-Expression (visioflow --export ps1 capture --source snip --action stdout --trigger myrule)

# Daemon
visioflow daemon start --hidden
visioflow --ipc-socket "\\.\pipe\visioflow.sock" capture --source snip --action stdout --trigger myrule
visioflow daemon reload
visioflow daemon stop
```

```bash
# bash parent session
eval "$(visioflow --export bash rule execute myrule --payload 'test')"

visioflow daemon start --hidden
visioflow --ipc-socket /tmp/visioflow.sock rule execute myrule --payload 'test'
```
