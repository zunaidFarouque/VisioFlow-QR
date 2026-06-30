# Custom Rules

Manage routing rules in the JSON store (`%APPDATA%\visioflow\rules.json` on Windows). Rules map decoded payloads to environment variables and optional actions.

```text
visioflow rule <subcommand>
```

---

## Subcommands

| Subcommand | Purpose |
|------------|---------|
| `create <NAME>` | Create an empty rule |
| `config <NAME> --regex <PAT> [--map G:VAR ...]` | Set regex and capture mappings |
| `set-action <NAME> [--exec <PATH>] [--wifi-connect]` | Attach exec script and/or OS WiFi join |
| `execute <NAME> --payload <STR> [--no-exec]` | Apply rule; spawn actions unless `--no-exec` |
| `list` | List rules (`--output json` for full objects) |
| `delete <NAME>` | Remove a rule |
| `init-defaults [--merge] [--force]` | Install stock rules |

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

For automatic OS join (netsh / nmcli), use `--wifi-connect`:

```powershell
visioflow rule set-action wifi --wifi-connect
```

Stock `wifi` rule uses **exec handoff** (Settings UI), not `--wifi-connect`.

---

## Attach actions

```powershell
visioflow rule set-action asset --exec C:\scripts\handle-asset.ps1
```

Scripts receive env vars in the **child process only** — VisioFlow never edits script files.

---

## Execute

```powershell
# Resolve variables and run exec / wifi_connect
visioflow rule execute url --payload "https://example.com"

# Inspect variables without side effects
visioflow rule execute url --payload "https://example.com" --no-exec
```

With `--ipc-socket` or `VISIOFLOW_IPC_SOCKET`, execution delegates to the daemon. See [[Daemon-and-IPC]].

---

## List and delete

```powershell
visioflow rule list
visioflow --output json rule list
visioflow rule delete oldrule
```

After editing `rules.json` by hand, run `visioflow daemon reload` if a daemon is running.

---

## auto_compatible and priority

| Field | Default | Notes |
|-------|---------|-------|
| `auto_compatible` | `false` for user rules | Set `true` to include rule in auto-route scan |
| `priority` | `100` | Lower = tried earlier in auto scan |

**Best practice:** keep custom rules `auto_compatible: false` until you trust the regex. Broad patterns can hijack auto mode.

To opt a custom rule into auto-route, edit `rules.json` or use the CLI workflow, then set `auto_compatible: true` and tune `priority`.

Example — auto-compatible custom rule:

```json
{
  "ticket": {
    "name": "ticket",
    "regex": "^TICKET:(?P<num>\\d+)$",
    "captures": { "num": "NUM" },
    "exec": "C:\\scripts\\open-ticket.ps1",
    "auto_compatible": true,
    "priority": 30,
    "wifi_connect": false
  }
}
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
| `auto_compatible` | Include in auto-route scan |
| `priority` | Lower = tried earlier in auto scan |

**JSON escaping:** double backslashes in regex and Windows paths.

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

## Variable namespaces

| Prefix | Source |
|--------|--------|
| `QR_RAW` | Full decoded string |
| `QR_NATIVE_*` | Built-in parsers (WiFi, URI, mailto, tel, geo, vCard) |
| `QR_VAR_*` | Regex named captures |

---

## Security

1. Child-process env only — no string substitution into script sources.
2. Treat `--export` output like code — eval only trusted payloads.
3. Daemon logs redact `QR_NATIVE_WIFI_PASSWORD` as `[REDACTED]`.
4. `asset` and custom exec rules default to `auto_compatible: false`.

---

## Related

- Stock rules: [[Default-Rules]]
- Routing modes: [[Routing-and-Auto-Route]]
- Full flag reference: [[CLI-Reference]]
