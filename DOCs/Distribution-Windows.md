# VisioFlow Windows Distribution

This document explains how to publish and validate the three supported install paths.

## Supported install paths

1. Scoop portable (recommended)
2. Traditional machine-local install
3. Zip/no-install portable

## Artifact layout for release zip

Release zip root should contain:

- `visioflow.exe`
- `default-rules.json`
- `install-shortcuts.ps1`
- `bootstrap-portable.ps1`
- `share/actions/*.ps1`

## Publish checklist

1. Build router-only binary:
   - `cargo build --release -p visioflow-cli --no-default-features`
2. Stage release directory:
   - Copy `target/release/visioflow.exe`
   - Copy `assets/default-rules.json`
   - Copy `share/` directory
   - Copy `scripts/install-shortcuts.ps1`
   - Copy `scripts/bootstrap-portable.ps1`
3. Zip the staged folder as `visioflow-win-x64.zip`.
4. Compute SHA256 of the zip.
5. Upload zip to GitHub release.
6. Update `scripts/packaging/scoop/visioflow.json`:
   - `version`
   - `architecture.64bit.url`
   - `architecture.64bit.hash`
7. Publish/refresh the Scoop bucket manifest.

## Local validation before publishing

Run:

```powershell
.\scripts\smoke-router.ps1
.\scripts\smoke-default-rules.ps1
.\scripts\smoke-shortcuts.ps1
.\scripts\smoke-distribution.ps1
```

If all pass, the distribution and install scripts are in a releasable state.
