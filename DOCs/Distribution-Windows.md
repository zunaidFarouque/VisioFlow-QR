# VisioFlow Windows Distribution

This document explains how to publish and validate the three supported install paths. See also [Getting Started](Getting-Started.md) for end-user install commands and [DOCs index](README.md).
## Supported install paths

1. Scoop portable (recommended)
2. Traditional machine-local install
3. Zip/no-install portable

## Artifact layout for release zip

Release zip root (`visioflow-win-x64/`) should contain:

- `visioflow.exe` — main CLI (webcam capture when built with default features)
- `visioflow-toast.exe` — toast activation helper for notification copy actions
- `models/` — WeChat CNN files (`detect.*`, `sr.*`) for webcam decode
- `logo v2.ico` — app icon (embedded in exes; used by Start Menu shortcuts)
- `default-rules.json`
- `install-shortcuts.ps1`
- `bootstrap-portable.ps1`
- `install-traditional.ps1`
- `share/actions/*.ps1`

## Build release zip

From the repo root (requires vcpkg for the full webcam build):

```powershell
.\scripts\build-release.ps1
```

Optional flags:

- `-RouterOnly` — build without OpenCV/webcam (`--no-default-features`) for a smaller binary
- `-VcpkgRoot "D:\vcpkg"` — override vcpkg location (default: `D:\vcpkg`)
- `-SkipZip` — stage `dist/visioflow-win-x64/` only

The script sets `VCPKG_ROOT` and `VCPKGRS_TRIPLET=x64-windows-static-md`, builds release binaries, stages `dist/visioflow-win-x64/`, creates `dist/visioflow-win-x64.zip`, and prints the SHA256 hash.

Manual equivalent:

```powershell
$env:VCPKG_ROOT = 'D:\vcpkg'
$env:VCPKGRS_TRIPLET = 'x64-windows-static-md'
cargo build --release -p visioflow-cli
```

## Publish checklist

1. Run `.\scripts\build-release.ps1` and note the printed SHA256.
2. Upload `dist/visioflow-win-x64.zip` to a GitHub release tag (e.g. `v0.1.5`).
3. Update `scripts/packaging/scoop/visioflow.json`:
   - `version`
   - `architecture.64bit.url` (release download URL)
   - `architecture.64bit.hash` (SHA256 from build script output; use `sha256:<hash>` for Scoop)
4. Copy `scripts/packaging/scoop/visioflow.json` into [Zunaid-Scoop-Bucket](https://github.com/zunaidFarouque/Zunaid-Scoop-Bucket) at `bucket/visioflow.json` and push (see `scripts/packaging/scoop/README.md`).

Scoop manifest path: `scripts/packaging/scoop/visioflow.json`

`post_install` seeds rules and syncs to `%APPDATA%\visioflow\rules.json`. Start Menu shortcuts come from the manifest `shortcuts` field (Scoop Apps → VisioFlow; no desktop shortcuts). `post_install` also removes legacy desktop shortcuts from older releases. `uninstaller` removes legacy launchers; rules persist under `~/scoop/persist/visioflow/` unless `scoop uninstall -p`.

## Local validation before publishing

Run:

```powershell
.\scripts\smoke-router.ps1
.\scripts\smoke-default-rules.ps1
.\scripts\smoke-shortcuts.ps1
.\scripts\smoke-distribution.ps1
```

If all pass, the distribution and install scripts are in a releasable state.
