#Requires -Version 5.1
<#
.SYNOPSIS
    Automates packaging, GitHub release creation, and Scoop bucket synchronization for VisioFlow.

.DESCRIPTION
    Executes the release pipeline:
    1. Validates preconditions (build output exists, gh CLI authenticated, tag availability).
    2. Packages the staged distribution directory into visioflow-win-x64.zip.
    3. Computes the SHA-256 checksum.
    4. Updates the canonical manifest (visioflow.json) with the new version and checksum.
    5. Commits and pushes the manifest to origin/main.
    6. Publishes the GitHub release with the zip asset attached.
    7. Triggers the sync-visioflow.yml workflow in Zunaid-Scoop-Bucket.
    8. Cleans up the temporary local zip artifact if desired.

.PARAMETER Version
    The new semantic version to release (e.g. "0.2.0"). Do not prefix with 'v'.

.PARAMETER Title
    The release title. Defaults to "VisioFlow v<Version>".

.PARAMETER Notes
    Markdown text for release notes.

.PARAMETER NotesFile
    Path to a markdown file containing release notes.

.PARAMETER BucketRepo
    Target Scoop bucket repository. Defaults to "zunaidFarouque/Zunaid-Scoop-Bucket".

.PARAMETER Force
    Bypasses interactive confirmation. (Required for non-interactive / agent automation).

.PARAMETER DryRun
    Simulates the process without pushing to GitHub, modifying files, or creating releases.

.EXAMPLE
    .\scripts\release.ps1 -Version "0.2.0" -Title "VisioFlow v0.2.0" -Notes "Major overhaul" -Force
#>

[CmdletBinding()]
param(
    [Parameter(Mandatory = $false)]
    [string]$Version,

    [Parameter(Mandatory = $false)]
    [string]$Title,

    [Parameter(Mandatory = $false)]
    [string]$Notes,

    [Parameter(Mandatory = $false)]
    [string]$NotesFile,

    [Parameter(Mandatory = $false)]
    [string]$BucketRepo = "zunaidFarouque/Zunaid-Scoop-Bucket",

    [Parameter(Mandatory = $false)]
    [switch]$Force,

    [Parameter(Mandatory = $false)]
    [switch]$DryRun
)

$ErrorActionPreference = 'Stop'
$repoRoot = Resolve-Path (Join-Path $PSScriptRoot '..')
Set-Location $repoRoot

# ==============================================================================
# CONFIGURATION
# ==============================================================================
$AppName       = "VisioFlow"
$PackageName   = "visioflow"
$DistRelFolder = "dist\visioflow-win-x64"
$MainBinaryRel = "target\release\visioflow.exe"
$BuildCommandHint = "Please run .\scripts\build-release.ps1 -RouterOnly first."
# ==============================================================================

Write-Host "===================================================" -ForegroundColor Cyan
Write-Host "  $AppName Automated Release Pipeline" -ForegroundColor Cyan
Write-Host "===================================================" -ForegroundColor Cyan

# ---------------------------------------------------------
# 1. Version Validation & Pre-flight Checks
# ---------------------------------------------------------
if (-not $Version) {
    $Version = Read-Host "Enter release version (e.g. 0.2.0)"
}
$Version = $Version.Trim().TrimStart('v').TrimStart('V')
if (-not $Version) {
    Write-Error "A valid version string (e.g. 0.2.0) is required."
    exit 1
}

$tag = "v$Version"
if (-not $Title) {
    $Title = "$AppName $tag"
}

# Check: Compiled binary exists
$exePath = Join-Path $repoRoot $MainBinaryRel
if (-not (Test-Path $exePath)) {
    Write-Error "Pre-flight failed: Compiled binary not found at '$exePath'. $BuildCommandHint"
    exit 1
}

# Check: GitHub CLI authenticated
try {
    $null = gh auth status 2>&1
    if ($LASTEXITCODE -ne 0) {
        Write-Error "Pre-flight failed: GitHub CLI (gh) is not authenticated. Please run 'gh auth login'."
        exit 1
    }
} catch {
    Write-Error "Pre-flight failed: GitHub CLI (gh) was not found on PATH. Please install gh."
    exit 1
}

# Check: Tag does not already exist
$existingTag = git tag -l $tag
if ($existingTag) {
    Write-Error "Pre-flight failed: Tag '$tag' already exists locally or remotely. Use a higher version number."
    exit 1
}

# Confirmation Prompt
if (-not $Force -and -not $DryRun) {
    Write-Host "`nRelease Plan Summary:" -ForegroundColor Yellow
    Write-Host "  - Version:      $Version ($tag)"
    Write-Host "  - Title:        $Title"
    Write-Host "  - Binary:       $exePath"
    Write-Host "  - Bucket Repo:  $BucketRepo"
    $confirm = Read-Host "`nAre you sure you want to publish this release to GitHub and trigger Scoop bucket sync? (y/N)"
    if ($confirm -notmatch '^(y|yes)$') {
        Write-Host "Release cancelled by user." -ForegroundColor Gray
        exit 0
    }
}

