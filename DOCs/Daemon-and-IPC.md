# Daemon and IPC

The VisioFlow **daemon** keeps rules in memory and serves routing over a **local socket**. The CLI connects when you pass `--ipc-socket` or set `VISIOFLOW_IPC_SOCKET`.

Transport: **named pipes** on Windows, **Unix domain sockets** on Linux (`interprocess` crate).

---

## Socket paths

| Platform | Default |
|----------|---------|
| Windows | `\\.\pipe\visioflow.sock` |
| Linux | `/tmp/visioflow.sock` |

Custom path:

```powershell
visioflow daemon start --socket "\\.\pipe\my-visioflow.sock"
visioflow --ipc-socket "\\.\pipe\my-visioflow.sock" rule execute asset --payload "ASSET:1"
```

---

## Daemon commands

```text
visioflow daemon <start|stop|status|reload>
```

| Subcommand | Purpose |
|------------|---------|
| `start` | Foreground server (blocks terminal) |
| `start --hidden` | Detached background process |
| `stop` | Stop running daemon |
| `status` | PID and socket health |
| `reload` | Re-read `rules.json` from disk |

```powershell
# Debug in foreground
visioflow daemon start

# Production background
visioflow daemon start --hidden
visioflow daemon status
visioflow daemon reload
visioflow daemon stop
```

PID file: `daemon.pid` next to the rules store.

---

## CLI via daemon

When `--ipc-socket` is set:

| Command | Behavior |
|---------|----------|
| `rule execute <NAME> --payload <STR>` | Daemon runs rule (same semantics as local) |
| `capture --trigger <NAME>` | Explicit trigger via IPC |
| `capture` (auto-route) | CLI resolves match locally with core `route_payload`, then sends `execute_rule` with matched name |

**Reload after disk edits:**

```powershell
visioflow daemon reload
```

Without `--ipc-socket`, the CLI reads `rules.json` on each invocation — no reload needed.

---

## Wire protocol (summary)

- **Framing:** one JSON object per line (NDJSON), UTF-8, newline-terminated.
- **Correlation:** every message has an `id`; server echoes it on responses.

### Client → server

| `type` | Purpose |
|--------|---------|
| `ping` | Health check → `pong` |
| `execute_rule` | `name` + `payload` → `rule_result` or `error` |
| `list_rules` | → `rules_list` |
| `reload` | → `pong` |

Example:

```json
{"type":"execute_rule","id":2,"name":"url","payload":"https://example.com"}
```

### Server → client

| `type` | Purpose |
|--------|---------|
| `pong` | Ping / reload ack |
| `rule_result` | `vars` map + optional `exit_code` |
| `error` | Human-readable `message` (no secrets) |
| `rules_list` | `names` array |

Full message schemas: [IPC_PROTOCOL.md](IPC_PROTOCOL.md).

---

## Example session

```powershell
visioflow daemon start --hidden

visioflow --ipc-socket "\\.\pipe\visioflow.sock" rule execute asset --payload "ASSET:99"

visioflow --ipc-socket "\\.\pipe\visioflow.sock" capture --source snip --trigger asset

# Auto-route: client resolves rule, daemon executes
visioflow --ipc-socket "\\.\pipe\visioflow.sock" capture --source snip

visioflow daemon reload
visioflow daemon stop
```

---

## Security

- IPC is **local-only** — do not use world-writable socket paths on multi-user Linux hosts.
- Daemon logging **redacts** sensitive values (`QR_NATIVE_WIFI_PASSWORD` → `[REDACTED]`).
- Error `message` strings are for operators; parse `type` + structured fields in automation.

---

## Implementation notes

- Pure-Rust daemon in `crates/visioflow-cli/src/commands/daemon.rs`.
- Protocol types in `visioflow_core::ipc`.
- Auto-route on the wire still uses explicit `execute_rule` with the resolved rule name (client-side resolution).
