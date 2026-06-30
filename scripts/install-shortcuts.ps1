# Installs Windows launchers + shortcuts for common VisioFlow capture entrypoints.
# Usage:
#   .\scripts\install-shortcuts.ps1
#   .\scripts\install-shortcuts.ps1 -BinPath "target\release\visioflow.exe"
#   .\scripts\install-shortcuts.ps1 -Force

param(
    [string]$BinPath,
    [ValidateSet("snip", "webcam")]
    [string]$Source = "snip",
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
        [string]$Description
    )

    $shell = New-Object -ComObject WScript.Shell
    $shortcut = $shell.CreateShortcut($Path)
    $shortcut.TargetPath = $TargetPath
    $shortcut.WorkingDirectory = Split-Path -Parent $TargetPath
    $shortcut.Description = $Description
    $shortcut.Save()
}

$bin = Resolve-BinPath -Requested $BinPath

$launcherRoot = if ($LauncherRoot) { $LauncherRoot } else { Join-Path $env:APPDATA "VisioFlow\launchers" }
$desktop = if ($DesktopDir) { $DesktopDir } else { [Environment]::GetFolderPath("Desktop") }
$startMenuPrograms = if ($StartMenuProgramsDir) { $StartMenuProgramsDir } else { [Environment]::GetFolderPath("Programs") }
$startMenuFolder = Join-Path $startMenuPrograms "VisioFlow"

New-Item -ItemType Directory -Path $launcherRoot -Force | Out-Null
New-Item -ItemType Directory -Path $startMenuFolder -Force | Out-Null

$wrappers = @(
    @{
        Name = "scan-auto"
        Args = "capture --source $Source"
        ShortcutName = "VisioFlow Scan (Auto)"
        Description = "Scan QR via $Source and auto-route with default rules"
    },
    @{
        Name = "scan-copy"
        Args = "capture --source $Source --trigger copy"
        ShortcutName = "VisioFlow Scan (Copy)"
        Description = "Scan QR via $Source and copy payload only"
    },
    @{
        Name = "scan-plain"
        Args = "capture --source $Source --trigger plain --action stdout"
        ShortcutName = "VisioFlow Scan (Plain)"
        Description = "Scan QR via $Source and print payload to stdout"
    }
)

foreach ($entry in $wrappers) {
    $wrapperPath = Join-Path $launcherRoot "$($entry.Name).cmd"
    if ((Test-Path $wrapperPath) -and -not $Force) {
        throw "Wrapper exists: $wrapperPath (rerun with -Force to overwrite)"
    }
    Write-Wrapper -Path $wrapperPath -Bin $bin -LaunchArgs $entry.Args

    $desktopShortcut = Join-Path $desktop "$($entry.ShortcutName).lnk"
    $menuShortcut = Join-Path $startMenuFolder "$($entry.ShortcutName).lnk"

    if (((Test-Path $desktopShortcut) -or (Test-Path $menuShortcut)) -and -not $Force) {
        throw "Shortcut exists for '$($entry.ShortcutName)' (rerun with -Force to overwrite)"
    }

    New-Shortcut -Path $desktopShortcut -TargetPath $wrapperPath -Description $entry.Description
    New-Shortcut -Path $menuShortcut -TargetPath $wrapperPath -Description $entry.Description
}

Write-Host "Installed VisioFlow launchers in: $launcherRoot"
Write-Host "Installed shortcuts on Desktop and Start Menu\Programs\VisioFlow"
Write-Host "Tip: map hotkeys in AHK/PowerToys to the .cmd launchers."
