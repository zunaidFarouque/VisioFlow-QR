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

    foreach ($name in @("camera-auto", "camera-copy", "snip-auto", "snip-copy")) {
        $launcherPath = Join-Path $launcherRoot "$name.vbs"
        Assert-True (Test-Path $launcherPath) "missing launcher: $launcherPath"
        $content = Get-Content -Path $launcherPath -Raw
        Assert-True ($content -match [regex]::Escape($bin)) "launcher does not reference bin: $launcherPath"
        Assert-True ($content -match "capture --source") "launcher missing capture args: $launcherPath"
    }

    $cameraAuto = Get-Content -Path (Join-Path $launcherRoot "camera-auto.vbs") -Raw
    Assert-True ($cameraAuto -match "--source webcam") "camera-auto missing webcam source"
    $snipAuto = Get-Content -Path (Join-Path $launcherRoot "snip-auto.vbs") -Raw
    Assert-True ($snipAuto -match "--source snip") "snip-auto missing snip source"

    $startMenuFolder = Join-Path $programs "VisioFlow"
    $sampleShortcut = Join-Path $startMenuFolder "VisioFlow QR Camera (auto).lnk"
    $shell = New-Object -ComObject WScript.Shell
    $lnk = $shell.CreateShortcut($sampleShortcut)
    Assert-True ($lnk.TargetPath -match "\.vbs$") "shortcut should target hidden .vbs launcher: $($lnk.TargetPath)"
    Assert-True ($lnk.IconLocation) "shortcut missing IconLocation"
    Assert-True ($lnk.IconLocation -like "$bin*") "shortcut icon should reference visioflow.exe: $($lnk.IconLocation)"

    foreach ($shortcut in @(
        "VisioFlow QR Camera (auto).lnk",
        "VisioFlow QR Camera (copy).lnk",
        "VisioFlow QR Snip (auto).lnk",
        "VisioFlow QR Snip (copy).lnk"
    )) {
        Assert-True (-not (Test-Path (Join-Path $desktop $shortcut))) "desktop shortcut should not exist: $shortcut"
        Assert-True (Test-Path (Join-Path $startMenuFolder $shortcut)) "missing start menu shortcut: $shortcut"
    }

    Write-Host "All shortcut smoke checks passed."
}
finally {
    Remove-Item -Path $tmp -Recurse -Force -ErrorAction SilentlyContinue
}
