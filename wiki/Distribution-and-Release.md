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
| `default-rules.json` | Stock rule pack |
| `install-shortcuts.ps1` | Desktop / Start Menu shortcuts |
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
2. Upload `dist/visioflow-win-x64.zip` to a GitHub release tag (e.g. [v0.1.0](https://github.com/zunaidFarouque/VisioFlow-QR/releases/tag/v0.1.0)).
3. Update `scripts/packaging/scoop/visioflow.json`:
   - `version`
   - `architecture.64bit.url` (release download URL)
   - `architecture.64bit.hash` (SHA256 from build script; use `sha256:<hash>` for Scoop)
4. Publish or refresh the Scoop bucket manifest.

Scoop manifest path: `scripts/packaging/scoop/visioflow.json`

Current release URL:

```
https://github.com/zunaidFarouque/VisioFlow-QR/releases/download/v0.1.0/visioflow-win-x64.zip
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

```json
{
  "version": "0.1.0",
  "architecture": {
    "64bit": {
      "url": "https://github.com/zunaidFarouque/VisioFlow-QR/releases/download/v0.1.0/visioflow-win-x64.zip",
      "hash": "sha256:aac65efc3bc0afea6478be3b1a2ba65ddc89664a534e1383e1bab599fd704b93",
      "extract_dir": "visioflow-win-x64"
    }
  }
}
```

---

## Related

- Install paths: [[Installation]]
- Notifications helper: [[Notifications]]
