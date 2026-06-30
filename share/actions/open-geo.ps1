$lat = $env:QR_NATIVE_GEO_LAT
$lon = $env:QR_NATIVE_GEO_LON
if (-not $lat -or -not $lon) { exit 1 }
$url = "https://www.google.com/maps/search/?api=1&query=$lat,$lon"
Start-Process $url
