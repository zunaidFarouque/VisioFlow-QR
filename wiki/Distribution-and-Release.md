# Distribution and Release

How to build, validate, and publish VisioFlow for Windows. End-user install commands are in [[Installation]].

---

## Supported install paths

1. **Scoop** portable (recommended)
2. **Traditional** machine-local install
3. **Zip** / no-install portable

---

## Release zip layout

Release zip root (`visioflow-win-x64/`) contains:

| File | Purpose |
|------|---------|
| `visioflow.exe` | Main CLI (webcam when built with default features) |
| `visioflow-toast.exe` | Toast activation helper for notification Copy button |
| `models/` | WeChat CNN files (`detect.*`, `sr.*`) for webcam decode |
| `launchers/*.vbs` | Five hidden launchers (`camera-auto`, `camera-copy`, `snip-auto`, `snip-copy`, `clipboard-qr`) |
| `logo v2.ico` | App icon (embedded in exes; Scoop/traditional shortcuts) |
| `default-rules.json` | Stock rule pack |
| `install-shortcuts.ps1` | Start Menu shortcuts targeting `.vbs` launchers (traditional/portable) |
| `bootstrap-portable.ps1` | Portable install bootstrap |
| `install-traditional.ps1` | Machine-local install script |
| `share/actions/*.ps1` | Platform action scripts |

---

## Build release zip

From the repo root (requires vcpkg for the full webcam build):

```powershell
.\scripts\build-release.ps1
```

### Optional flags

| Flag | Description |
|------|-------------|
| `-RouterOnly` | Build without OpenCV/webcam (`--no-default-features`) for a smaller binary |
| `-VcpkgRoot "D:\vcpkg"` | Override vcpkg location (default: `D:\vcpkg`) |
| `-SkipZip` | Stage `dist/visioflow-win-x64/` only |

The script sets `VCPKG_ROOT` and `VCPKGRS_TRIPLET=x64-windows-static-md`, builds release binaries, stages `dist/visioflow-win-x64/`, creates `dist/visioflow-win-x64.zip`, and prints the SHA256 hash.

### Manual equivalent

```powershell
$env:VCPKG_ROOT = 'D:\vcpkg'
$env:VCPKGRS_TRIPLET = 'x64-windows-static-md'
cargo build --release -p visioflow-cli
```

---

## Publish checklist

1. Run `.\scripts\build-release.ps1` and note the printed SHA256.
2. Upload `dist/visioflow-win-x64.zip` to a GitHub release tag (e.g. [v0.1.6](https://github.com/zunaidFarouque/VisioFlow-QR/releases/tag/v0.1.6)).
3. Update `scripts/packaging/scoop/visioflow.json`:
   - `version`
   - `architecture.64bit.url` (release download URL)
   - `architecture.64bit.hash` (SHA256 from build script; use `sha256:<hash>` for Scoop)
4. Copy `scripts/packaging/scoop/visioflow.json` into [Zunaid-Scoop-Bucket](https://github.com/zunaidFarouque/Zunaid-Scoop-Bucket) at `bucket/visioflow.json` and push (see `scripts/packaging/scoop/README.md`).

Scoop manifest path: `scripts/packaging/scoop/visioflow.json`

`post_install` seeds rules; Start Menu shortcuts target bundled **`launchers\*.vbs`** (Scoop Apps → VisioFlow; five entries including Clipboard to QR). Legacy desktop shortcuts and old `.cmd` / `scan-*` launcher names are removed on upgrade. `uninstaller` cleans legacy `%APPDATA%\VisioFlow\launchers` but leaves toast `VisioFlow.lnk` and the `visioflow:` protocol handler. Rules persist unless `scoop uninstall -p`.

Current release URL:

```
https://github.com/zunaidFarouque/VisioFlow-QR/releases/download/v0.1.6/visioflow-win-x64.zip
```

---

## Local validation before publishing

```powershell
.\scripts\smoke-router.ps1
.\scripts\smoke-default-rules.ps1
.\scripts\smoke-shortcuts.ps1
.\scripts\smoke-distribution.ps1
```

If all pass, the distribution and install scripts are in a releasable state.

---

## Scoop manifest summary

Manifest `shortcuts` point at bundled VBS (not `visioflow.exe` args directly):

```json
{
  "version": "0.1.6",
  "env_set": { "VISIOFLOW_MODELS_DIR": "$dir\\models" },
  "shortcuts": [
    ["launchers\\camera-auto.vbs", "VisioFlow\\VisioFlow QR Camera (auto)", "", "logo v2.ico"],
    ["launchers\\camera-copy.vbs", "VisioFlow\\VisioFlow QR Camera (copy)", "", "logo v2.ico"],
    ["launchers\\snip-auto.vbs", "VisioFlow\\VisioFlow QR Snip (auto)", "", "logo v2.ico"],
    ["launchers\\snip-copy.vbs", "VisioFlow\\VisioFlow QR Snip (copy)", "", "logo v2.ico"],
    ["launchers\\clipboard-qr.vbs", "VisioFlow\\VisioFlow Clipboard to QR", "", "logo v2.ico"]
  ],
  "architecture": {
    "64bit": {
      "url": "https://github.com/zunaidFarouque/VisioFlow-QR/releases/download/v0.1.6/visioflow-win-x64.zip",
      "extract_dir": "visioflow-win-x64"
    }
  }
}
```

(Hash omitted — copy the SHA256 from `build-release.ps1` into the real manifest.)

---

## Related

- Install paths: [[Installation]]
- Notifications helper: [[Notifications]]
