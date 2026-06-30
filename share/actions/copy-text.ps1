$text = $env:QR_VAR_TEXT
if (-not $text) { $text = $env:QR_RAW }
if (-not $text) { exit 1 }
Set-Clipboard -Value $text
