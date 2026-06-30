# VisioFlow v2 default-rules smoke (no webcam/OpenCV required).
# Usage:
#   .\scripts\smoke-default-rules.ps1

$ErrorActionPreference = "Stop"
Set-Location (Split-Path $PSScriptRoot -Parent)

function Assert-LastExit($label) {
    if ($LASTEXITCODE -ne 0) {
        throw "$label failed (exit $LASTEXITCODE)"
    }
}

Write-Host "==> Building binary..."
cargo build -p visioflow-cli --no-default-features --quiet
Assert-LastExit "build"

$smokeDir = Join-Path $env:TEMP "visioflow-default-rules-$(Get-Random)"
New-Item -ItemType Directory -Path $smokeDir | Out-Null
$store = Join-Path $smokeDir "rules.json"
$bin = "target\debug\visioflow.exe"

try {
    Write-Host "==> rule init-defaults (temp store)..."
    & $bin rule --store $store init-defaults
    Assert-LastExit "rule init-defaults"

    Write-Host "==> rule list..."
    $list = & $bin rule --store $store list 2>&1 | Out-String
    foreach ($name in @("wifi", "url", "mailto", "plain", "asset")) {
        if ($list -notmatch $name) {
            throw "rule list missing stock rule '$name':`n$list"
        }
    }
    Write-Host "    OK ($($list.Trim().Split("`n").Count) rules)"

    Write-Host "==> rule execute url --no-exec..."
    $out = & $bin rule --store $store execute url --payload "https://example.com" --no-exec 2>&1 | Out-String
    if ($out -notmatch "QR_RAW=https://example.com") {
        throw "url rule execute unexpected output:`n$out"
    }
    Write-Host "    OK"

    Write-Host ""
    Write-Host "All default-rules smoke checks passed."
}
finally {
    Remove-Item -Recurse -Force $smokeDir -ErrorAction SilentlyContinue
}
