# Portable zip/no-install bootstrap for VisioFlow on Windows.
# Usage:
#   .\bootstrap-portable.ps1 -DistRoot .

param(
    [string]$DistRoot,
    [string]$DesktopDir,
    [string]$StartMenuProgramsDir,
    [string]$AppDataDir,
    [switch]$Force
)

$ErrorActionPreference = "Stop"
Set-Location (Split-Path $PSScriptRoot -Parent)

function Resolve-DistRoot([string]$Requested) {
    if ($Requested) {
        $resolved = Resolve-Path -Path $Requested -ErrorAction SilentlyContinue
        if (-not $resolved) {
            throw "Dist root not found: $Requested"
        }
        return $resolved.Path
    }
    return (Resolve-Path ".").Path
}

function Ensure-DefaultRulesStore {
    param(
        [string]$DefaultRulesPath,
        [string]$RulesStorePath,
        [switch]$Overwrite
    )

    if ((Test-Path $RulesStorePath) -and -not $Overwrite) {
        return
    }
    New-Item -ItemType Directory -Path (Split-Path -Parent $RulesStorePath) -Force | Out-Null
    Copy-Item -Path $DefaultRulesPath -Destination $RulesStorePath -Force
}

$resolvedDist = Resolve-DistRoot -Requested $DistRoot
$bin = Join-Path $resolvedDist "visioflow.exe"
$rulesAsset = Join-Path $resolvedDist "default-rules.json"
$shortcutInstaller = Join-Path $resolvedDist "install-shortcuts.ps1"

if (-not (Test-Path $bin)) { throw "Portable directory missing visioflow.exe: $bin" }
if (-not (Test-Path $rulesAsset)) { throw "Portable directory missing default-rules.json: $rulesAsset" }
if (-not (Test-Path $shortcutInstaller)) { throw "Portable directory missing install-shortcuts.ps1: $shortcutInstaller" }

$appDataRoot = if ($AppDataDir) { $AppDataDir } else { $env:APPDATA }
$launcherRoot = Join-Path $appDataRoot "VisioFlow\launchers"
$rulesStore = Join-Path $appDataRoot "visioflow\rules.json"
$programs = if ($StartMenuProgramsDir) { $StartMenuProgramsDir } else { [Environment]::GetFolderPath("Programs") }
$legacyDesktop = if ($DesktopDir) { $DesktopDir } else { [Environment]::GetFolderPath("Desktop") }

Ensure-DefaultRulesStore -DefaultRulesPath $rulesAsset -RulesStorePath $rulesStore -Overwrite:$Force

& $shortcutInstaller `
    -BinPath $bin `
    -LauncherRoot $launcherRoot `
    -DesktopDir $legacyDesktop `
    -StartMenuProgramsDir $programs `
    -Force:$Force

Write-Host "Portable bootstrap complete."
Write-Host "Binary location: $bin"
Write-Host "Rules store: $rulesStore"
Write-Host "Launchers: $launcherRoot"
