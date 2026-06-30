#!/bin/sh
text="${QR_VAR_TEXT:-$QR_RAW}"
[ -n "$text" ] || exit 1
if command -v wl-copy >/dev/null 2>&1; then
  printf '%s' "$text" | wl-copy
elif command -v xclip >/dev/null 2>&1; then
  printf '%s' "$text" | xclip -selection clipboard
elif command -v pbcopy >/dev/null 2>&1; then
  printf '%s' "$text" | pbcopy
else
  exit 1
fi
