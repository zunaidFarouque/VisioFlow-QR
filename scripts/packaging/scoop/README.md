# Scoop bucket layout

Copy `visioflow.json` into a Scoop bucket repository:

```
visioflow-bucket/
  bucket.json
  visioflow.json
```

`bucket.json` example:

```json
{
  "version": 1,
  "description": "VisioFlow QR visual payload router",
  "homepage": "https://github.com/zunaidFarouque/VisioFlow-QR",
  "license": "MIT"
}
```

End users:

```powershell
scoop bucket add visioflow-bucket https://github.com/<org>/visioflow-bucket
scoop install visioflow
```

`post_install` seeds rules, syncs them to `%APPDATA%\visioflow\rules.json`, and runs `install-shortcuts.ps1` from the release zip. No manual bootstrap step.

`persist: data` keeps `rules.json` under `~/scoop/persist/visioflow/` across upgrades. `pre_uninstall` copies live rules back into persist before Scoop removes the app directory.

`uninstaller` removes Desktop/Start Menu shortcuts and `%APPDATA%\VisioFlow\launchers`. It does **not** remove the `visioflow:` protocol handler in HKCU (safe to leave; refreshed when `visioflow notify test` runs after reinstall).

Purge persisted rules: `scoop uninstall -p visioflow`
