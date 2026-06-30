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
Assert-Contains $manifestRaw "VisioFlow Scan (Auto)" "scoop manifest uninstaller"
Assert-Contains $manifestRaw "launchers" "scoop manifest uninstaller"

$postInstall = @($manifest.post_install)
Assert-True ($postInstall.Count -ge 3) "scoop manifest: post_install should run bootstrap logic"
Assert-True (-not ($manifestRaw -match "Run bootstrap once")) "scoop manifest: manual bootstrap message should be removed"

$uninstaller = $manifest.uninstaller
Assert-True ($uninstaller) "scoop manifest: uninstaller block missing"
$uninstallScript = @($uninstaller.script)
Assert-True ($uninstallScript.Count -ge 1) "scoop manifest: uninstaller.script missing"

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

Write-Host "Scoop manifest validation passed."
