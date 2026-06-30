# VisioFlow: Routing, Default Rules & UX Architecture

> **Status:** **v2 routing implemented** in core + CLI (auto-route, builtins, default rule pack, `init-defaults`, mismatch copy fallback, and `--notify` desktop toasts on Windows). Daemon IPC `execute_rule` remains explicit-name on the wire; capture auto-route parity is achieved client-side by resolving the matched rule with core `route_payload` then executing that rule via IPC — see §10.
>
> **Audience:** AI agents and human implementers. Read with [`ENGINE_RULES.md`](ENGINE_RULES.md), [`Architecture.md`](Architecture.md), [`USER_GUIDE.md`](USER_GUIDE.md).

---

## 1. Product intent

VisioFlow is used to **trigger actions** from scanned visual payloads (open URL, join WiFi, run script, etc.). It is not primarily a “show text in a terminal” tool.

**Default human experience:**

1. Decode payload from snip/webcam.
2. **Try to trigger** the right action (auto-routing or an explicit rule).
3. If nothing applies → **copy payload to clipboard** and notify the user.
4. Do **not** require users to read a black CMD window to recover the scan.

**Machines and power users** opt into stdout/piping/export via explicit CLI flags (`--action stdout`, `--export`, `--output json`, `rule execute --no-exec`).

---

## 2. Routing modes (three ways to run)

| Mode | How the user invokes it | Engine behavior |
|------|-------------------------|-----------------|
| **Auto route** | Omit `--trigger` (default after defaults are seeded) | Scan `auto_compatible` rules by `priority`; first match wins; run actions |
| **Explicit rule** | `--trigger <NAME>` | Only that rule; regex/native checks apply; match → actions; no match → fallback (see §5) |
| **Builtin escape** | `--trigger copy` | Copy payload only; **no** rule scan, **no** exec, **never** part of auto pool |

Reserved builtin trigger names (not creatable via `rule create`):

| Name | Purpose |
|------|---------|
| `copy` | Clipboard only; escape hatch from auto-routing |
| `plain` | Explicit stdout of payload (power users / debugging); not default snip UX |
| `auto` | Optional synonym documenting auto mode; omitting `--trigger` is preferred |

---

## 3. Rule model extensions (implemented)

Extend `Rule` in `visioflow-core` (serde defaults for backward compatibility):

```json
{
  "url": {
    "name": "url",
    "auto_compatible": true,
    "priority": 10,
    "regex": "^https?://\\S+$",
    "captures": {},
    "exec": "<share>/actions/open-url.ps1",
    "wifi_connect": false
  }
}
```

| Field | Type | Default | Meaning |
|-------|------|---------|---------|
| `auto_compatible` | `bool` | `false` | When true, rule is considered during **auto** scan |
| `priority` | `u32` | `100` | Lower number = tried earlier in auto scan |
| `regex` | `Option<String>` | `null` | If set, must match for rule to win (explicit or auto) |
| `captures` | map | `{}` | Regex group → `QR_VAR_*` suffix |
| `exec` | `Option<Path>` | `null` | Child script/binary; env vars only ([`ENGINE_RULES.md`](ENGINE_RULES.md)) |
| `wifi_connect` | `bool` | `false` | OS WiFi join from `QR_NATIVE_WIFI_*` |

**User-defined rules** should default to `auto_compatible: false` so broad regexes do not hijack auto mode until the user opts in.

**Shipped default rules** ship with `auto_compatible: true` and tuned `priority` values.

---

## 4. Auto-routing algorithm (implemented)

**Inputs:** decoded payload string, rule store, optional `--except <name>` (repeatable), optional `--only <name>` (repeatable).

**Candidates:**

```
rules where auto_compatible == true
  AND name not in --except
  AND name not in RESERVED_BUILTINS { copy, plain, auto }
sort by priority ascending (then stable name order)
```

**For each candidate rule:**

