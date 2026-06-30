# VisioFlow: Architecture & TDD Master Context

## 1. Project overview

VisioFlow is an optical automation engine and **visual payload router**. It captures QR payloads via webcam or screen snip, parses them, maps data to ephemeral environment variables, and triggers native OS actions.

- **User docs:** [`DOCs/README.md`](README.md) — topic guides
- **Routing:** [`Routing-And-Default-Rules.md`](Routing-And-Default-Rules.md) — **implemented** (auto-route, builtins, default pack, copy fallback, Windows toasts)

## 2. Technology stack and constraints

| Layer | Choice |
|-------|--------|
| Core | Rust (`visioflow-core`) — snip decode, rules, IPC, native parsers |
| CLI | `visioflow-cli` — capture UX, minifb webcam preview, daemon |
| Daemon | Pure-Rust background service — **implemented**; Tauri wrapper deferred |
| Platforms | Windows + Linux (macOS out of scope) |

**Constraints:**

- **Static capture:** snip/file path uses native Rust (`rqrr`, Otsu/Median) — no OpenCV in static path.
- **Webcam:** OpenCV `VideoCapture`, WeChat CNN, exposure bracketing — see [`Rust OpenCV QR Scanning Architecture.md`](Rust%20OpenCV%20QR%20Scanning%20Architecture.md).
- **IPC:** CLI ↔ daemon via local sockets (named pipes / UDS). No file polling.
- **Sandbox:** env vars via child `Command::env()` only; parent injection via `--export` only.

## 3. CLI architecture (noun-verb)

Implemented with `clap`:

### Global flags

| Flag | Purpose |
|------|---------|
| `--output plain\|json` | Resolved variable format |
| `--verbose` | Diagnostics on stderr |
| `--silent` | Suppress stdout |
| `--export bash\|ps1` | Parent shell assignments |
| `--ipc-socket <PATH>` | Daemon routing (`VISIOFLOW_IPC_SOCKET` env) |
| `--disable-telemetry` | Hidden air-gap guard (future OTLP) |

### `capture` — execution engine

| Area | Flags / behavior |
|------|------------------|
| Source | `--source snip\|webcam`, `--filter otsu\|median`, `--action stdout\|copy` |
| Routing | Omit `--trigger` → auto-route; `--except`, `--only`, `--on-mismatch` |
| WiFi | `--wifi-handoff open-settings\|print` (default: Settings handoff) |
| Feedback | Toasts **on** by default; `--no-notify` |
| Webcam | `--timeout`, preview position/scale, exposure bracket tuning, `--no-mirror` (mirrored default) |
| Halts | `--select`, `--interactive` |

### `rule` — automation manager

`create`, `config`, `set-action`, `execute`, `list`, `delete`, `init-defaults` — see [`Rules-CLI.md`](Rules-CLI.md).

### `notify` — Windows notifications

`notify test` — toast smoke test; hidden `notify copy` for protocol activation.

### `daemon` — background service

`start [--hidden]`, `stop`, `status`, `reload` — see [`Daemon-and-IPC.md`](Daemon-and-IPC.md).

## 4. Execution sandbox

- Variables live in the **child process** only.
- `--export` is the only supported parent-shell injection path.
- Sensitive keys redacted in daemon logs (`[REDACTED]`).

---

## 5. TDD engineering protocol

### Step 1: Interface and trait definition

Abstract OS layer (`SystemExecutor`, IPC traits) for testability with `mockall`.

### Step 2: Red phase

Write failing tests first; assert exact outputs.

### Step 3: Green phase

Minimal implementation; no `.unwrap()` in production paths.

### Step 4: Refactor

Zero-bloat; `#[cfg(target_os = …)]` for platform branches.

---

## Related documents

| Document | Role |
|----------|------|
| [`ENGINE_RULES.md`](ENGINE_RULES.md) | Variable namespaces, optical pipeline notes |
| [`Handoff-Router-Phase.md`](Handoff-Router-Phase.md) | Feature status table |
| [`IPC_PROTOCOL.md`](IPC_PROTOCOL.md) | NDJSON wire format |
| [`PLATFORM_CI.md`](PLATFORM_CI.md) | CI matrix and conventions |
