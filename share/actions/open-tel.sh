#!/bin/sh
target="${QR_RAW:-}"
[ -n "$target" ] || exit 1
if command -v xdg-open >/dev/null 2>&1; then
  exec xdg-open "$target"
elif command -v open >/dev/null 2>&1; then
  exec open "$target"
fi
exit 1