1. If `regex` is set → `apply_rule`; on `NoMatch`, continue to next rule.
2. If no `regex`:
   - **WiFi-style:** match if `wifi_connect` and native parsers produced `QR_NATIVE_WIFI_SSID` (or payload starts with `WIFI:`).
   - **Catch-all `plain`:** only if it is the last resort (highest `priority` number among auto rules, e.g. `999`) — behavior: **copy** (see §6), not stdout.
3. First successful match → `resolve_payload_fully` → `run_rule_actions` → notify → done.

**No match:** fallback policy (§5).

**Explicit `--trigger <name>`:** skip auto scan; load single rule by name (ignore `auto_compatible`). Same match/fail semantics.

---

## 5. Mismatch and fallback policy (implemented)

| Situation | Rule actions | Fallback (default) |
|-----------|--------------|-------------------|
| Auto: rule matched | Run exec / wifi_connect | — |
| Auto: no rule matched | None | **Copy** + notify |
| Explicit: rule matched | Run actions | — |
| Explicit: regex/native failed | None | **Copy** + notify |
| Builtin `--trigger copy` | None | Copy (primary outcome) |

**CLI flags:**

| Flag | Default | Purpose |
|------|---------|---------|
| `--on-mismatch <copy\|none>` | `copy` | After routing failure, copy payload or exit strict |
| `--except <NAME>` | — | Exclude rule(s) from auto scan (repeatable) |
| `--only <NAME>` | — | Optional whitelist for auto (stricter than except) |

**Exit codes:**

- `0` — routing succeeded and actions completed (or copy-only builtin).
- `1` — routing failed and `--on-mismatch none`, or user cancelled `--interactive`.
- Consider `2` for decode failure (no payload).

Decode success + routing failure must still expose `QR_RAW` to fallback copy.

---

## 6. stdout vs copy (UX contract)

| Mechanism | Primary audience | When to use |
|-----------|------------------|-------------|
| **Clipboard fallback** | Default snip user | Auto/explicit mismatch; builtin `copy` |
| **`--action copy`** | Human | Legacy/explicit copy without rules |
| **`--action stdout`** | Scripts, CI, pipes | `payload=$(visioflow capture ... --action stdout)` |
| **`--export bash\|ps1`** | Parent shell injection | Eval in bash/PowerShell |
| **`--output json`** | Tools, automation | Structured rule list / vars |
| **`--trigger plain`** | Debugging | Print payload to stdout on purpose |

**Do not** use stdout as the default snip feedback channel. Users get **notifications** (§7) instead of reading the console.

**Catch-all `plain` rule in auto:** copies payload; does **not** print to stdout unless user explicitly `--trigger plain` or `--action stdout`.

---

## 7. Notifications and feedback (implemented)

When not `--silent`, emit clear stderr lines. On Windows, `capture --notify` also emits native toast notifications.

| Event | Example message |
|-------|-----------------|
| Auto matched | `visioflow: matched rule "url"` |
| Explicit matched | `visioflow: rule "asset" applied` |
| Explicit mismatch | `visioflow: rule "asset" did not match; copied payload to clipboard` |
| Auto no match | `visioflow: no auto rule matched; copied payload to clipboard` |
| Copy builtin | `visioflow: copy-only mode` |
| WiFi connect | `visioflow: connecting to WiFi (rule "wifi")` |

`--notify` values:

- `off` — no desktop notification side-channel.
- `errors-only` (default) — notify on explicit mismatch, no auto match, and WiFi connect failures.
- `on` — notify on all routed outcomes, including successful matches.

If desktop notifications are unavailable, capture/routing still succeeds; a short stderr note is printed only in `--verbose` mode.

Optional structured line for tooling (`--output json` on capture):

```json
{"event":"rule_matched","rule":"url","fallback":false}
{"event":"rule_mismatch","rule":"asset","fallback":"copy"}
{"event":"no_auto_match","fallback":"copy"}
```

Sensitive values remain redacted per [`ENGINE_RULES.md`](ENGINE_RULES.md).

---

## 8. Default rule pack (implemented)

