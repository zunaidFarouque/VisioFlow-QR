# VisioFlow Windows dev environment for OpenCV + WeChat webcam builds.
# Usage (each new terminal):
#   . .\scripts\dev-env.ps1
#   cargo run --release -p visioflow-cli -- capture --source webcam --action stdout --verbose

$ErrorActionPreference = "Stop"

$llvmCandidates = @("C:\Program Files\LLVM\bin", "D:\_installed\scoop\apps\llvm\current\bin")
$llvmBin = $llvmCandidates | Where-Object { Test-Path $_ } | Select-Object -First 1
if ($llvmBin) {
    $env:PATH = "$llvmBin;$env:PATH"
} elseif (-not (Get-Command clang -ErrorAction SilentlyContinue)) {
    Write-Warning "LLVM not found — install LLVM or adjust scripts/dev-env.ps1"
}

$vcpkgCandidates = @($env:VCPKG_ROOT, "D:\_installed\scoop\apps\vcpkg\current", "D:\vcpkg")
$vcpkgRoot = $vcpkgCandidates | Where-Object { $_ -and (Test-Path "$_\vcpkg.exe") } | Select-Object -First 1
if ($vcpkgRoot) {
    $env:VCPKG_ROOT = $vcpkgRoot
    $env:VCPKGRS_TRIPLET = "x64-windows-static-md"
} else {
    Write-Warning "vcpkg not found at D:\vcpkg or Scoop install — set VCPKG_ROOT to your vcpkg install"
}

Write-Host "VisioFlow dev env:"
Write-Host "  VCPKG_ROOT      = $($env:VCPKG_ROOT)"
Write-Host "  VCPKGRS_TRIPLET = $($env:VCPKGRS_TRIPLET)"
Write-Host "  LLVM in PATH    = $(if (Get-Command clang -ErrorAction SilentlyContinue) { 'yes' } else { 'no' })"
