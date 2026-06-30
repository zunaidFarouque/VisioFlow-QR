# VisioFlow distribution/install smoke test (Windows-first).
# Usage:
#   .\scripts\smoke-distribution.ps1

$ErrorActionPreference = "Stop"
Set-Location (Split-Path $PSScriptRoot -Parent)

function Assert-True($condition, $message) {
    if (-not $condition) {
        throw $message
    }
}

function Assert-Contains([string]$text, [string]$needle, [string]$label) {
    if ($text -notmatch [regex]::Escape($needle)) {
        throw "$label missing '$needle'"
    }
}

$manifestPath = "scripts\packaging\scoop\visioflow.json"
$installerPath = "scripts\install-traditional.ps1"
$bootstrapPath = "scripts\bootstrap-portable.ps1"
if ($bootstrapPath -is [string]) {
    $bootstrapPath = (Resolve-Path $bootstrapPath).Path
}


Assert-True (Test-Path $manifestPath) "missing scoop manifest: $manifestPath"
Assert-True (Test-Path $installerPath) "missing install script: $installerPath"
Assert-True (Test-Path $bootstrapPath) "missing bootstrap script: $bootstrapPath"

$manifestRaw = Get-Content -Path $manifestPath -Raw
$manifest = $manifestRaw | ConvertFrom-Json

Assert-True ($manifest.version) "scoop manifest: version missing"
$scoopUrl = $manifest.url
if (-not $scoopUrl -and $manifest.architecture) {
    $scoopUrl = $manifest.architecture.'64bit'.url
}
Assert-True $scoopUrl "scoop manifest: url missing"
Assert-True ($manifest.bin) "scoop manifest: bin missing"
Assert-Contains $manifestRaw "shortcuts" "scoop manifest"
Assert-Contains $manifestRaw "VisioFlow QR Camera (auto)" "scoop manifest"
Assert-Contains $manifestRaw "uninstaller" "scoop manifest"

$tmp = Join-Path $env:TEMP "visioflow-dist-smoke-$(Get-Random)"
$distRoot = Join-Path $tmp "dist"
$installRoot = Join-Path $tmp "install"
$desktop = Join-Path $tmp "desktop"
$programs = Join-Path $tmp "programs"
$appData = Join-Path $tmp "appdata"

New-Item -ItemType Directory -Path $distRoot -Force | Out-Null
New-Item -ItemType Directory -Path $installRoot -Force | Out-Null
New-Item -ItemType Directory -Path $desktop -Force | Out-Null
New-Item -ItemType Directory -Path $programs -Force | Out-Null
New-Item -ItemType Directory -Path $appData -Force | Out-Null

$binCandidates = @(
    "target\release\visioflow.exe",
    "target\debug\visioflow.exe"
)
$bin = $null
foreach ($candidate in $binCandidates) {
    if (Test-Path $candidate) {
        $bin = (Resolve-Path $candidate).Path
        break
    }
}
if (-not $bin) {
    cargo build -p visioflow-cli --no-default-features --quiet
    if ($LASTEXITCODE -ne 0) {
        throw "build failed (exit $LASTEXITCODE)"
    }
    $bin = (Resolve-Path "target\debug\visioflow.exe").Path
}

Copy-Item -Path $bin -Destination (Join-Path $distRoot "visioflow.exe")
$toastBin = Join-Path (Split-Path $bin -Parent) "visioflow-toast.exe"
if (Test-Path $toastBin) {
    Copy-Item -Path $toastBin -Destination (Join-Path $distRoot "visioflow-toast.exe")
}
Copy-Item -Path "assets\default-rules.json" -Destination (Join-Path $distRoot "default-rules.json")
Copy-Item -Path "assets\VisiFlow-QR.ico" -Destination (Join-Path $distRoot "VisiFlow-QR.ico")
Copy-Item -Path "scripts\install-shortcuts.ps1" -Destination (Join-Path $distRoot "install-shortcuts.ps1")
Copy-Item -Path $bootstrapPath -Destination (Join-Path $distRoot "bootstrap-portable.ps1")

$modelsDir = Join-Path $distRoot "models"
New-Item -ItemType Directory -Path $modelsDir -Force | Out-Null
foreach ($modelFile in @("detect.prototxt", "detect.caffemodel", "sr.prototxt", "sr.caffemodel")) {
    Set-Content -Path (Join-Path $modelsDir $modelFile) -Value "stub" -NoNewline
}

try {
    & ".\scripts\install-traditional.ps1" `
        -DistRoot $distRoot `
        -InstallRoot $installRoot `
        -DesktopDir $desktop `
        -StartMenuProgramsDir $programs `
        -AppDataDir $appData `
        -Force

    $installedBin = Join-Path $installRoot "visioflow.exe"
    Assert-True (Test-Path $installedBin) "traditional install did not place visioflow.exe"
    $installedToast = Join-Path $installRoot "visioflow-toast.exe"
    if (Test-Path (Join-Path $distRoot "visioflow-toast.exe")) {
        Assert-True (Test-Path $installedToast) "traditional install did not place visioflow-toast.exe"
    }
    Assert-True (Test-Path (Join-Path $installRoot "share\default-rules.json")) "traditional install missing default rules"
    Assert-True (Test-Path (Join-Path $installRoot "models\detect.caffemodel")) "traditional install missing models"
    Assert-True (Test-Path (Join-Path $installRoot "VisiFlow-QR.ico")) "traditional install missing VisiFlow-QR.ico"

    & $bootstrapPath `
        -DistRoot $distRoot `
        -DesktopDir $desktop `
        -StartMenuProgramsDir $programs `
        -AppDataDir $appData `
        -Force

    foreach ($name in @("camera-auto", "camera-copy", "snip-auto", "snip-copy")) {
        Assert-True (Test-Path (Join-Path $appData "VisioFlow\launchers\$name.cmd")) "portable bootstrap missing launcher $name.cmd"
    }

    $startMenuFolder = Join-Path $programs "VisioFlow"
    foreach ($shortcut in @(
        "VisioFlow QR Camera (auto).lnk",
        "VisioFlow QR Snip (copy).lnk"
    )) {
        Assert-True (-not (Test-Path (Join-Path $desktop $shortcut))) "desktop shortcut should not exist: $shortcut"
        Assert-True (Test-Path (Join-Path $startMenuFolder $shortcut)) "missing start menu shortcut: $shortcut"
    }
    Assert-True (Test-Path (Join-Path $appData "visioflow\rules.json")) "portable bootstrap missing rules store"

    & (Join-Path $PSScriptRoot "test-scoop-manifest.ps1")

    Write-Host "All distribution smoke checks passed."
}
finally {
    Remove-Item -Path $tmp -Recurse -Force -ErrorAction SilentlyContinue
}
