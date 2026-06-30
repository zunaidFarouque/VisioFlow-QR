# VisioFlow Windows shortcut installer smoke test.
# Usage:
#   .\scripts\smoke-shortcuts.ps1

$ErrorActionPreference = "Stop"
Set-Location (Split-Path $PSScriptRoot -Parent)

function Assert-True($condition, $message) {
    if (-not $condition) {
        throw $message
    }
}

$binCandidates = @(
    "target\release\visioflow.exe",
    "target\debug\visioflow.exe"
)
$bin = $null
foreach ($candidate in $binCandidates) {
    if (Test-Path $candidate) {
        $bin = (Resolve-Path $candidate).Path
        break
    }
}
if (-not $bin) {
    Write-Host "==> Building binary..."
    cargo build -p visioflow-cli --no-default-features --quiet
    if ($LASTEXITCODE -ne 0) {
        throw "build failed (exit $LASTEXITCODE)"
    }
    $bin = (Resolve-Path "target\debug\visioflow.exe").Path
}

$tmp = Join-Path $env:TEMP "visioflow-shortcuts-$(Get-Random)"
$launcherRoot = Join-Path $tmp "launchers"
$desktop = Join-Path $tmp "desktop"
$programs = Join-Path $tmp "programs"

New-Item -ItemType Directory -Path $launcherRoot -Force | Out-Null
New-Item -ItemType Directory -Path $desktop -Force | Out-Null
New-Item -ItemType Directory -Path $programs -Force | Out-Null

try {
    Write-Host "==> Installing shortcuts into temp dirs..."
    & ".\scripts\install-shortcuts.ps1" `
        -BinPath $bin `
        -LauncherRoot $launcherRoot `
        -DesktopDir $desktop `
        -StartMenuProgramsDir $programs `
        -Force

    foreach ($name in @("scan-auto", "scan-copy", "scan-plain")) {
        $cmdPath = Join-Path $launcherRoot "$name.cmd"
        Assert-True (Test-Path $cmdPath) "missing launcher: $cmdPath"
        $content = Get-Content -Path $cmdPath -Raw
        Assert-True ($content -match [regex]::Escape($bin)) "launcher does not reference bin: $cmdPath"
        Assert-True ($content -match "capture --source") "launcher missing capture args: $cmdPath"
    }

    $startMenuFolder = Join-Path $programs "VisioFlow"
    foreach ($shortcut in @(
        "VisioFlow Scan (Auto).lnk",
        "VisioFlow Scan (Copy).lnk",
        "VisioFlow Scan (Plain).lnk"
    )) {
        Assert-True (Test-Path (Join-Path $desktop $shortcut)) "missing desktop shortcut: $shortcut"
        Assert-True (Test-Path (Join-Path $startMenuFolder $shortcut)) "missing start menu shortcut: $shortcut"
    }

    Write-Host "All shortcut smoke checks passed."
}
finally {
    Remove-Item -Path $tmp -Recurse -Force -ErrorAction SilentlyContinue
}
