#!/bin/sh
# Parse MATMSG QR payloads and open the default mail client via mailto:.
raw="${QR_RAW:-}"
[ -n "$raw" ] || exit 1
case "$raw" in
  MATMSG:*) ;;
  *) exit 1 ;;
esac

payload="${raw#MATMSG:}"
to="" sub="" body="" cc=""
IFS=';'
for part in $payload; do
  case "$part" in
    TO:*) to="${part#TO:}" ;;
    SUB:*) sub="${part#SUB:}" ;;
    BODY:*) body="${part#BODY:}" ;;
    CC:*) cc="${part#CC:}" ;;
  esac
done
unset IFS

[ -n "$to" ] || exit 1

# Minimal percent-encoding for mailto query values.
encode() {
  printf '%s' "$1" | sed 's/ /%20/g; s/?/%3F/g; s/&/%26/g; s/=/%3D/g'
}

mailto="mailto:$to"
query=""
if [ -n "$sub" ]; then
  query="subject=$(encode "$sub")"
fi
if [ -n "$body" ]; then
  [ -n "$query" ] && query="$query&"
  query="${query}body=$(encode "$body")"
fi
if [ -n "$cc" ]; then
  [ -n "$query" ] && query="$query&"
  query="${query}cc=$(encode "$cc")"
fi
[ -n "$query" ] && mailto="$mailto?$query"

if command -v xdg-open >/dev/null 2>&1; then
  exec xdg-open "$mailto"
elif command -v open >/dev/null 2>&1; then
  exec open "$mailto"
fi
exit 1
