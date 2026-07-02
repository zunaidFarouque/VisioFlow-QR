# Build and stage a Windows x64 release zip for VisioFlow-QR.
# Usage:
#   .\scripts\build-release.ps1
#   .\scripts\build-release.ps1 -RouterOnly
#   .\scripts\build-release.ps1 -VcpkgRoot "D:\vcpkg"
#
# After the zip is created, update scripts/packaging/scoop/visioflow.json
# with the printed SHA256 hash before publishing a Scoop manifest.

param(
    [string]$VcpkgRoot = "D:\vcpkg",
    [string]$VcpkgTriplet = "x64-windows-static-md",
    [string]$OutDir = "dist\visioflow-win-x64",
    [string]$ZipPath = "dist\visioflow-win-x64.zip",
    [switch]$RouterOnly,
    [switch]$SkipZip
)

$ErrorActionPreference = "Stop"
Set-Location (Split-Path $PSScriptRoot -Parent)

function Assert-LastExit([string]$Label) {
    if ($LASTEXITCODE -ne 0) {
        throw "$Label failed (exit $LASTEXITCODE)"
    }
}

$env:VCPKG_ROOT = $VcpkgRoot
$env:VCPKGRS_TRIPLET = $VcpkgTriplet

Write-Host "==> Building visioflow-cli (release)..."
if ($RouterOnly) {
    cargo build --release -p visioflow-cli --no-default-features
} else {
    cargo build --release -p visioflow-cli
}
Assert-LastExit "cargo build"

$releaseDir = Join-Path (Resolve-Path "target\release").Path ""
$mainBin = Join-Path $releaseDir "visioflow.exe"
$toastBin = Join-Path $releaseDir "visioflow-toast.exe"

if (-not (Test-Path $mainBin)) {
    throw "Missing release binary: $mainBin"
}
if (-not (Test-Path $toastBin)) {
    throw "Missing release binary: $toastBin"
}

Write-Host "==> Staging $OutDir ..."
if (Test-Path $OutDir) {
    Remove-Item -Path $OutDir -Recurse -Force
}
New-Item -ItemType Directory -Path $OutDir -Force | Out-Null
New-Item -ItemType Directory -Path (Join-Path $OutDir "share\actions") -Force | Out-Null

Copy-Item -Path $mainBin -Destination (Join-Path $OutDir "visioflow.exe") -Force
Copy-Item -Path $toastBin -Destination (Join-Path $OutDir "visioflow-toast.exe") -Force
Copy-Item -Path "assets\default-rules.json" -Destination (Join-Path $OutDir "default-rules.json") -Force
Copy-Item -Path "assets\logo v2.ico" -Destination (Join-Path $OutDir "logo v2.ico") -Force
Copy-Item -Path "share\actions\*" -Destination (Join-Path $OutDir "share\actions") -Recurse -Force
Copy-Item -Path "scripts\install-shortcuts.ps1" -Destination (Join-Path $OutDir "install-shortcuts.ps1") -Force
Copy-Item -Path "scripts\bootstrap-portable.ps1" -Destination (Join-Path $OutDir "bootstrap-portable.ps1") -Force
Copy-Item -Path "scripts\install-traditional.ps1" -Destination (Join-Path $OutDir "install-traditional.ps1") -Force
Copy-Item -Path "scripts\download-wechat-models.ps1" -Destination (Join-Path $OutDir "download-wechat-models.ps1") -Force

if (-not $RouterOnly) {
    Write-Host "==> Downloading WeChat models into $OutDir\models ..."
    & (Join-Path $PSScriptRoot "download-wechat-models.ps1") -ModelsDir (Join-Path $OutDir "models")

    $modelFiles = @("detect.prototxt", "detect.caffemodel", "sr.prototxt", "sr.caffemodel")
    foreach ($modelFile in $modelFiles) {
        $modelPath = Join-Path (Join-Path $OutDir "models") $modelFile
        if (-not (Test-Path $modelPath)) {
            throw "Missing staged model file: $modelPath"
        }
    }
}

if ($SkipZip) {
    Write-Host "Staging complete (zip skipped): $OutDir"
    exit 0
}

$zipParent = Split-Path -Parent $ZipPath
if ($zipParent -and -not (Test-Path $zipParent)) {
    New-Item -ItemType Directory -Path $zipParent -Force | Out-Null
}
if (Test-Path $ZipPath) {
    Remove-Item -Path $ZipPath -Force
}

Write-Host "==> Creating $ZipPath ..."
Compress-Archive -Path $OutDir -DestinationPath $ZipPath -Force

$hash = (Get-FileHash -Path $ZipPath -Algorithm SHA256).Hash.ToLower()
Write-Host ""
Write-Host "Release zip: $((Resolve-Path $ZipPath).Path)"
Write-Host "SHA256:      $hash"
Write-Host ""
Write-Host "Update scripts/packaging/scoop/visioflow.json:"
Write-Host "  architecture.64bit.hash = $hash"
