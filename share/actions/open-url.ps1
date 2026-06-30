$url = $env:QR_RAW
if (-not $url) { exit 1 }
Start-Process $url
