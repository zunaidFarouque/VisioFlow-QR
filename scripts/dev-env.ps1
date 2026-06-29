# VisioFlow Windows dev environment for OpenCV + WeChat webcam builds.
# Usage (each new terminal):
#   . .\scripts\dev-env.ps1
#   cargo run --release -p visioflow-cli -- capture --source webcam --action stdout --verbose

$ErrorActionPreference = "Stop"

$llvmBin = "C:\Program Files\LLVM\bin"
if (Test-Path $llvmBin) {
    $env:PATH = "$llvmBin;$env:PATH"
} else {
    Write-Warning "LLVM not found at $llvmBin — install LLVM or adjust scripts/dev-env.ps1"
}

$vcpkgRoot = "D:\vcpkg"
if (Test-Path "$vcpkgRoot\vcpkg.exe") {
    $env:VCPKG_ROOT = $vcpkgRoot
    $env:VCPKGRS_TRIPLET = "x64-windows-static-md"
} else {
    Write-Warning "vcpkg not found at $vcpkgRoot — set VCPKG_ROOT to your vcpkg install"
}

Write-Host "VisioFlow dev env:"
Write-Host "  VCPKG_ROOT      = $($env:VCPKG_ROOT)"
Write-Host "  VCPKGRS_TRIPLET = $($env:VCPKGRS_TRIPLET)"
Write-Host "  LLVM in PATH    = $(if (Get-Command clang -ErrorAction SilentlyContinue) { 'yes' } else { 'no' })"
