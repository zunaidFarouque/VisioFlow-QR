# VisioFlow IPC Protocol

> **Phase D foundation.** Wire format and message types for CLI ↔ daemon communication.
> Transport (Named Pipes on Windows, Unix domain sockets on Linux) is implemented in a later phase using `interprocess::local_socket` per [`PLATFORM_CI.md`](PLATFORM_CI.md).

## Transport framing

- One JSON object per line, terminated by `\n` (newline-delimited JSON / NDJSON).
- UTF-8 encoding only.
- No length prefix; the receiver reads until `\n`, then parses the line as JSON.
- Socket path is supplied via CLI `--ipc-socket` (e.g. `\\.\pipe\visioflow.sock` on Windows, `/tmp/visioflow.sock` on Linux).

## Request / response correlation

Every message carries an `id` field so the client can match responses to requests.

| Field | Type | Notes |
|---|---|---|
| `id` | `u64` | Monotonically increasing client-assigned id (recommended). UUID strings may be adopted later; clients must use unique ids per in-flight request. |

The server **must** echo the same `id` on the corresponding response (including `Error`).

## Message envelope

Messages use an externally tagged JSON representation:

```json
{ "type": "<variant>", "id": <u64>, ... }
```

The `type` discriminator uses `snake_case` variant names.

---

## Client → Server messages

### `ping`

Health check. Server responds with `pong`.

```json
{"type":"ping","id":1}
```

| Field | Type | Required |
|---|---|---|
| `type` | `"ping"` | yes |
| `id` | `u64` | yes |

### `execute_rule`

Run a named rule against a payload string (same semantics as `visioflow rule execute`).

```json
{"type":"execute_rule","id":2,"name":"wifi","payload":"WIFI:T:MyNet;P:secret;;"}
```

| Field | Type | Required |
|---|---|---|
| `type` | `"execute_rule"` | yes |
| `id` | `u64` | yes |
| `name` | `string` | yes — rule name |
| `payload` | `string` | yes — raw QR / barcode text |

### `list_rules`

List all configured rule names.

```json
{"type":"list_rules","id":3}
```

| Field | Type | Required |
|---|---|---|
| `type` | `"list_rules"` | yes |
| `id` | `u64` | yes |

### `reload`

Reload rules from disk into the daemon's in-memory store.

```json
{"type":"reload","id":4}
```

| Field | Type | Required |
|---|---|---|
| `type` | `"reload"` | yes |
| `id` | `u64` | yes |

---

## Server → Client messages

### `pong`

Response to `ping`. Also used as a success ack for `reload` when no structured result is needed.

```json
{"type":"pong","id":1}
```

| Field | Type | Required |
|---|---|---|
| `type` | `"pong"` | yes |
| `id` | `u64` | yes — matches request |

### `rule_result`

Result of `execute_rule`. Variables follow [`ENGINE_RULES.md`](ENGINE_RULES.md) namespaces (`QR_RAW`, `QR_NATIVE_*`, `QR_VAR_*`).

```json
{"type":"rule_result","id":2,"vars":{"QR_RAW":"asset:123","QR_VAR_ASSET":"123"},"exit_code":0}
```

| Field | Type | Required |
|---|---|---|
| `type` | `"rule_result"` | yes |
| `id` | `u64` | yes |
| `vars` | `object` (string → string) | yes — may be empty `{}` |
| `exit_code` | `i32` | no — child process exit code when rule defines an action |

When `exit_code` is absent, the rule had no exec action or the daemon did not run a child process.

### `error`

Request failed (unknown rule, invalid payload, internal error, etc.).

```json
{"type":"error","id":2,"message":"rule not found: wifi"}
```

| Field | Type | Required |
|---|---|---|
| `type` | `"error"` | yes |
| `id` | `u64` | yes |
| `message` | `string` | yes — human-readable; must not contain secrets |

### `rules_list`

Response to `list_rules`.

```json
{"type":"rules_list","id":3,"names":["wifi","uri","asset"]}
```

| Field | Type | Required |
|---|---|---|
| `type` | `"rules_list"` | yes |
| `id` | `u64` | yes |
| `names` | `string[]` | yes — may be empty `[]` |

---

## Example exchanges

### Ping / pong

```
→ {"type":"ping","id":1}
← {"type":"pong","id":1}
```

### Execute rule (success)

```
→ {"type":"execute_rule","id":10,"name":"asset","payload":"asset:999"}
← {"type":"rule_result","id":10,"vars":{"QR_RAW":"asset:999","QR_VAR_ASSET":"999"},"exit_code":0}
```

### Execute rule (failure)

```
→ {"type":"execute_rule","id":11,"name":"missing","payload":"x"}
← {"type":"error","id":11,"message":"rule not found: missing"}
```

### List rules

```
→ {"type":"list_rules","id":20}
← {"type":"rules_list","id":20,"names":["wifi","uri"]}
```

### Reload

```
→ {"type":"reload","id":30}
← {"type":"pong","id":30}
```

---

## Rust types (`visioflow_core::ipc`)

| Wire `type` | Rust enum variant |
|---|---|
| `ping` | `ClientMessage::Ping` |
| `execute_rule` | `ClientMessage::ExecuteRule` |
| `list_rules` | `ClientMessage::ListRules` |
| `reload` | `ClientMessage::Reload` |
| `pong` | `ServerMessage::Pong` |
| `rule_result` | `ServerMessage::RuleResult` |
| `error` | `ServerMessage::Error` |
| `rules_list` | `ServerMessage::RulesList` |

Traits (mockable via `mockall` in tests):

- `IpcClient` — `send_request`, `recv_response`
- `IpcServer` — `accept`, `handle_one_message`

Codec helpers: `serialize_client_line`, `deserialize_client_line`, `serialize_server_line`, `deserialize_server_line`.

---

## Security notes

- Never log `QR_NATIVE_WIFI_PASSWORD` or other sensitive `vars` values; redact as `[REDACTED]` in daemon logs.
- IPC is local-only; socket paths must not be world-writable on multi-user systems.
- Error `message` strings are for operators, not for automated parsing by the CLI (use `type` + `id`).
