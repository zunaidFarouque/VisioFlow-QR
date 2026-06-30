#!/bin/sh
lat="${QR_NATIVE_GEO_LAT:-}"
lon="${QR_NATIVE_GEO_LON:-}"
[ -n "$lat" ] && [ -n "$lon" ] || exit 1
url="https://www.google.com/maps/search/?api=1&query=${lat},${lon}"
if command -v xdg-open >/dev/null 2>&1; then
  exec xdg-open "$url"
elif command -v open >/dev/null 2>&1; then
  exec open "$url"
fi
exit 1
