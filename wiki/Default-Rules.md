# Default Rules

The stock rule pack ships in `assets/default-rules.json` and is installed with:

```powershell
visioflow rule init-defaults          # upsert all defaults
visioflow rule init-defaults --merge  # add missing only
visioflow rule init-defaults --force  # replace entire store
```

Action scripts resolve from the binary directory, `VISIOFLOW_SHARE`, or the repo `share/` tree during development.

---

## Stock rules table

| Rule | Priority | Auto | Regex / Match | Action |
|------|----------|------|---------------|--------|
| `wifi` | 5 | yes | `^WIFI:` | exec: `wifi-handoff.ps1` (Settings UI; not silent netsh join) |
| `url` | 10 | yes | `^https?://\S+$` | exec: open browser (`QR_RAW` or URI vars) |
| `mailto` | 15 | yes | `^mailto:` | exec: open mail handler |
| `tel` | 16 | yes | `^tel:` | exec: open dialer |
| `geo` | 17 | yes | `^geo:` | exec: open maps URL |
| `vcard` | 18 | yes | `BEGIN:VCARD` | exec: copy contact text |
| `event` | 19 | yes | `BEGIN:VEVENT` | exec: copy calendar text |
| `clipboard` | 20 | yes | `^(?i)(?:clipboard\|clip):(?P<text>.+)$` | exec: copy `QR_VAR_TEXT` |
| `matmsg` | 21 | yes | `^MATMSG:` | exec: parse MATMSG → `mailto:` |
| `asset` | 50 | **no** | `^ASSET:(?P<id>\d+)$` | optional exec; **explicit `--trigger asset` only** |
| `plain` | 999 | yes | catch-all (no regex) | copy payload (last resort) |

Lower **priority** numbers are tried first during auto-route.

---

## Payload conventions

| Type | Format | Example |
|------|--------|---------|
| **URL** | Standard `http://` / `https://` | `https://example.com` |
| **Clipboard prefix** | `Clipboard:` or `CLIP:` (case-insensitive) | `CLIP:meeting notes` |
| **WiFi** | Standard WiFi QR format | `WIFI:T:WPA;S:MyNetwork;P:secret;;` |
| **Asset tags** | `ASSET:` + digits | `ASSET:42` |
| **MATMSG** | Japanese mobile email QR | `MATMSG:TO:user@example.com;SUB:Hello;;` |
| **vCard / event** | vCard or iCalendar text | `BEGIN:VCARD` … |

---

## Examples by rule

### URL

```powershell
visioflow rule execute url --payload "https://example.com" --no-exec
visioflow capture --source snip   # auto-matches URL QRs
```

Opens the default browser via `share/actions/open-url.ps1`.

### WiFi

```powershell
visioflow capture --source snip --trigger wifi
visioflow capture --source snip --except wifi   # auto-route but skip WiFi
```

Stock `wifi` rule uses **Settings handoff** (`wifi-handoff.ps1`), not silent `netsh` join. Override with `--wifi-handoff print` to print credentials on stdout.

For automatic OS join, configure a custom rule with `--wifi-connect` (see [[Custom-Rules]]).

### Clipboard prefix

Payload: `CLIP:copy this text` → copies `copy this text` via `QR_VAR_TEXT`.

### MATMSG

Payload: `MATMSG:TO:user@example.com;SUB:Hello;;` → parsed and opened as `mailto:`.

### Asset (explicit only)

```powershell
visioflow capture --source snip --trigger asset
```

`asset` has `auto_compatible: false` so broad corporate tags never hijack auto mode.

### Plain catch-all

When no other auto rule matches, `plain` (priority 999) copies the raw payload to the clipboard.

---

## Action scripts

Scripts read **environment variables only** — VisioFlow never edits script files.

| Script | Reads | OS behavior |
|--------|-------|-------------|
| `open-url.ps1` / `open-url.sh` | `QR_RAW` or `QR_NATIVE_URI_*` | `Start-Process` / `xdg-open` |
| `copy-text.ps1` / `copy-text.sh` | `QR_VAR_TEXT` or `QR_RAW` | Clipboard APIs |
| `open-mailto.*` | `QR_RAW` | Default mailto handler |
| `open-tel.*` | `QR_RAW` | Default tel handler |
| `open-geo.*` | `QR_NATIVE_GEO_LAT/LON` | Maps URL |
| `wifi-handoff.ps1` | `QR_NATIVE_WIFI_*`, `VISIOFLOW_WIFI_HANDOFF_MODE` | Settings handoff UI |
| `open-matmsg.ps1` | `QR_RAW` | Parse MATMSG → `mailto:` |

---

## Variable namespaces

| Prefix | Source |
|--------|--------|
| `QR_RAW` | Full decoded string |
| `QR_NATIVE_*` | Built-in parsers (WiFi, URI, mailto, tel, geo, vCard) |
| `QR_VAR_*` | Regex named captures |

Common native keys include `QR_NATIVE_WIFI_SSID`, `QR_NATIVE_WIFI_PASSWORD`, `QR_NATIVE_URI_HOST`, `QR_NATIVE_GEO_LAT`, `QR_NATIVE_GEO_LON`.

---

## Related

- Routing behavior: [[Routing-and-Auto-Route]]
- Custom rules: [[Custom-Rules]]
- Install command: [[Quick-Start]]
