#!/bin/sh
url="${QR_RAW:-}"
[ -n "$url" ] || exit 1
if command -v xdg-open >/dev/null 2>&1; then
  exec xdg-open "$url"
elif command -v open >/dev/null 2>&1; then
  exec open "$url"
fi
exit 1
