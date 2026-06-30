use std::process::Command;

use crate::error::{Result, VisioFlowError};

pub struct PlatformExecutor;

fn nmcli_available() -> bool {
    Command::new("nmcli")
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

/// Args for `nmcli device wifi connect` (password omitted for open networks).
#[must_use]
pub(crate) fn nmcli_wifi_connect_args(ssid: &str, password: &str) -> Vec<String> {
    let mut args = vec![
        "device".to_owned(),
        "wifi".to_owned(),
        "connect".to_owned(),
        ssid.to_owned(),
    ];
    if !password.is_empty() {
        args.push("password".to_owned());
        args.push(password.to_owned());
    }
    args
}

fn connect_via_nmcli(ssid: &str, password: &str) -> Result<()> {
    let args = nmcli_wifi_connect_args(ssid, password);
    let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();

    let output = Command::new("nmcli")
        .args(&arg_refs)
        .output()
        .map_err(|e| VisioFlowError::Capture(format!("nmcli spawn failed: {e}")))?;

    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    Err(VisioFlowError::Capture(format!(
        "nmcli device wifi connect failed: {stdout}{stderr}"
    )))
}

impl super::SystemExecutor for PlatformExecutor {
    /// Connect via NetworkManager (`nmcli device wifi connect`).
    ///
    /// When `nmcli` is unavailable, returns an error documenting that a manual
    /// `wpa_supplicant` workflow is required (not automated here).
    fn connect_wifi(&self, ssid: &str, password: &str) -> Result<()> {
        if nmcli_available() {
            return connect_via_nmcli(ssid, password);
        }

        Err(VisioFlowError::Capture(
            "nmcli not found: install NetworkManager or connect manually with wpa_supplicant \
             (wpa_passphrase + wpa_supplicant -B -i <iface> -c <conf>; then dhclient <iface>)"
                .into(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nmcli_wifi_connect_args_includes_password_when_set() {
        let args = nmcli_wifi_connect_args("lab", "secret");
        assert_eq!(
            args,
            vec![
                "device",
                "wifi",
                "connect",
                "lab",
                "password",
                "secret"
            ]
        );
    }

    #[test]
    fn nmcli_wifi_connect_args_omits_password_for_open_network() {
        let args = nmcli_wifi_connect_args("OpenNet", "");
        assert_eq!(args, vec!["device", "wifi", "connect", "OpenNet"]);
    }
}
