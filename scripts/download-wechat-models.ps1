# Download OpenCV WeChat QR CNN model files.
# Usage:
#   .\scripts\download-wechat-models.ps1
#   .\scripts\download-wechat-models.ps1 -ModelsDir "D:\dist\visioflow-win-x64\models"

param(
    [string]$ModelsDir
)

$ErrorActionPreference = "Stop"

$base = "https://raw.githubusercontent.com/WeChatCV/opencv_3rdparty/a8b69ccc738421293254aec5ddb38bd523503252"

if ($ModelsDir) {
    $modelsDir = $ModelsDir
} else {
    $modelsRoot = Join-Path (Join-Path $PSScriptRoot "..") "models"
    if (Test-Path $modelsRoot) {
        $modelsDir = (Resolve-Path $modelsRoot).Path
    } else {
        $modelsDir = Join-Path (Get-Location) "models"
    }
}

New-Item -ItemType Directory -Path $modelsDir -Force | Out-Null

$files = @("detect.prototxt", "detect.caffemodel", "sr.prototxt", "sr.caffemodel")
foreach ($file in $files) {
    $destination = Join-Path $modelsDir $file
    if (Test-Path $destination) {
        Write-Host "Skipping $file (already exists)"
        continue
    }
    Write-Host "Downloading $file..."
    Invoke-WebRequest -Uri "$base/$file" -OutFile $destination
}

foreach ($file in $files) {
    $destination = Join-Path $modelsDir $file
    if (-not (Test-Path $destination)) {
        throw "Missing model file after download: $destination"
    }
}

Write-Host "WeChat models ready at $modelsDir"
