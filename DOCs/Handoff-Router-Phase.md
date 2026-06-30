# VisioFlow — Handoff: Visual Payload Router Phase

> **Purpose:** Handoff for maintainers and AI sessions. Router phase, v2 routing, Windows notifications, and distribution paths are **shipped**. Optional future work: dedicated IPC auto-route message, OTLP telemetry, Tauri daemon wrapper.

---

## 1. What this project is

**VisioFlow** captures QR payloads via webcam or screen snip, routes them through rules (regex, native parsers), maps data to ephemeral environment variables, and triggers OS actions.

It is **not** just a QR scanner.

### Authoritative docs

| Document | Governs |
|----------|---------|
| [`DOCs/README.md`](README.md) | Documentation index |
| [`Architecture.md`](Architecture.md) | CLI shape, TDD, constraints |
| [`ENGINE_RULES.md`](ENGINE_RULES.md) | Variable namespaces, sandbox |
| [`Routing-And-Default-Rules.md`](Routing-And-Default-Rules.md) | v2 auto-routing, default pack |
| [`Getting-Started.md`](Getting-Started.md) | Install and first scan |
| [`Distribution-Windows.md`](Distribution-Windows.md) | Release zip, Scoop, publish |

---

## 2. Repository layout

```
crates/visioflow-core/   # Engine: optical pipeline, rules, IPC, opencv_webcam (feature)
crates/visioflow-cli/    # Binary visioflow + visioflow-toast (Windows)
assets/default-rules.json
share/actions/           # Platform exec scripts
scripts/build-release.ps1
scripts/packaging/scoop/visioflow.json
scripts/dev-env.ps1      # Windows vcpkg + LLVM for webcam builds
```

**GitHub:** [zunaidFarouque/VisioFlow-QR](https://github.com/zunaidFarouque/VisioFlow-QR)

---

## 3. What works today (do not break)

| Feature | Status |
|---------|--------|
| `capture --source snip` | rqrr + Otsu/Median |
| `capture --source webcam` | OpenCV + WeChat CNN, minifb preview, exposure bracket |
| Auto-route (omit `--trigger`) | Stock rules by `priority` |
| Builtins `copy` / `plain` | Explicit escape hatches |
| `--except` / `--only` / `--on-mismatch` | Auto scan constraints and fallback |
| `rule init-defaults` | Stock pack incl. MATMSG (priority 21) |
| Windows toasts | **On by default**; `--no-notify` to disable |
| `visioflow-toast.exe` | Toast Copy button via `visioflow:` protocol |
| WiFi stock rule | `wifi-handoff.ps1` — Settings UI (not silent netsh) |
| `--wifi-handoff open-settings\|print` | Handoff mode env |
| Webcam `--no-mirror` | Mirrored preview/decode by default |
| `daemon` + IPC | Named pipes / UDS, `reload` |
| `--export bash\|ps1` | Parent shell injection |
| Distribution | Scoop, traditional, portable zip via `build-release.ps1` |

### Webcam (stable — do not refactor unless required)

```powershell
. .\scripts\dev-env.ps1
cargo run --release -p visioflow-cli -- capture --source webcam --timeout 30 --verbose
```

Details: [`Rust OpenCV QR Scanning Architecture.md`](Rust%20OpenCV%20QR%20Scanning%20Architecture.md).

---

## 4. Router phase status

### Done

| Item | Location / doc |
|------|----------------|
| Rule model + JSON store | `visioflow-core/src/rules/` |
| `rule` CLI | `commands/rule.rs` — [`Rules-CLI.md`](Rules-CLI.md) |
| v2 routing | `rules/auto.rs`, [`Routing-And-Default-Rules.md`](Routing-And-Default-Rules.md) |
| Default rule pack + MATMSG | `assets/default-rules.json`, `share/actions/open-matmsg.ps1` |
| Windows notifications | [`Notifications-Windows.md`](Notifications-Windows.md) |
| IPC + daemon | [`Daemon-and-IPC.md`](Daemon-and-IPC.md), `IPC_PROTOCOL.md` |
| Release packaging | [`Distribution-Windows.md`](Distribution-Windows.md) |

### Not built / deferred

| Item | Notes |
|------|-------|
| Tauri headless daemon | Pure-Rust daemon ships today |
| OTLP / network telemetry | Air-gap hook exists; no exporter |
| IPC `route_auto` message | Client resolves auto-route today |
| Linux desktop notifications | Windows only |

---

## 5. Target CLI shape (current)

### Global

`--output`, `--verbose`, `--silent`, `--export`, `--ipc-socket`

### `capture`

- `--source`, `--filter`, `--action`
- `--trigger` (omit = auto), `--except`, `--only`, `--on-mismatch`
- `--wifi-handoff`, `--no-notify`, `--no-mirror`
- `--select`, `--interactive`
- Webcam tuning flags (timeout, preview, exposure bracket)

### `rule`

`create`, `config`, `set-action`, `execute`, `list`, `delete`, `init-defaults`

### `notify`

`test` (+ hidden `copy` for toast protocol)

### `daemon`

`start [--hidden]`, `stop`, `status`, `reload`

---

## 6. Non-negotiable constraints

1. Env vars only via child `Command::env()` — never `str::replace` on user scripts.
2. Variable namespaces: `QR_RAW`, `QR_NATIVE_*`, `QR_VAR_*`.
3. IPC: local sockets only.
4. TDD: traits → failing tests → minimal impl.
5. Platforms: Windows + Linux; macOS out of scope.
6. Redact sensitive values in logs.

---

## 7. Suggested next work (optional)

| Priority | Deliverable |
|----------|-------------|
| Later | IPC auto-route message on daemon wire |
| Later | OTLP telemetry (air-gap hook exists) |
| Later | Tauri daemon wrapper |

**Do not** document or implement as shipped: Linux desktop notifications, WiFi print as default shortcut, toast staging cleanup automation.

---

## 8. Development commands

```powershell
. .\scripts\dev-env.ps1   # webcam builds on Windows
cargo test
cargo clippy -- -D warnings
.\scripts\build-release.ps1
.\scripts\smoke-distribution.ps1
```

---

## 9. Maintainer notes

- TDD expected for new features.
- Do not auto-commit unless asked.
- Read `git status` before assuming clean tree.

---

## 10. Prompt for new AI sessions

> Router + v2 routing + Windows notifications + distribution are complete. Read [`DOCs/README.md`](README.md) and [`Routing-And-Default-Rules.md`](Routing-And-Default-Rules.md). Notify is **on by default** (`--no-notify` to disable). WiFi stock rule uses Settings handoff, not auto netsh. Do not refactor webcam/OpenCV unless required.