**Location:** `assets/default-rules.json` + `share/actions/*` (platform scripts).

**Install command:** `visioflow rule init-defaults [--merge|--force]`

- **`--merge`:** add missing default rules only.
- **`--force`:** replace with stock defaults (dangerous; document clearly).

### 8.1 Stock rules

| Rule | priority | auto_compatible | Match | Action |
|------|----------|-----------------|-------|--------|
| `wifi` | 5 | yes | Native `WIFI:` / `QR_NATIVE_WIFI_*` | `wifi_connect: true` |
| `url` | 10 | yes | `^https?://\S+$` | exec: open browser (`QR_RAW` or URI vars) |
| `mailto` | 15 | yes | `^mailto:` | exec: open mail handler |
| `tel` | 16 | yes | `^tel:` | exec: open dialer |
| `geo` | 17 | yes | `^geo:` | exec: open maps URL |
| `vcard` | 18 | yes | `BEGIN:VCARD` | exec: copy contact fields or open `.vcf` |
| `clipboard` | 20 | yes | `^(?i)(?:clipboard\|clip):(?P<text>.+)$` | exec: copy `QR_VAR_TEXT` |
| `plain` | 999 | yes | catch-all (no regex) | copy payload (last resort) |
| `asset` | 50 | **no** | `^ASSET:(?P<id>\d+)$` | exec: optional; **explicit `--trigger asset` only** |

### 8.2 Payload conventions

- **URL:** standard `http://` / `https://` strings.
- **Clipboard prefix:** `Clipboard:some text` or `CLIP:some text` (case-insensitive). Prefer **prefix** over suffix.
- **WiFi:** standard `WIFI:T:...;S:...;P:...;;` QR format.
- **Asset tags:** `ASSET:42` — explicit-only rule for corporate safety.

### 8.3 Action scripts (implemented)

Scripts live beside the binary, under `VISIOFLOW_SHARE`, or in the repo `share/` tree during dev. They read **env vars only** — never mutate script files.

| Script | Reads | OS behavior |
|--------|-------|-------------|
| `open-url.ps1` / `open-url.sh` | `QR_RAW` or `QR_NATIVE_URI_*` | `Start-Process` / `xdg-open` |
| `copy-text.ps1` / `copy-text.sh` | `QR_VAR_TEXT` or `QR_RAW` | Clipboard APIs |
| `open-mailto.*` | `QR_RAW` | Default mailto handler |
| `open-tel.*` | `QR_RAW` | Default tel handler |
| `open-geo.*` | `QR_NATIVE_GEO_LAT/LON` | Maps URL |

Native actions (`open_url`, `copy_to_clipboard` in Rust) may replace scripts in a later iteration if shell quoting becomes painful.

---

## 9. CLI examples (target UX)

### Daily use (auto)

```powershell
# Default: auto-route, copy on no match
visioflow capture --source snip

# Auto but never auto-join WiFi
visioflow capture --source snip --except wifi

# Desktop toast + stderr
visioflow capture --source snip --notify on
```

### Explicit rule

```powershell
# Corporate asset tag only
visioflow capture --source snip --trigger asset

# Mismatch still copies (default)
# stderr: rule "asset" did not match; copied payload to clipboard

# Strict automation: no copy on mismatch
visioflow capture --source snip --trigger asset --on-mismatch none
```

### Escape hatch

```powershell
# Never run any rule — copy only
visioflow capture --source snip --trigger copy
```

### Power users / machines

```powershell
# Pipe payload
for /f "delims=" %i in ('visioflow capture --source snip --action stdout --input-image qr.png') do @echo %i

# Parent shell vars
eval "$(visioflow capture --source snip --trigger asset --export bash --input-image qr.png)"

# Inspect routing without side effects
visioflow rule execute url --payload "https://example.com" --no-exec
visioflow --output json rule list
```

### Halts (already implemented; compose with routing)

```powershell
# Multiple payloads: pick one, then auto-route
visioflow capture --source snip --select

# Confirm before triggering
visioflow capture --source snip --interactive --trigger wifi
```

