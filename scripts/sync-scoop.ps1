#Requires -Version 5.1
<#
.SYNOPSIS
    Quickly triggers the Scoop bucket sync workflow for VisioFlow.

.DESCRIPTION
    Dispatches the `sync-visioflow.yml` GitHub Actions workflow in `zunaidFarouque/Zunaid-Scoop-Bucket`.
    Optionally pulls latest changes into the local Scoop bucket clone if found.

.PARAMETER Watch
    Watches the GitHub Actions workflow run in real-time until completion.

.PARAMETER LocalBucketDir
    Path to local clone of Zunaid-Scoop-Bucket. Defaults to sibling repo folder if present.

.EXAMPLE
    .\scripts\sync-scoop.ps1
    .\scripts\sync-scoop.ps1 -Watch
#>

[CmdletBinding()]
param(
    [switch]$Watch,
    [string]$LocalBucketDir = "..\..\..\Zunaid-Scoop-Bucket"
)

$ErrorActionPreference = 'Stop'
$BucketRepo = "zunaidFarouque/Zunaid-Scoop-Bucket"
$Workflow = "sync-visioflow.yml"

if (-not (Get-Command gh -ErrorAction SilentlyContinue)) {
    Write-Error "GitHub CLI (gh) is not installed or not on PATH."
    exit 1
}

Write-Host "Triggering Scoop bucket sync on $BucketRepo..." -ForegroundColor Cyan
& gh workflow run $Workflow -R $BucketRepo
if ($LASTEXITCODE -ne 0) {
    Write-Error "Failed to dispatch $Workflow on $BucketRepo."
    exit 1
}

Write-Host "Sync workflow successfully triggered!" -ForegroundColor Green
Start-Sleep -Seconds 2

Write-Host "`nRecent run status:" -ForegroundColor Yellow
& gh run list -R $BucketRepo --workflow $Workflow --limit 1

if ($Watch) {
    Write-Host "`nWatching workflow run live..." -ForegroundColor Cyan
    & gh run watch -R $BucketRepo
} else {
    Write-Host "`nTo watch live in terminal, run: gh run watch -R $BucketRepo" -ForegroundColor Gray
}

# If local bucket repo is present, offer or perform git pull
$resolvedLocalBucket = $null
if (Test-Path $LocalBucketDir) {
    $resolvedLocalBucket = (Resolve-Path $LocalBucketDir).Path
} elseif (Test-Path "D:\_installed\VScode repos\Zunaid-Scoop-Bucket") {
    $resolvedLocalBucket = "D:\_installed\VScode repos\Zunaid-Scoop-Bucket"
}

if ($resolvedLocalBucket -and (Test-Path (Join-Path $resolvedLocalBucket ".git"))) {
    Write-Host "`nLocal Scoop bucket found at: $resolvedLocalBucket" -ForegroundColor Cyan
    Write-Host "Pulling latest changes locally..." -ForegroundColor Gray
    git -C $resolvedLocalBucket pull
}
