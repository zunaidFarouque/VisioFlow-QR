# Routing and Auto-Route

VisioFlow is used to **trigger actions** from scanned visual payloads (open URL, join WiFi, run script, etc.). It is not primarily a "show text in a terminal" tool.

**Default human experience:**

1. Decode payload from snip/webcam.
2. **Try to trigger** the right action (auto-routing or an explicit rule).
3. If nothing applies → **copy payload to clipboard** and notify the user.
4. Do **not** require users to read a black CMD window to recover the scan.

---

## Routing modes

| Mode | How the user invokes it | Engine behavior |
|------|-------------------------|-----------------|
| **Auto route** | Omit `--trigger` (default after defaults are seeded) | Scan `auto_compatible` rules by `priority`; first match wins; run actions |
| **Explicit rule** | `--trigger <NAME>` | Only that rule; regex/native checks apply; match → actions; no match → fallback |
| **Builtin escape** | `--trigger copy` | Copy payload only; **no** rule scan, **no** exec, **never** part of auto pool |

### Reserved builtin trigger names

These names cannot be created via `rule create`:

| Name | Purpose |
|------|---------|
| `copy` | Clipboard only; escape hatch from auto-routing |
| `plain` | Explicit stdout of payload (power users / debugging); not default snip UX |
| `auto` | Optional synonym documenting auto mode; omitting `--trigger` is preferred |

---

## Auto-routing algorithm

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
   - **Catch-all `plain`:** only if it is the last resort (highest `priority` number among auto rules, e.g. `999`) — behavior: **copy**, not stdout.
3. First successful match → resolve variables → run rule actions → notify → done.

**No match:** fallback policy (see below).

**Explicit `--trigger <name>`:** skip auto scan; load single rule by name (ignore `auto_compatible`). Same match/fail semantics.

---

## Mismatch and fallback policy

| Situation | Rule actions | Fallback (default) |
|-----------|--------------|-------------------|
| Auto: rule matched | Run exec / wifi_connect | — |
| Auto: no rule matched | None | **Copy** + notify |
| Explicit: rule matched | Run actions | — |
| Explicit: regex/native failed | None | **Copy** + notify |
| Builtin `--trigger copy` | None | Copy (primary outcome) |

| Flag | Default | Purpose |
|------|---------|---------|
| `--on-mismatch <copy\|none>` | `copy` | After routing failure, copy payload or exit strict |
| `--except <NAME>` | — | Exclude rule(s) from auto scan (repeatable) |
| `--only <NAME>` | — | Optional whitelist for auto (stricter than except) |

**Exit codes:**

- `0` — routing succeeded and actions completed (or copy-only builtin).
- `1` — routing failed and `--on-mismatch none`, or user cancelled `--interactive`.
- `2` — decode failure (no payload).

---

## stdout vs copy (UX contract)

| Mechanism | Primary audience | When to use |
|-----------|------------------|-------------|
| **Clipboard fallback** | Default snip user | Auto/explicit mismatch; builtin `copy` |
| **`--action copy`** | Human | Legacy/explicit copy without rules |
| **`--action stdout`** | Scripts, CI, pipes | `payload=$(visioflow capture ... --action stdout)` |
| **`--export bash\|ps1`** | Parent shell injection | Eval in bash/PowerShell |
| **`--output json`** | Tools, automation | Structured rule list / vars |
| **`--trigger plain`** | Debugging | Print payload to stdout on purpose |

**Do not** use stdout as the default snip feedback channel. Users get **notifications** instead of reading the console.

**Catch-all `plain` rule in auto:** copies payload; does **not** print to stdout unless user explicitly `--trigger plain` or `--action stdout`.

---

## Examples

### Daily use (auto)

```powershell
# Default: auto-route, copy on no match
visioflow capture --source snip

# Auto but never auto-join WiFi
visioflow capture --source snip --except wifi

# Default: toasts on (use --no-notify to disable)
visioflow capture --source snip
```

### Explicit rule

```powershell
# Corporate asset tag only
visioflow capture --source snip --trigger asset

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
$payload = visioflow capture --source snip --action stdout

# Parent shell vars
Invoke-Expression (visioflow --export ps1 capture --source snip --trigger asset)

# Inspect routing without side effects
visioflow rule execute url --payload "https://example.com" --no-exec
visioflow --output json rule list
```

### Halts

```powershell
# Multiple payloads: pick one, then auto-route
visioflow capture --source snip --select

# Confirm before triggering
visioflow capture --source snip --interactive --trigger wifi
```

---

## Rule model fields

| Field | Type | Default | Meaning |
|-------|------|---------|---------|
| `auto_compatible` | `bool` | `false` | When true, rule is considered during **auto** scan |
| `priority` | `u32` | `100` | Lower number = tried earlier in auto scan |
| `regex` | `Option<String>` | `null` | If set, must match for rule to win |
| `captures` | map | `{}` | Regex group → `QR_VAR_*` suffix |
| `exec` | `Option<Path>` | `null` | Child script/binary; env vars only |
| `wifi_connect` | `bool` | `false` | OS WiFi join from `QR_NATIVE_WIFI_*` |

User-defined rules default to `auto_compatible: false`. Shipped default rules use `auto_compatible: true` with tuned priorities. See [[Default-Rules]] and [[Custom-Rules]].

---

## Related

- Stock rules: [[Default-Rules]]
- Custom rules: [[Custom-Rules]]
- Capture flags: [[Capture]]
- Notifications: [[Notifications]]
- Daemon IPC: [[Daemon-and-IPC]]
