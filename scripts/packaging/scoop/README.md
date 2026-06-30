# Scoop bucket layout

Canonical manifest lives here: `scripts/packaging/scoop/visioflow.json`

Published in the personal bucket: [Zunaid-Scoop-Bucket](https://github.com/zunaidFarouque/Zunaid-Scoop-Bucket) (`bucket/visioflow.json`).

End users:

```powershell
scoop bucket add zunaid-scoop-bucket https://github.com/zunaidFarouque/Zunaid-Scoop-Bucket
scoop install zunaid-scoop-bucket/visioflow
```

`post_install` seeds rules, syncs them to `%APPDATA%\visioflow\rules.json`, and runs `install-shortcuts.ps1` from the release zip. No manual bootstrap step.

`persist: data` keeps `rules.json` under `~/scoop/persist/visioflow/` across upgrades. `pre_uninstall` copies live rules back into persist before Scoop removes the app directory.

`uninstaller` removes Desktop/Start Menu shortcuts and `%APPDATA%\VisioFlow\launchers`. It does **not** remove the `visioflow:` protocol handler in HKCU (safe to leave; refreshed when `visioflow notify test` runs after reinstall).

Purge persisted rules: `scoop uninstall -p visioflow`
