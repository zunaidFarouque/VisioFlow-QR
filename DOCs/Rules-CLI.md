# Rules CLI

Manage routing rules in the JSON store (`%APPDATA%\visioflow\rules.json` on Windows). Rules map decoded payloads to environment variables and optional actions.

---

## Command reference

```text
visioflow rule <subcommand>
```

| Subcommand | Purpose |
|------------|---------|
| `create <NAME>` | Create an empty rule |
| `config <NAME> --regex <PAT> [--map G:VAR ...]` | Set regex and capture mappings |
| `set-action <NAME> [--exec <PATH>] [--wifi-connect]` | Attach exec script and/or OS WiFi join |
| `execute <NAME> --payload <STR> [--no-exec]` | Apply rule; spawn actions unless `--no-exec` |
| `list` | List rules (`--output json` for full objects) |
| `delete <NAME>` | Remove a rule |
| `init-defaults [--merge] [--force]` | Install stock rules from `assets/default-rules.json` |

Hidden integration flag: `--store <path>` (tests only).

---

## Create and configure

```powershell
visioflow rule create asset
visioflow rule config asset --regex "ASSET:(?P<asset>\d+)" --map asset:ASSET
visioflow rule execute asset --payload "ASSET:42"
```

Output:

```text
QR_RAW=ASSET:42
QR_VAR_ASSET=42
```

- `--regex` uses **named capture groups** (`(?P<name>…)`).
- `--map asset:ASSET` maps group `asset` → env suffix `ASSET` → `QR_VAR_ASSET`.
- Omit `--map` to auto-uppercase the group name.

### Native-parser-only rules

WiFi and similar rules can omit `--regex`; native parsers populate `QR_NATIVE_*` keys.

```powershell
visioflow rule create wifi
visioflow rule set-action wifi --exec share/actions/wifi-handoff.ps1
```

For automatic OS join (netsh / nmcli), use `--wifi-connect` instead of or in addition to `--exec`:

```powershell
visioflow rule set-action wifi --wifi-connect
```

Stock `wifi` rule uses **exec handoff** (Settings UI), not `--wifi-connect`.

---

## Attach actions

```powershell
visioflow rule set-action asset --exec C:\scripts\handle-asset.ps1
```

Scripts receive env vars in the **child process only** — VisioFlow never edits script files. See [ENGINE_RULES.md](ENGINE_RULES.md).

---

## Execute

```powershell
# Resolve variables and run exec / wifi_connect
visioflow rule execute url --payload "https://example.com"

# Inspect variables without side effects
visioflow rule execute url --payload "https://example.com" --no-exec
```

With `--ipc-socket` or `VISIOFLOW_IPC_SOCKET`, execution delegates to the daemon.

---

## init-defaults

Installs the stock rule pack and resolves `share/actions/*` scripts.

```powershell
visioflow rule init-defaults          # upsert all defaults
visioflow rule init-defaults --merge    # add missing only
visioflow rule init-defaults --force    # replace entire store
```

Rule table: [Routing-And-Default-Rules.md](Routing-And-Default-Rules.md) § Default rule pack.

---

## List and delete

```powershell
visioflow rule list
visioflow --output json rule list
visioflow rule delete oldrule
```

After editing `rules.json` by hand, run `visioflow daemon reload` if a daemon is running.

---

## Export to parent shell

Global `--export` prints eval-safe assignments for the **parent** session:

**PowerShell**

```powershell
Invoke-Expression (visioflow --export ps1 rule execute asset --payload "ASSET:42")
$env:QR_VAR_ASSET
```

**bash**

```bash
eval "$(visioflow --export bash rule execute asset --payload 'ASSET:42')"
echo "$QR_VAR_ASSET"
```

After capture + route:

```powershell
Invoke-Expression (visioflow --export ps1 capture --source snip --trigger asset)
```

---

## rules.json schema

```json
{
  "asset": {
    "name": "asset",
    "regex": "ASSET:(?P<asset>\\d+)",
    "captures": { "asset": "ASSET" },
    "exec": "C:\\scripts\\handle-asset.ps1",
    "auto_compatible": false,
    "priority": 50,
    "wifi_connect": false
  }
}
```

| Field | Notes |
|-------|-------|
| `name` | Must match object key |
| `regex` | Rust regex; omit for native-only rules |
| `captures` | Group name → `QR_VAR_*` suffix |
| `exec` | Script path (resolved via binary dir, `VISIOFLOW_SHARE`, or repo `share/`) |
| `wifi_connect` | When true, OS WiFi join from `QR_NATIVE_WIFI_*` |
| `auto_compatible` | Include in auto-route scan (default `false` for user rules) |
| `priority` | Lower = tried earlier in auto scan (default `100`) |

**JSON escaping:** double backslashes in regex and Windows paths.

---

## Variable namespaces

| Prefix | Source |
|--------|--------|
| `QR_RAW` | Full decoded string |
| `QR_NATIVE_*` | Built-in parsers (WiFi, URI, mailto, tel, geo, vCard) |
| `QR_VAR_*` | Regex named captures |

Common `QR_NATIVE_*` keys: see [ENGINE_RULES.md](ENGINE_RULES.md) and the variable table in [Rules-CLI.md](Rules-CLI.md) (namespace section).

---

## Security

1. Child-process env only — no string substitution into script sources.
2. Treat `--export` output like code — eval only trusted payloads.
3. Daemon logs redact `QR_NATIVE_WIFI_PASSWORD` as `[REDACTED]`.
4. `asset` and custom exec rules default to `auto_compatible: false`.
