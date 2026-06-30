# Installs Windows launchers + Start Menu shortcuts for VisioFlow capture entrypoints.
# Usage:
#   .\scripts\install-shortcuts.ps1
#   .\scripts\install-shortcuts.ps1 -BinPath "target\release\visioflow.exe"
#   .\scripts\install-shortcuts.ps1 -Force

param(
    [string]$BinPath,
    [string]$IconPath,
    [string]$LauncherRoot,
    [string]$DesktopDir,
    [string]$StartMenuProgramsDir,
    [switch]$Force
)

$ErrorActionPreference = "Stop"
Set-Location (Split-Path $PSScriptRoot -Parent)

function Resolve-BinPath {
    param([string]$Requested)

    if ($Requested) {
        $resolved = Resolve-Path -Path $Requested -ErrorAction SilentlyContinue
        if (-not $resolved) {
            throw "visioflow binary not found at '$Requested'"
        }
        return $resolved.Path
    }

    $candidates = @(
        "target\release\visioflow.exe",
        "target\debug\visioflow.exe"
    )
    foreach ($candidate in $candidates) {
        if (Test-Path $candidate) {
            return (Resolve-Path $candidate).Path
        }
    }

    throw "No visioflow.exe found. Build first: cargo build --release -p visioflow-cli --no-default-features"
}

function Resolve-IconLocation {
    param(
        [string]$Requested,
        [string]$Bin
    )

    if ($Requested) {
        $resolved = Resolve-Path -Path $Requested -ErrorAction SilentlyContinue
        if (-not $resolved) {
            throw "Icon not found at '$Requested'"
        }
        return "$($resolved.Path),0"
    }

    return "$Bin,0"
}

function Write-Wrapper {
    param(
        [string]$Path,
        [string]$Bin,
        [string]$LaunchArgs
    )

    $body = @"
@echo off
"$Bin" $LaunchArgs %*
"@
    Set-Content -Path $Path -Value $body -Encoding ASCII
}

function New-Shortcut {
    param(
        [string]$Path,
        [string]$TargetPath,
        [string]$Description,
        [string]$IconLocation
    )

    $shell = New-Object -ComObject WScript.Shell
    $shortcut = $shell.CreateShortcut($Path)
    $shortcut.TargetPath = $TargetPath
    $shortcut.WorkingDirectory = Split-Path -Parent $TargetPath
    $shortcut.Description = $Description
    if ($IconLocation) {
        $shortcut.IconLocation = $IconLocation
    }
    $shortcut.Save()
}

function Remove-LegacyShortcuts {
    param(
        [string]$Desktop,
        [string]$StartMenuFolder
    )

    $legacyNames = @(
        "VisioFlow Scan (Auto)",
        "VisioFlow Scan (Copy)",
        "VisioFlow Scan (Plain)"
    )
    foreach ($name in $legacyNames) {
        Remove-Item (Join-Path $Desktop "$name.lnk") -Force -ErrorAction SilentlyContinue
        Remove-Item (Join-Path $StartMenuFolder "$name.lnk") -Force -ErrorAction SilentlyContinue
    }

    $legacyLaunchers = @("scan-auto.cmd", "scan-copy.cmd", "scan-plain.cmd")
    $launcherRoot = Join-Path $env:APPDATA "VisioFlow\launchers"
    foreach ($file in $legacyLaunchers) {
        Remove-Item (Join-Path $launcherRoot $file) -Force -ErrorAction SilentlyContinue
    }
}

$bin = Resolve-BinPath -Requested $BinPath
$iconLocation = Resolve-IconLocation -Requested $IconPath -Bin $bin

$launcherRoot = if ($LauncherRoot) { $LauncherRoot } else { Join-Path $env:APPDATA "VisioFlow\launchers" }
$desktop = if ($DesktopDir) { $DesktopDir } else { [Environment]::GetFolderPath("Desktop") }
$startMenuPrograms = if ($StartMenuProgramsDir) { $StartMenuProgramsDir } else { [Environment]::GetFolderPath("Programs") }
$startMenuFolder = Join-Path $startMenuPrograms "VisioFlow"

New-Item -ItemType Directory -Path $launcherRoot -Force | Out-Null
New-Item -ItemType Directory -Path $startMenuFolder -Force | Out-Null

if ($Force) {
    Remove-LegacyShortcuts -Desktop $desktop -StartMenuFolder $startMenuFolder
}

$wrappers = @(
    @{
        Name = "camera-auto"
        Args = "capture --source webcam"
        ShortcutName = "VisioFlow QR Camera (auto)"
        Description = "Scan QR via webcam and auto-route with default rules"
    },
    @{
        Name = "camera-copy"
        Args = "capture --source webcam --trigger copy"
        ShortcutName = "VisioFlow QR Camera (copy)"
        Description = "Scan QR via webcam and copy payload only"
    },
    @{
        Name = "snip-auto"
        Args = "capture --source snip"
        ShortcutName = "VisioFlow QR Snip (auto)"
        Description = "Scan QR via screen snip and auto-route with default rules"
    },
    @{
        Name = "snip-copy"
        Args = "capture --source snip --trigger copy"
        ShortcutName = "VisioFlow QR Snip (copy)"
        Description = "Scan QR via screen snip and copy payload only"
    }
)

foreach ($entry in $wrappers) {
    $wrapperPath = Join-Path $launcherRoot "$($entry.Name).cmd"
    if ((Test-Path $wrapperPath) -and -not $Force) {
        throw "Wrapper exists: $wrapperPath (rerun with -Force to overwrite)"
    }
    Write-Wrapper -Path $wrapperPath -Bin $bin -LaunchArgs $entry.Args

    $menuShortcut = Join-Path $startMenuFolder "$($entry.ShortcutName).lnk"

    if ((Test-Path $menuShortcut) -and -not $Force) {
        throw "Shortcut exists for '$($entry.ShortcutName)' (rerun with -Force to overwrite)"
    }

    New-Shortcut -Path $menuShortcut -TargetPath $wrapperPath -Description $entry.Description -IconLocation $iconLocation
}

Write-Host "Installed VisioFlow launchers in: $launcherRoot"
Write-Host "Installed shortcuts in Start Menu\Programs\VisioFlow (no desktop shortcuts)"
Write-Host "Shortcut icon: $iconLocation"
Write-Host "Tip: map hotkeys in AHK/PowerToys to the .cmd launchers."
