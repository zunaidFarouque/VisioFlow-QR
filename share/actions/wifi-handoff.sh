#!/bin/sh
ssid="${QR_NATIVE_WIFI_SSID:-}"
password="${QR_NATIVE_WIFI_PASSWORD:-}"
mode="${VISIOFLOW_WIFI_HANDOFF_MODE:-open-settings}"

[ -n "$ssid" ] || {
  echo "Missing QR_NATIVE_WIFI_SSID" >&2
  exit 1
}

if [ "$mode" = "print" ]; then
  printf 'WIFI_SSID=%s\n' "$ssid"
  printf 'WIFI_PASSWORD=%s\n' "$password"
  exit 0
fi

printf 'WiFi handoff:\n'
printf '  SSID: %s\n' "$ssid"
printf '  Password: %s\n' "$password"
printf 'Open your WiFi settings and connect manually.\n'

if command -v wl-copy >/dev/null 2>&1; then
  printf '%s' "$password" | wl-copy
elif command -v xclip >/dev/null 2>&1; then
  printf '%s' "$password" | xclip -selection clipboard
fi

if command -v xdg-open >/dev/null 2>&1; then
  xdg-open "settings://network/wifi" >/dev/null 2>&1 || true
fi
