# VisioFlow router-phase smoke test (no webcam/OpenCV required).
# Usage:
#   .\scripts\smoke-router.ps1

$ErrorActionPreference = "Stop"
Set-Location (Split-Path $PSScriptRoot -Parent)

function Assert-LastExit($label) {
    if ($LASTEXITCODE -ne 0) {
        throw "$label failed (exit $LASTEXITCODE)"
    }
}

Write-Host "==> Automated tests (core)..."
cargo test -p visioflow-core --quiet
Assert-LastExit "visioflow-core tests"

Write-Host "==> Automated tests (CLI, no opencv-webcam)..."
cargo test -p visioflow-cli --no-default-features --quiet
Assert-LastExit "visioflow-cli tests"

Write-Host "==> Building binary..."
cargo build -p visioflow-cli --no-default-features --quiet
Assert-LastExit "build"

$smokeDir = Join-Path $env:TEMP "visioflow-smoke-$(Get-Random)"
New-Item -ItemType Directory -Path $smokeDir | Out-Null
$store = Join-Path $smokeDir "rules.json"
$bin = "target\debug\visioflow.exe"

try {
    Write-Host "==> rule create / config / execute..."
    & $bin rule --store $store create asset
    Assert-LastExit "rule create"
    & $bin rule --store $store config asset --regex "ASSET:(?P<asset>\d+)" --map asset:ASSET
    Assert-LastExit "rule config"

    $out = & $bin rule --store $store execute asset --payload "ASSET:99" --no-exec 2>&1 | Out-String
    if ($out -notmatch "QR_VAR_ASSET=99") {
        throw "rule execute unexpected output:`n$out"
    }
    Write-Host "    OK"

    Write-Host "==> --export bash (rule execute)..."
    $out = & $bin --export bash rule --store $store execute asset --payload "ASSET:99" --no-exec 2>&1 | Out-String
    if ($out -notmatch "export QR_VAR_ASSET='99'") {
        throw "export bash unexpected output:`n$out"
    }
    Write-Host "    OK"

    Write-Host "==> rule list / delete..."
    $list = & $bin rule --store $store list 2>&1 | Out-String
    if ($list -notmatch "asset") { throw "rule list missing asset:`n$list" }
    & $bin rule --store $store delete asset
    Assert-LastExit "rule delete"
    Write-Host "    OK"

    Write-Host "==> capture --trigger via integration test (generates QR fixture)..."
    cargo test -p visioflow-cli --no-default-features capture_trigger --quiet
    Assert-LastExit "capture_trigger integration tests"
    Write-Host "    OK"

    Write-Host ""
    Write-Host "All router smoke checks passed."
    Write-Host ""
    Write-Host "Manual snip note:"
    Write-Host "  Rule 'asset' in %APPDATA%\visioflow\rules.json expects payload ASSET:<digits>."
    Write-Host "  Snip a QR encoding exactly that (e.g. ASSET:42), or use a rule without --regex."
    Write-Host "  Add --verbose to see decoded payload before routing."
}
finally {
    Remove-Item -Recurse -Force $smokeDir -ErrorAction SilentlyContinue
}
