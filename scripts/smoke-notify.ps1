# VisioFlow Windows toast notification smoke test.
# Usage:
#   .\scripts\smoke-notify.ps1

$ErrorActionPreference = "Stop"
Set-Location (Split-Path $PSScriptRoot -Parent)

function Assert-True($condition, $message) {
    if (-not $condition) {
        throw $message
    }
}

$binCandidates = @(
    "target\debug\visioflow.exe",
    "target\release\visioflow.exe"
)
$bin = $null
foreach ($candidate in $binCandidates) {
    if (Test-Path $candidate) {
        $candidateBin = (Resolve-Path $candidate).Path
        & $candidateBin notify test --help *> $null
        if ($LASTEXITCODE -eq 0) {
            $bin = $candidateBin
            break
        }
    }
}
if (-not $bin) {
    cargo build -p visioflow-cli --no-default-features --quiet
    if ($LASTEXITCODE -ne 0) {
        throw "build failed (exit $LASTEXITCODE)"
    }
    $bin = (Resolve-Path "target\debug\visioflow.exe").Path
}

Write-Host "==> Running toast smoke via: $bin notify test --verbose"
& $bin notify test --title "VisioFlow Smoke" --body "If you see this toast, notifications work." --verbose
if ($LASTEXITCODE -ne 0) {
    throw "notify test failed (exit $LASTEXITCODE)"
}

Write-Host "Toast smoke command succeeded (check Action Center for the notification)."
