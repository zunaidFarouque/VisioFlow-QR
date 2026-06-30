use std::fs;
use std::process::Command;

use tempfile::NamedTempFile;

use crate::error::{Result, VisioFlowError};

pub struct PlatformExecutor;

/// Build a WLAN profile XML for `netsh wlan add profile`.
///
/// Open networks omit the shared key; WPA2-PSK is used when a password is present.
#[must_use]
pub(crate) fn wlan_profile_xml(ssid: &str, password: &str) -> String {
    let escaped_ssid = xml_escape(ssid);
    if password.is_empty() {
        return format!(
            r#"<?xml version="1.0"?>
<WLANProfile xmlns="http://www.microsoft.com/networking/WLAN/profile/v1">
    <name>{escaped_ssid}</name>
    <SSIDConfig>
        <SSID>
            <name>{escaped_ssid}</name>
        </SSID>
    </SSIDConfig>
    <connectionType>ESS</connectionType>
    <connectionMode>auto</connectionMode>
    <MSM>
        <security>
            <authEncryption>
                <authentication>open</authentication>
                <encryption>none</encryption>
                <useOneX>false</useOneX>
            </authEncryption>
        </security>
    </MSM>
</WLANProfile>
"#
        );
    }

    let escaped_password = xml_escape(password);
    format!(
        r#"<?xml version="1.0"?>
<WLANProfile xmlns="http://www.microsoft.com/networking/WLAN/profile/v1">
    <name>{escaped_ssid}</name>
    <SSIDConfig>
        <SSID>
            <name>{escaped_ssid}</name>
        </SSID>
    </SSIDConfig>
    <connectionType>ESS</connectionType>
    <connectionMode>auto</connectionMode>
    <MSM>
        <security>
            <authEncryption>
                <authentication>WPA2PSK</authentication>
                <encryption>AES</encryption>
                <useOneX>false</useOneX>
            </authEncryption>
            <sharedKey>
                <keyType>passPhrase</keyType>
                <protected>false</protected>
                <keyMaterial>{escaped_password}</keyMaterial>
            </sharedKey>
        </security>
    </MSM>
</WLANProfile>
"#
    )
}

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn run_netsh(args: &[&str]) -> Result<()> {
    let output = Command::new("netsh")
        .args(args)
        .output()
        .map_err(|e| VisioFlowError::Capture(format!("netsh spawn failed: {e}")))?;

    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    Err(VisioFlowError::Capture(format!(
        "netsh wlan failed (admin may be required): {stdout}{stderr}"
    )))
}

impl super::SystemExecutor for PlatformExecutor {
    /// Connect via `netsh wlan add profile` + `netsh wlan connect`.
    ///
    /// Adding a profile may require an elevated (Administrator) shell on some Windows
    /// versions. Existing saved profiles are updated in place for the current user.
    fn connect_wifi(&self, ssid: &str, password: &str) -> Result<()> {
        let xml = wlan_profile_xml(ssid, password);
        let tmp = NamedTempFile::new().map_err(VisioFlowError::Io)?;
        fs::write(tmp.path(), xml).map_err(VisioFlowError::Io)?;

        let profile_path = tmp.path().to_string_lossy();
        run_netsh(&[
            "wlan",
            "add",
            "profile",
            &format!("filename={profile_path}"),
            "user=current",
        ])?;
        run_netsh(&["wlan", "connect", &format!("name={ssid}")])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wlan_profile_xml_open_network_omits_shared_key() {
        let xml = wlan_profile_xml("OpenNet", "");
        assert!(xml.contains("<authentication>open</authentication>"));
        assert!(!xml.contains("sharedKey"));
        assert!(xml.contains("<name>OpenNet</name>"));
    }

    #[test]
    fn wlan_profile_xml_wpa_includes_password() {
        let xml = wlan_profile_xml("MyHome", "secret123");
        assert!(xml.contains("<authentication>WPA2PSK</authentication>"));
        assert!(xml.contains("<keyMaterial>secret123</keyMaterial>"));
    }

    #[test]
    fn wlan_profile_xml_escapes_special_chars() {
        let xml = wlan_profile_xml("Cafe&Co", "p\"ss");
        assert!(xml.contains("Cafe&amp;Co"));
        assert!(xml.contains("p&quot;ss"));
    }
}
