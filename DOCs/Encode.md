# QR encode (`encode`)

Generate QR codes from clipboard content — the inverse of `capture`.

## Quick start

```powershell
# Copy text or a URL, then:
visioflow encode --source clipboard
```

Default behavior (`--deliver preview`):

1. Read and normalize clipboard text (including Windows `.url` internet shortcuts)
2. Render a QR image
3. Open a **square** preview window sized 50–90% of screen height (DPI-aware, denser QR = larger)
4. Press Escape or close the window to dismiss

Start Menu shortcut: **VisioFlow Clipboard to QR** (via `clipboard-qr.vbs` launcher).

## Deliver modes

| `--deliver` | Preview | Copy image | Toast |
|-------------|---------|------------|-------|
| `preview` (default) | yes | no | no |
| `preview-copy` | yes | yes | yes |
| `copy` | no | yes | yes |

```powershell
# Preview only (default — shortcut uses this)
visioflow encode --source clipboard

# Preview and copy QR image to clipboard
visioflow encode --source clipboard --deliver preview-copy

# Copy image only (no window)
visioflow encode --source clipboard --deliver copy
```

## Clipboard normalization

| Clipboard content | Encoded payload |
|-------------------|-----------------|
| `https://example.com` | URL as-is |
| Plain text | Text as-is |
| Path to `.url` file | `URL=` value from the shortcut file |
| Copied `.url` file in Explorer (CF_HDROP) | `URL=` from first shortcut file |
| `.desktop` link file (Linux) | `URL=` from desktop entry |

## Flags

| Flag | Default | Purpose |
|------|---------|---------|
| `--source clipboard` | required | Clipboard input (only source in v1) |
| `--deliver` | `preview` | `preview`, `preview-copy`, or `copy` |
| `--preview-position` | `center` | Preview window anchor |
| `--ecc` | `m` | Error correction: `l`, `m`, `q`, `h` |
| `--no-notify` | off | Suppress success toast (copy deliver modes) |
| `--output <path>` | — | Save PNG alongside deliver action |

Preview window sizing is automatic: **50%** of work-area height for simple QR codes, up to **90%** for dense payloads. The window is always square and cannot be resized.

## Examples

```powershell
# Headless: save QR to file only
visioflow encode --source clipboard --deliver copy --no-notify --output qr.png
```

## Errors

| Message | Cause |
|---------|-------|
| `clipboard is empty` | No text and no file paths on clipboard |
| `no encodable text on clipboard` | Clipboard had files but none were `.url`/`.desktop` shortcuts |
| `payload too long for QR code` | Text exceeds QR capacity — shorten the input |

## Related

- [`Capture.md`](Capture.md) — decode QR from screen or webcam
- [`Routing-And-Default-Rules.md`](Routing-And-Default-Rules.md) — actions triggered after decode
