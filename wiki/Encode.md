# QR encode (`encode`)

Generate QR codes from clipboard content — the **inverse** of [[Capture]] (decode → route). Copy text or a URL, run `encode`, and show or copy a QR image.

## Quick start

```powershell
# Copy text or a URL, then:
visioflow encode --source clipboard
```

Default behavior (`--deliver preview`):

1. Read clipboard text and/or Explorer file drops (CF_HDROP)
2. Normalize payload (plain text/URL as-is; `.url` / `.desktop` → extract `URL=`)
3. Render a QR image (min ~512px per side; ECC level `m`)
4. Open a **square, non-resizable** preview window sized ~50–90% of work-area height by QR density (DPI-aware)
5. Press Escape or close the window to dismiss

On success (when stdout is not silenced), the CLI prints the **normalized payload** string.

Start Menu shortcut: **VisioFlow Clipboard to QR** (`clipboard-qr.vbs` → `encode --source clipboard`).

## Deliver modes

| `--deliver` | Preview | Copy QR image | Toast |
|-------------|---------|---------------|-------|
| `preview` (default) | yes | no | no |
| `preview-copy` | yes | yes | yes† |
| `copy` | no | yes | yes† |

† Toast only when image copy succeeds and `--no-notify` is not set. Body is truncated payload text (`QR copied — …`).

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
| Plain text | Text as-is (trimmed) |
| Path / `file://` path to `.url` or `.desktop` | `URL=` value from the shortcut file |
| Copied `.url` / `.desktop` in Explorer (CF_HDROP) | `URL=` from the first matching shortcut file |

Non-shortcut files on the clipboard alone are not encoded (see Errors).

## Flags

| Flag | Default | Purpose |
|------|---------|---------|
| `--source clipboard` | required | Clipboard input (only source in v0.1.6) |
| `--deliver` | `preview` | `preview`, `preview-copy`, or `copy` |
| `--preview-position` | `center` | Same anchors as capture: `top-left` … `bottom-right` |
| `--ecc` | `m` | Error correction: `l`, `m`, `q`, `h` |
| `--no-notify` | off | Suppress success toast (copy deliver modes only) |
| `--output <path>` | — | Save PNG alongside the deliver action |

### Preview sizing

- Always **square**; window **resize disabled**
- Side length ≈ work-area height × fraction in **[0.50, 0.90]**, interpolated by QR module count (version 1 ≈ 21 modules → 50%; dense ≈ 77 → 90%), then clamped to fit the work area
- Positioned with `--preview-position` (default **center**)

## Examples

```powershell
# Headless: save QR to file and copy image (no toast)
visioflow encode --source clipboard --deliver copy --no-notify --output qr.png

# Stronger ECC, preview top-right
visioflow encode --source clipboard --ecc h --preview-position top-right
```

## Errors

| Message | Cause |
|---------|-------|
| `clipboard is empty` | No usable text and no file paths on clipboard |
| `no encodable text on clipboard` | Clipboard had file paths but none were `.url` / `.desktop` shortcuts |
| `payload too long for QR code` | Text exceeds QR capacity — shorten the input |

## Related

- [[Capture]] — decode QR from screen or webcam (inverse direction)
- [[Quick-Start]] — install and Start Menu shortcuts
- [[Routing-and-Auto-Route]] — actions triggered after decode
- [[CLI-Reference]] — full `encode` flag list
