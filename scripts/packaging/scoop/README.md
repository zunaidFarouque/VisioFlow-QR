# Scoop bucket layout

Canonical manifest lives here: `scripts/packaging/scoop/visioflow.json`

Published in [Zunaid-Scoop-Bucket](https://github.com/zunaidFarouque/Zunaid-Scoop-Bucket) (`bucket/visioflow.json`). Register the bucket locally as **`Zuanid-Scoop`**:

End users:

```powershell
scoop bucket add Zuanid-Scoop https://github.com/zunaidFarouque/Zunaid-Scoop-Bucket
scoop install Zuanid-Scoop/visioflow
```

`post_install` seeds rules, syncs them to `%APPDATA%\visioflow\rules.json`, and runs `install-shortcuts.ps1` from the release zip. No manual bootstrap step.

Release zips bundle `models/` (WeChat CNN files). Manifest `env_set` points `VISIOFLOW_MODELS_DIR` at `$dir\models`.

`persist: data` keeps `rules.json` under `~/scoop/persist/visioflow/` across upgrades. `pre_uninstall` copies live rules back into persist before Scoop removes the app directory.

`uninstaller` removes Desktop/Start Menu shortcuts and `%APPDATA%\VisioFlow\launchers`. It does **not** remove the `visioflow:` protocol handler in HKCU (safe to leave; refreshed when `visioflow notify test` runs after reinstall).

Purge persisted rules: `scoop uninstall -p visioflow`
