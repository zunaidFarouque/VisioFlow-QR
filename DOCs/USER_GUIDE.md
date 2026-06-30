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

### Recommended install methods (Windows-first)

Use one of these three paths depending on how you want to run VisioFlow:

1. **Scoop portable (recommended)**
2. **Traditional machine-local install**
3. **Zip/no-install portable**

#### 1) Scoop portable (recommended)

```powershell
# Add your bucket that contains scripts/packaging/scoop/visioflow.json
scoop bucket add visioflow-bucket <bucket-url>
scoop install visioflow

# One-time bootstrap for shortcuts/rules
powershell -ExecutionPolicy Bypass -File "$env:USERPROFILE\scoop\apps\visioflow\current\bootstrap-portable.ps1" -DistRoot "$env:USERPROFILE\scoop\apps\visioflow\current" -Force
```

Smoke checks:

```powershell
visioflow --help
visioflow rule list
```

#### 2) Traditional machine-local install

This copies `visioflow.exe` + `share/` to `%LOCALAPPDATA%\Programs\VisioFlow` by default, seeds `%APPDATA%\visioflow\rules.json`, then creates Desktop and Start Menu shortcuts.

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\install-traditional.ps1 -DistRoot .\dist\visioflow-win-x64 -Force
```

Smoke checks:

```powershell
& "$env:LOCALAPPDATA\Programs\VisioFlow\visioflow.exe" --help
powershell -ExecutionPolicy Bypass -File .\scripts\smoke-shortcuts.ps1
```

#### 3) Zip / no-install portable (no Scoop)

Extract the release zip anywhere (for example `D:\tools\visioflow-win-x64`) and bootstrap once:

```powershell
cd D:\tools\visioflow-win-x64
powershell -ExecutionPolicy Bypass -File .\bootstrap-portable.ps1 -DistRoot . -Force
```

Smoke checks:

```powershell
.\visioflow.exe --help
.\visioflow.exe rule list
```

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
.\scripts\smoke-default-rules.ps1
.\scripts\smoke-shortcuts.ps1
.\scripts\smoke-distribution.ps1
```

`smoke-router.ps1` runs core/CLI tests, builds with `--no-default-features`, and exercises rule create/config/execute plus `--export bash`.

`smoke-default-rules.ps1` seeds stock rules via `rule init-defaults` into a temp store, runs `rule execute url --no-exec`, and checks `rule list`.

`smoke-shortcuts.ps1` validates the Windows shortcut installer in temp directories (launcher `.cmd` files + Desktop/Start Menu `.lnk` files).

`smoke-distribution.ps1` validates distribution artifacts for Scoop/traditional/zip paths, including install bootstrap and expected file layout.

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

### 2b. Install stock default rules (`init-defaults`)

Shipped rules (URL, WiFi, mailto, tel, geo, vCard, clipboard prefix, catch-all `plain`, explicit-only `asset`) live in `assets/default-rules.json`. Install them into your store:

```powershell
visioflow rule init-defaults
```

| Flag | Behavior |
|---|---|
| *(none)* | Upsert all stock rules (overwrites same names) |
| `--merge` | Add missing stock rules only; keep your edits to existing names |
| `--force` | Replace the entire store with stock defaults |

Action scripts resolve from the binary directory, `VISIOFLOW_SHARE`, or the repo `share/` tree during dev. After install, snip capture **auto-routes** without `--trigger` (see § Capture routing).

Integration tests can pass `--store <path>` to use a temporary rules file (hidden in normal use).

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

### 5. Install double-click shortcuts (Windows)

Generate launchers and clickable shortcuts for common scan flows:

```powershell
.\scripts\install-shortcuts.ps1
```

This creates:

- `%APPDATA%\VisioFlow\launchers\scan-auto.cmd`
- `%APPDATA%\VisioFlow\launchers\scan-copy.cmd`
- `%APPDATA%\VisioFlow\launchers\scan-plain.cmd`
- Desktop shortcuts: `VisioFlow Scan (Auto|Copy|Plain)`
- Start Menu shortcuts under `Programs\VisioFlow`

Modes:

- `scan-auto` → `capture --source snip` (auto-route)
- `scan-copy` → `capture --source snip --trigger copy`
- `scan-plain` → `capture --source snip --trigger plain --action stdout`

Useful flags:

```powershell
# Choose a specific binary
.\scripts\install-shortcuts.ps1 -BinPath .\target\release\visioflow.exe

# Overwrite existing wrappers/shortcuts
.\scripts\install-shortcuts.ps1 -Force
```

You can bind hotkeys in AutoHotkey/PowerToys to the generated `.cmd` launchers.

---

## Capture routing (v2)

After `rule init-defaults`, omitting `--trigger` **auto-routes** the decoded payload: stock rules with `auto_compatible: true` are scanned by ascending `priority`; the first match wins and runs its actions (exec script and/or `--wifi-connect`).

```powershell
# Auto-route URL QRs (default snip UX — copies on no match)
visioflow capture --source snip

# Auto but never auto-join WiFi
visioflow capture --source snip --except wifi

# Corporate asset tag — explicit rule only (not in auto pool)
visioflow capture --source snip --trigger asset

# Mismatch still copies (default)
# stderr: visioflow: rule "asset" did not match; copied payload to clipboard

# Strict automation: no copy on mismatch
visioflow capture --source snip --trigger asset --on-mismatch none

# Never run any rule — copy only
visioflow capture --source snip --trigger copy

# Debug: print payload to stdout on purpose
visioflow capture --source snip --trigger plain --action stdout
```

