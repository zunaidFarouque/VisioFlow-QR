$ssid = $env:QR_NATIVE_WIFI_SSID
$password = $env:QR_NATIVE_WIFI_PASSWORD
$mode = $env:VISIOFLOW_WIFI_HANDOFF_MODE
if (-not $mode) { $mode = "open-settings" }

if (-not $ssid) {
    Write-Error "Missing QR_NATIVE_WIFI_SSID"
    exit 1
}

if ($mode -eq "print") {
    Write-Output "WIFI_SSID=$ssid"
    if ($password) {
        Write-Output "WIFI_PASSWORD=$password"
    } else {
        Write-Output "WIFI_PASSWORD="
    }
    exit 0
}

try {
    Add-Type -AssemblyName System.Windows.Forms | Out-Null
    Add-Type -AssemblyName System.Drawing | Out-Null
} catch {
    # If forms are unavailable, still open settings and copy password.
}

if ($password) {
    Set-Clipboard -Value $password
}

Start-Process "ms-settings:network-wifi" | Out-Null

if ("System.Windows.Forms.Form" -as [type]) {
    $form = New-Object System.Windows.Forms.Form
    $form.Text = "VisioFlow WiFi Handoff"
    $form.StartPosition = "CenterScreen"
    $form.Size = New-Object System.Drawing.Size(520, 220)
    $form.TopMost = $true

    $label = New-Object System.Windows.Forms.Label
    $label.AutoSize = $true
    $label.Location = New-Object System.Drawing.Point(20, 20)
    $label.Text = "Select WiFi network '$ssid' in Settings. Password is copied to clipboard."
    $form.Controls.Add($label)

    $ssidBox = New-Object System.Windows.Forms.TextBox
    $ssidBox.Location = New-Object System.Drawing.Point(20, 60)
    $ssidBox.Size = New-Object System.Drawing.Size(350, 24)
    $ssidBox.ReadOnly = $true
    $ssidBox.Text = $ssid
    $form.Controls.Add($ssidBox)

    $copySsid = New-Object System.Windows.Forms.Button
    $copySsid.Location = New-Object System.Drawing.Point(380, 58)
    $copySsid.Size = New-Object System.Drawing.Size(110, 28)
    $copySsid.Text = "Copy SSID"
    $copySsid.Add_Click({ Set-Clipboard -Value $ssidBox.Text })
    $form.Controls.Add($copySsid)

    $pwdBox = New-Object System.Windows.Forms.TextBox
    $pwdBox.Location = New-Object System.Drawing.Point(20, 100)
    $pwdBox.Size = New-Object System.Drawing.Size(350, 24)
    $pwdBox.ReadOnly = $true
    $pwdBox.Text = $password
    $form.Controls.Add($pwdBox)

    $copyPwd = New-Object System.Windows.Forms.Button
    $copyPwd.Location = New-Object System.Drawing.Point(380, 98)
    $copyPwd.Size = New-Object System.Drawing.Size(110, 28)
    $copyPwd.Text = "Copy Password"
    $copyPwd.Add_Click({ Set-Clipboard -Value $pwdBox.Text })
    $form.Controls.Add($copyPwd)

    $closeBtn = New-Object System.Windows.Forms.Button
    $closeBtn.Location = New-Object System.Drawing.Point(380, 140)
    $closeBtn.Size = New-Object System.Drawing.Size(110, 28)
    $closeBtn.Text = "Done"
    $closeBtn.Add_Click({ $form.Close() })
    $form.Controls.Add($closeBtn)

    [void]$form.ShowDialog()
}