---

## 10. Interaction with existing features

| Feature | Behavior with v2 routing |
|---------|--------------------------|
| `--export bash\|ps1` | After successful route; vars from matched rule |
| `--ipc-socket` | Daemon `execute_rule` requires an explicit rule name on the wire; CLI `capture` now resolves auto-route locally (same core routing API) and forwards the matched rule to daemon for execution. |
| `daemon reload` | Reload rules after editing `rules.json` |
| `rule list` / `rule delete` | Unchanged; manage auto pool membership via `auto_compatible` |
| Two rule sources | Terminal **and** JSON file — same store (`rules.json`) |

Rules remain editable via CLI or direct JSON ([`USER_GUIDE.md`](USER_GUIDE.md)).

---

## 11. Security (unchanged)

- Env vars via child `Command::env()` only — no `str::replace` on user scripts.
- `QR_NATIVE_WIFI_PASSWORD` and similar: redact in logs.
- `asset` and custom exec rules default to **not** auto-compatible.
- `--interactive` recommended for sensitive payloads (WiFi, OTP) — document in USER_GUIDE.

---

## 12. Implementation phases (for AI / dev)

| Phase | Deliverable | Status |
|-------|-------------|--------|
| **1** | `auto_compatible`, `priority` on `Rule`; `route_payload()` / `route_auto()`; reserved names | **Done** — `rules/auto_test.rs`, `rules/builtins_test.rs` |
| **2** | CLI: default auto when no `--trigger`; `--except`; `--on-mismatch`; builtins `copy`/`plain` | **Done** — `tests/capture_routing.rs` |
| **3** | Fallback copy + stderr events | **Done** — `rules/notify_test.rs`, `capture_routing` mismatch tests |
| **4** | `assets/default-rules.json`; `rule init-defaults`; `share/actions/*` | **Done** — `tests/rule_init_defaults.rs`, `share.rs` tests |
| **5** | `smoke-default-rules.ps1` | **Done** — `scripts/smoke-default-rules.ps1` |
| **Later** | Optional dedicated IPC auto-route message | Not started |

**Do not refactor** webcam/OpenCV unless required. Follow TDD per [`Architecture.md`](Architecture.md).

---

## 13. Open decisions (locked)

| Decision | Resolution |
|----------|------------|
| Default when `--trigger` omitted | **Auto-route** (after defaults seeded) |
| Mismatch fallback | **Copy** (both auto and explicit); override with `--on-mismatch none` |
| Builtin `copy` in auto pool | **Never** — explicit only |
| Catch-all `plain` in auto | **Copy**, not stdout |
| stdout for snip users | **Opt-in** via `--action stdout` / `--trigger plain` |
| User rules in auto | **`auto_compatible: false`** by default |
| Regex in auto | **Required** when rule defines `regex`; must match to win |
| Notifications | stderr always (unless `--silent`); desktop optional |

---

## 14. AI session prompt (copy/adapt)

> Implement v2 routing per `DOCs/Routing-And-Default-Rules.md`: auto-route when `--trigger` omitted; `auto_compatible` + `priority`; builtins `copy`/`plain`; `--except` and `--on-mismatch`; copy fallback + user notifications; ship `assets/default-rules.json` and action scripts; TDD first. Do not break existing `--trigger <name>`, daemon IPC, or webcam path. Update `USER_GUIDE.md` when behavior ships.

---

## 15. Related documents

| Document | Role |
|----------|------|
| [`ENGINE_RULES.md`](ENGINE_RULES.md) | Variable namespaces, sandbox, redaction |
| [`Architecture.md`](Architecture.md) | CLI noun-verb, TDD, daemon |
| [`Handoff-Router-Phase.md`](Handoff-Router-Phase.md) | What is already built |
| [`USER_GUIDE.md`](USER_GUIDE.md) | End-user commands (update after implementation) |
| [`IPC_PROTOCOL.md`](IPC_PROTOCOL.md) | Daemon message shapes |
