$ErrorActionPreference = "Stop"

$base = "https://raw.githubusercontent.com/WeChatCV/opencv_3rdparty/a8b69ccc738421293254aec5ddb38bd523503252"
$modelsRoot = Join-Path (Join-Path $PSScriptRoot "..") "models"
if (Test-Path $modelsRoot) {
    $modelsDir = (Resolve-Path $modelsRoot).Path
} else {
    $modelsDir = Join-Path (Get-Location) "models"
    New-Item -ItemType Directory -Path $modelsDir -Force | Out-Null
}

$files = @("detect.prototxt", "detect.caffemodel", "sr.prototxt", "sr.caffemodel")
foreach ($file in $files) {
    $destination = Join-Path $modelsDir $file
    Write-Host "Downloading $file..."
    Invoke-WebRequest -Uri "$base/$file" -OutFile $destination
}

Write-Host "WeChat models downloaded to $modelsDir"
