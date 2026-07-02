# Validates scripts/packaging/scoop/visioflow.json structure and bootstrap hooks.
# Usage:
#   .\scripts\test-scoop-manifest.ps1

$ErrorActionPreference = "Stop"
Set-Location (Split-Path $PSScriptRoot -Parent)

function Assert-True($condition, $message) {
    if (-not $condition) {
        throw $message
    }
}

function Assert-Contains([string]$text, [string]$needle, [string]$label) {
    if ($text -notmatch [regex]::Escape($needle)) {
        throw "$label missing '$needle'"
    }
}

$manifestPath = "scripts\packaging\scoop\visioflow.json"
Assert-True (Test-Path $manifestPath) "missing scoop manifest: $manifestPath"

$manifestRaw = Get-Content -Path $manifestPath -Raw
$manifest = $manifestRaw | ConvertFrom-Json

Assert-True ($manifest.version) "scoop manifest: version missing"
Assert-True ($manifest.bin) "scoop manifest: bin missing"
Assert-True ($manifest.persist) "scoop manifest: persist missing"
Assert-Contains $manifestRaw "VISIOFLOW_MODELS_DIR" "scoop manifest env_set"
Assert-Contains $manifestRaw "pre_uninstall" "scoop manifest"
Assert-Contains $manifestRaw "uninstaller" "scoop manifest"
Assert-Contains $manifestRaw "shortcuts" "scoop manifest"
Assert-Contains $manifestRaw "VisioFlow QR Camera (auto)" "scoop manifest shortcuts"
Assert-Contains $manifestRaw "VisioFlow QR Snip (copy)" "scoop manifest shortcuts"
Assert-True (-not ($manifestRaw -match "install-shortcuts\.ps1")) "scoop manifest post_install should not call install-shortcuts.ps1"
Assert-Contains $manifestRaw "VisioFlow Scan (Auto)" "scoop manifest legacy cleanup"
Assert-Contains $manifestRaw "launchers" "scoop manifest legacy cleanup"

$shortcuts = @($manifest.shortcuts)
Assert-True ($shortcuts.Count -eq 4) "scoop manifest: shortcuts should have 4 entries"
foreach ($entry in $shortcuts) {
    Assert-True ($entry.Count -ge 4) "scoop manifest: shortcut missing icon path"
    Assert-True ($entry[3] -eq "logo v2.ico") "scoop manifest: shortcut icon should be logo v2.ico"
}

Assert-Contains $manifestRaw "logo v2.ico" "scoop manifest shortcuts icon"

$postInstall = @($manifest.post_install)
Assert-True ($postInstall.Count -ge 3) "scoop manifest: post_install should run bootstrap logic"
Assert-True (-not ($manifestRaw -match "Run bootstrap once")) "scoop manifest: manual bootstrap message should be removed"

$uninstaller = $manifest.uninstaller
Assert-True ($uninstaller) "scoop manifest: uninstaller block missing"
$uninstallScript = @($uninstaller.script)
Assert-True ($uninstallScript.Count -ge 1) "scoop manifest: uninstaller.script missing"
Assert-True (-not ($manifestRaw -match ", 'VisioFlow'\)")) "scoop manifest uninstaller should not remove toast VisioFlow.lnk"

$scoopUrl = $manifest.url
if (-not $scoopUrl -and $manifest.architecture) {
    $scoopUrl = $manifest.architecture.'64bit'.url
}
Assert-True $scoopUrl "scoop manifest: url missing"
$scoopHash = $manifest.hash
if (-not $scoopHash -and $manifest.architecture) {
    $scoopHash = $manifest.architecture.'64bit'.hash
}
Assert-True $scoopHash "scoop manifest: hash missing"
Assert-True ($scoopHash -ne "PLACEHOLDER_HASH") "scoop manifest: hash must be updated before release"

Write-Host "Scoop manifest validation passed."
