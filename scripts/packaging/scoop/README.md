# Scoop bucket layout

Canonical manifest lives here: `scripts/packaging/scoop/visioflow.json`

Published in [Zunaid-Scoop-Bucket](https://github.com/zunaidFarouque/Zunaid-Scoop-Bucket) (`bucket/visioflow.json`). Register the bucket locally as **`Zuanid-Scoop`**:

End users:

```powershell
scoop bucket add Zuanid-Scoop https://github.com/zunaidFarouque/Zunaid-Scoop-Bucket
scoop install Zuanid-Scoop/visioflow
```

`post_install` seeds rules and syncs them to `%APPDATA%\visioflow\rules.json`. Start Menu shortcuts are defined in the manifest `shortcuts` field (four entries under **Scoop Apps → VisioFlow**; no desktop shortcuts). `post_install` removes legacy desktop shortcuts from older releases.

Release zips bundle `models/` (WeChat CNN files). Manifest `env_set` points `VISIOFLOW_MODELS_DIR` at `$dir\models`.

`persist: data` keeps `rules.json` under `~/scoop/persist/visioflow/` across upgrades. `pre_uninstall` copies live rules back into persist before Scoop removes the app directory.

`uninstaller` removes legacy `%APPDATA%\VisioFlow\launchers` and any leftover Start Menu shortcuts from pre-0.1.3 installs. It does **not** remove the toast registration shortcut (`VisioFlow.lnk`) or the `visioflow:` protocol handler in HKCU.

Purge persisted rules: `scoop uninstall -p visioflow`
