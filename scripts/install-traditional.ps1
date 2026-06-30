# Traditional machine-local install for VisioFlow on Windows.
# Usage:
#   .\scripts\install-traditional.ps1 -DistRoot .\dist\visioflow-win64

param(
    [string]$DistRoot,
    [string]$InstallRoot,
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
$resolvedInstall = if ($InstallRoot) { $InstallRoot } else { Join-Path $env:LOCALAPPDATA "Programs\VisioFlow" }
$resolvedInstall = [System.IO.Path]::GetFullPath($resolvedInstall)

$distBin = Join-Path $resolvedDist "visioflow.exe"
$distRules = Join-Path $resolvedDist "default-rules.json"
if (-not (Test-Path $distBin)) { throw "Distribution missing visioflow.exe: $distBin" }
if (-not (Test-Path $distRules)) { throw "Distribution missing default-rules.json: $distRules" }

$distShare = Join-Path $resolvedDist "share"

New-Item -ItemType Directory -Path $resolvedInstall -Force | Out-Null
New-Item -ItemType Directory -Path (Join-Path $resolvedInstall "share") -Force | Out-Null

Copy-Item -Path $distBin -Destination (Join-Path $resolvedInstall "visioflow.exe") -Force
Copy-Item -Path $distRules -Destination (Join-Path $resolvedInstall "share\default-rules.json") -Force
if (Test-Path $distShare) {
    Copy-Item -Path (Join-Path $distShare "*") -Destination (Join-Path $resolvedInstall "share") -Recurse -Force
}

$installScript = Join-Path $resolvedDist "install-shortcuts.ps1"
if (-not (Test-Path $installScript)) {
    $installScript = Join-Path $PSScriptRoot "install-shortcuts.ps1"
}
if (-not (Test-Path $installScript)) {
    throw "install-shortcuts.ps1 not found in dist or scripts folder"
}

$appDataRoot = if ($AppDataDir) { $AppDataDir } else { $env:APPDATA }
$launcherRoot = Join-Path $appDataRoot "VisioFlow\launchers"
$rulesStore = Join-Path $appDataRoot "visioflow\rules.json"
$desktop = if ($DesktopDir) { $DesktopDir } else { [Environment]::GetFolderPath("Desktop") }
$programs = if ($StartMenuProgramsDir) { $StartMenuProgramsDir } else { [Environment]::GetFolderPath("Programs") }

Ensure-DefaultRulesStore -DefaultRulesPath (Join-Path $resolvedInstall "share\default-rules.json") -RulesStorePath $rulesStore -Overwrite:$Force

& $installScript `
    -BinPath (Join-Path $resolvedInstall "visioflow.exe") `
    -LauncherRoot $launcherRoot `
    -DesktopDir $desktop `
    -StartMenuProgramsDir $programs `
    -Force:$Force

Write-Host "Traditional install complete: $resolvedInstall"
Write-Host "Rules store: $rulesStore"
Write-Host "Launchers: $launcherRoot"