| Flag | Default | Purpose |
|---|---|---|
| `--trigger <NAME>` | *(omit = auto)* | Explicit rule, or builtin `copy` / `plain` |
| `--except <NAME>` | — | Exclude rule(s) from auto scan (repeatable) |
| `--only <NAME>` | — | Whitelist for auto scan (repeatable) |
| `--on-mismatch <copy\|none>` | `copy` | After routing failure, copy payload or exit strict |
| `--notify <off\|on\|errors-only>` | `errors-only` | Native Windows toast notifications for routing outcomes |

`--notify` behavior:

- `errors-only` (default): toast on explicit mismatch, no auto match, and WiFi connect failure.
- `on`: toast all of the above plus successful rule matches.
- `off`: no desktop toast.

If the OS notification channel is unavailable, capture continues normally. In `--verbose` mode, VisioFlow prints a one-line stderr diagnostic instead of failing.

**Human-first default:** successful routing runs actions; failures **copy** the payload and print a stderr notice (unless `--silent`). Use `--action stdout` only for scripting — not the default snip experience.

Full routing spec: [`Routing-And-Default-Rules.md`](Routing-And-Default-Rules.md).

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

When `--ipc-socket` is set, `rule execute` and `capture --trigger` delegate to the daemon (same semantics as local routing, including native parsers and optional exec). For **auto-routing** (omitting `--trigger`), the CLI now resolves the matching rule locally with the same core routing API and then executes that matched rule through daemon IPC. On the wire, `execute_rule` still carries an explicit rule name. Reload the daemon after editing `rules.json` on disk:

```powershell
visioflow daemon reload
```

Wire format details: [`IPC_PROTOCOL.md`](IPC_PROTOCOL.md).

---

## Rule management reference

| Command | Purpose |
|---|---|
| `rule create <NAME>` | Create an empty rule |
| `rule list` | Print rule names (plain) or full rule objects (`--output json`) |
| `rule config <NAME> --regex <PAT> [--map G:VAR ...]` | Set regex and optional capture mappings |
| `rule set-action <NAME> [--exec <PATH>] [--wifi-connect]` | Script and/or OS WiFi connect after route |
| `rule execute <NAME> --payload <STR> [--no-exec]` | Apply rule; spawns `--exec` unless `--no-exec` |
| `rule init-defaults [--merge] [--force]` | Install stock rules from `assets/default-rules.json` |
| `rule delete <NAME>` | Remove a rule from the JSON store |

You can also edit `rules.json` directly (same schema as `rule list --output json`). After manual edits, run `visioflow daemon reload` if a daemon is running so it picks up changes from disk.

**Example: WiFi QR (native parsing + auto-connect)**

```powershell
visioflow rule create wifi
visioflow rule set-action wifi --wifi-connect
# Native parsers fill QR_NATIVE_WIFI_*; --wifi-connect runs OS connect (netsh / nmcli)
visioflow capture --source snip --action stdout --trigger wifi
```

**Example: URI rule with regex**

```powershell
visioflow rule create uri
visioflow rule config uri --regex "^https://(?P<host>[^/]+)"
visioflow rule execute uri --payload "https://example.com/path"
```

Integration tests can pass `--store <path>` to use a temporary rules file (hidden in normal use).

### Editing `rules.json` directly

Rules are stored as a single JSON object: **keys are rule names**, values are rule records. The CLI writes the same shape via `rule create` / `rule config` / `rule set-action`.

**Example** (`%APPDATA%\visioflow\rules.json` on Windows):

```json
{
  "asset": {
    "name": "asset",
    "regex": "ASSET:(?P<asset>\\d+)",
    "captures": {
      "asset": "ASSET"
    },
    "exec": "C:\\scripts\\handle-asset.ps1"
  },
  "wifi": {
    "name": "wifi"
  }
}
```

| Field | Required | Notes |
|---|---|---|
| `name` | yes | Must match the object key |
| `regex` | no | Rust regex; omit for native-parser-only rules (e.g. WiFi) |
| `captures` | no | Map capture group → env suffix (`asset` → `QR_VAR_ASSET`) |
| `exec` | no | Script path run after a successful route |
| `wifi_connect` | no | When true, OS WiFi join from `QR_NATIVE_WIFI_*` after route |
| `auto_compatible` | no | When true, rule participates in auto scan (default `false` for user rules) |
| `priority` | no | Auto-scan order; lower = tried first (default `100`) |

**Escaping**

- **Regex in JSON:** backslashes must be doubled (`\d` → `\\d`, `\w` → `\\w`).
- **Windows paths:** use doubled backslashes (`C:\\scripts\\foo.ps1`) or forward slashes (`C:/scripts/foo.ps1`).
- **Special characters in regex:** quote-sensitive chars still need JSON string escaping (e.g. `"` → `\"`).

After editing the file on disk, reload an in-memory daemon (or restart it):

```powershell
visioflow daemon reload
```

Without `--ipc-socket`, the CLI reads `rules.json` on each local `rule` / `capture --trigger` call — no reload needed.

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
.\scripts\smoke-default-rules.ps1
```

Confirms tests, build, rule workflow, `--export bash`, and stock default rules in one pass.

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
| [`Routing-And-Default-Rules.md`](Routing-And-Default-Rules.md) | v2 auto-routing, default rule pack, builtins, copy fallback |
| [`Handoff-Router-Phase.md`](Handoff-Router-Phase.md) | Phase context and implementation status |
| [`Rust OpenCV QR Scanning Architecture.md`](Rust%20OpenCV%20QR%20Scanning%20Architecture.md) | Webcam / OpenCV pipeline (webcam only) |
| [`PLATFORM_CI.md`](PLATFORM_CI.md) | Cross-platform CI and IPC conventions |
| [`Distribution-Windows.md`](Distribution-Windows.md) | Scoop/traditional/zip packaging and publish checklist |
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
