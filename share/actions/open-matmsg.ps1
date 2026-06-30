# Parse MATMSG QR payloads (e.g. MATMSG:TO:user@example.com;SUB:Hello;BODY:World;;)
# and open the default mail client via mailto:.
$raw = $env:QR_RAW
if (-not $raw) { exit 1 }
if ($raw -notmatch '^MATMSG:(.*)$') { exit 1 }

$to = $null
$sub = $null
$body = $null
$cc = $null

foreach ($part in ($Matches[1] -split ';')) {
    if ($part -match '^TO:(.*)$') { $to = $Matches[1] }
    elseif ($part -match '^SUB:(.*)$') { $sub = $Matches[1] }
    elseif ($part -match '^BODY:(.*)$') { $body = $Matches[1] }
    elseif ($part -match '^CC:(.*)$') { $cc = $Matches[1] }
}

if (-not $to) { exit 1 }

$mailto = "mailto:$to"
$params = @()
if ($sub) { $params += "subject=$([uri]::EscapeDataString($sub))" }
if ($body) { $params += "body=$([uri]::EscapeDataString($body))" }
if ($cc) { $params += "cc=$([uri]::EscapeDataString($cc))" }
if ($params.Count -gt 0) {
    $mailto += "?" + ($params -join "&")
}

Start-Process $mailto