# ---------------------------------------------------------
# 2. Package Zip Archive
# ---------------------------------------------------------
$zipName = "visioflow-win-x64.zip"
$zipPath = Join-Path $repoRoot "dist\$zipName"

Write-Host "`n[1/6] Packaging portable distribution into '$zipName'..." -ForegroundColor Green
& (Join-Path $PSScriptRoot "build-release.ps1") -RouterOnly

if (-not (Test-Path $zipPath)) {
    Write-Error "Distribution packaging failed: '$zipPath' was not generated."
    exit 1
}

# ---------------------------------------------------------
# 3. Compute SHA-256 Checksum
# ---------------------------------------------------------
Write-Host "[2/6] Computing SHA-256 hash..." -ForegroundColor Green
$sha256 = (Get-FileHash -Path $zipPath -Algorithm SHA256).Hash.ToLower()
Write-Host "      SHA-256: $sha256" -ForegroundColor Cyan

# ---------------------------------------------------------
# 4. Update Canonical Scoop Manifests
# ---------------------------------------------------------
Write-Host "[3/6] Updating canonical Scoop manifests..." -ForegroundColor Green
$manifestPaths = @(
    (Join-Path $repoRoot "$PackageName.json"),
    (Join-Path $repoRoot "scripts\packaging\scoop\$PackageName.json")
)

foreach ($manifestPath in $manifestPaths) {
    if (-not (Test-Path $manifestPath)) {
        continue
    }

    $manifest = Get-Content -LiteralPath $manifestPath -Raw | ConvertFrom-Json
    $manifest.version = $Version

    $githubRepoSlug = "zunaidFarouque/VisioFlow-QR"
    if ($manifest.architecture -and $manifest.architecture.'64bit') {
        $manifest.architecture.'64bit'.url = "https://github.com/$githubRepoSlug/releases/download/$tag/$zipName"
        $manifest.architecture.'64bit'.hash = $sha256
    }

    $jsonString = $manifest | ConvertTo-Json -Depth 10
    $jsonString = $jsonString -replace "`r`n", "`n" -replace "`n", "`r`n"
    if (-not $jsonString.EndsWith("`r`n")) {
        $jsonString += "`r`n"
    }

    if (-not $DryRun) {
        $utf8NoBom = New-Object System.Text.UTF8Encoding($false)
        [System.IO.File]::WriteAllText($manifestPath, $jsonString, $utf8NoBom)
        Write-Host "      Updated $manifestPath" -ForegroundColor Cyan
    }
}

# ---------------------------------------------------------
# 5. Commit & Push Updated Manifest
# ---------------------------------------------------------
Write-Host "[4/6] Committing and pushing updated manifest to main..." -ForegroundColor Green
if (-not $DryRun) {
    git add "$PackageName.json" "scripts/packaging/scoop/$PackageName.json" "Cargo.toml"
    git diff --staged --quiet
    if ($LASTEXITCODE -ne 0) {
        git commit -m "chore(release): Bump Scoop manifest to $tag"
        git push origin main
    } else {
        Write-Host "      Manifest already up to date on main." -ForegroundColor Gray
    }
} else {
    Write-Host "      [DryRun] Skipped git commit and push." -ForegroundColor Gray
}

# ---------------------------------------------------------
# 6. Publish GitHub Release
# ---------------------------------------------------------
Write-Host "[5/6] Publishing release '$tag' on GitHub..." -ForegroundColor Green
if (-not $DryRun) {
    $ghArgs = @("release", "create", $tag, $zipPath, "--title", $Title)
    if ($NotesFile -and (Test-Path $NotesFile)) {
        $ghArgs += @("--notes-file", $NotesFile)
    } elseif ($Notes) {
        $ghArgs += @("--notes", $Notes)
    } else {
        $ghArgs += @("--generate-notes")
    }

    & gh @ghArgs
    if ($LASTEXITCODE -ne 0) {
        Write-Error "Failed to publish GitHub release."
        exit 1
    }
    Write-Host "      Release $tag successfully published!" -ForegroundColor Cyan
} else {
    Write-Host "      [DryRun] Skipped gh release create." -ForegroundColor Gray
}

# ---------------------------------------------------------
# 7. Trigger Bucket Sync
# ---------------------------------------------------------
Write-Host "[6/6] Triggering Scoop bucket sync in '$BucketRepo'..." -ForegroundColor Green
$workflowName = "sync-$PackageName.yml"
if (-not $DryRun) {
    try {
        gh workflow run $workflowName -R $BucketRepo
        Write-Host "      Successfully dispatched $workflowName to $BucketRepo!" -ForegroundColor Cyan
    } catch {
        Write-Warning "Failed to trigger workflow in $BucketRepo. You can manually run: gh workflow run $workflowName -R $BucketRepo"
    }
} else {
    Write-Host "      [DryRun] Skipped workflow dispatch ($workflowName)." -ForegroundColor Gray
}

Write-Host "`n===================================================" -ForegroundColor Green
Write-Host "  $AppName $tag Release Complete!" -ForegroundColor Green
Write-Host "===================================================" -ForegroundColor Green
