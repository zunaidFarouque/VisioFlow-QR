$target = $env:QR_RAW
if (-not $target) { exit 1 }
Start-Process $target
