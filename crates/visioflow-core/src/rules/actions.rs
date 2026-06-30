//! Post-route rule actions: WiFi connect and child-process exec.

use std::path::Path;
use std::process::Command;

use crate::sys::SystemExecutor;

use super::error::{Result as RuleResult, RuleError};
use super::model::{ResolvedVars, Rule};

#[cfg(windows)]
fn command_for_exec_path(exec_path: &Path) -> Command {
    // On Windows, `.ps1` is not directly executable by `CreateProcess`.
    // We run it via PowerShell to support default action scripts under `share/actions/*.ps1`.
    let exec_str = exec_path.to_string_lossy();
    if exec_str.to_ascii_lowercase().ends_with(".ps1") {
        let mut cmd = Command::new("powershell");
        cmd.arg("-NoProfile")
            .arg("-ExecutionPolicy")
            .arg("Bypass")
            .arg("-File")
            .arg(exec_path);
        return cmd;
    }
    Command::new(exec_path)
}

#[cfg(not(windows))]
fn command_for_exec_path(exec_path: &Path) -> Command {
    Command::new(exec_path)
}

/// Run configured rule actions (WiFi connect, then optional exec script).
///
/// Returns the child process exit code when an exec action ran.
pub fn run_rule_actions<E: SystemExecutor + ?Sized>(
    rule: &Rule,
    vars: &ResolvedVars,
    executor: &E,
) -> RuleResult<Option<i32>> {
    if rule.wifi_connect {
        connect_wifi_from_vars(vars, executor)?;
    }

    let Some(exec_path) = rule.exec.as_ref() else {
        return Ok(None);
    };

    let mut command = command_for_exec_path(exec_path);
    for (key, value) in vars.iter() {
        command.env(key, value);
    }

    let status = command
        .status()
        .map_err(|e| RuleError::ExecFailed(e.to_string()))?;

    Ok(Some(status.code().unwrap_or(-1)))
}

/// Connect using `QR_NATIVE_WIFI_SSID` and optional `QR_NATIVE_WIFI_PASSWORD`.
pub fn connect_wifi_from_vars<E: SystemExecutor + ?Sized>(
    vars: &ResolvedVars,
    executor: &E,
) -> RuleResult<()> {
    let ssid = vars
        .get("QR_NATIVE_WIFI_SSID")
        .ok_or_else(|| RuleError::WifiConnectFailed("missing QR_NATIVE_WIFI_SSID".into()))?;
    let password = vars.get("QR_NATIVE_WIFI_PASSWORD").unwrap_or("");
    executor
        .connect_wifi(ssid, password)
        .map_err(|e| RuleError::WifiConnectFailed(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sys::MockSystemExecutor;
    use mockall::predicate::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn connect_wifi_from_vars_invokes_executor() {
        let mut mock = MockSystemExecutor::new();
        mock.expect_connect_wifi()
            .with(eq("lab"), eq("secret"))
            .times(1)
            .returning(|_, _| Ok(()));

        let mut vars = ResolvedVars::new();
        vars.insert("QR_NATIVE_WIFI_SSID", "lab");
        vars.insert("QR_NATIVE_WIFI_PASSWORD", "secret");

        connect_wifi_from_vars(&vars, &mock).expect("connect");
    }

    #[test]
    fn connect_wifi_from_vars_nopass_uses_empty_password() {
        let mut mock = MockSystemExecutor::new();
        mock.expect_connect_wifi()
            .with(eq("OpenNet"), eq(""))
            .times(1)
            .returning(|_, _| Ok(()));

        let mut vars = ResolvedVars::new();
        vars.insert("QR_NATIVE_WIFI_SSID", "OpenNet");

        connect_wifi_from_vars(&vars, &mock).expect("connect");
    }

    #[test]
    fn connect_wifi_from_vars_missing_ssid_errors() {
        let mock = MockSystemExecutor::new();
        let vars = ResolvedVars::new();
        let err = connect_wifi_from_vars(&vars, &mock).expect_err("missing ssid");
        assert!(matches!(err, RuleError::WifiConnectFailed(_)));
    }

    #[test]
    fn run_rule_actions_wifi_only_skips_exec() {
        let mut mock = MockSystemExecutor::new();
        mock.expect_connect_wifi()
            .with(eq("lab"), eq("pass"))
            .times(1)
            .returning(|_, _| Ok(()));

        let mut rule = Rule::new("wifi");
        rule.wifi_connect = true;

        let mut vars = ResolvedVars::new();
        vars.insert("QR_NATIVE_WIFI_SSID", "lab");
        vars.insert("QR_NATIVE_WIFI_PASSWORD", "pass");

        let code = run_rule_actions(&rule, &vars, &mock).expect("actions");
        assert!(code.is_none());
    }

    #[test]
    fn run_rule_actions_without_wifi_connect_skips_executor() {
        let mock = MockSystemExecutor::new();
        let rule = Rule::new("plain");
        let vars = ResolvedVars::new();
        let code = run_rule_actions(&rule, &vars, &mock).expect("no actions");
        assert!(code.is_none());
    }

    #[cfg(windows)]
    #[test]
    fn run_rule_actions_exec_powershell_script_on_windows() {
        let dir = TempDir::new().expect("tempdir");
        let out_path = dir.path().join("out.txt");
        let script_path = dir.path().join("write-env.ps1");
        let script = format!(
            "$p = \"{}\"; Set-Content -Path $p -Value $env:QR_VAR_TEST -Encoding utf8",
            out_path.display()
        );
        fs::write(&script_path, script).expect("write ps1");

        let mock = MockSystemExecutor::new();
        let mut rule = Rule::new("ps1");
        rule.exec = Some(script_path);

        let mut vars = ResolvedVars::new();
        vars.insert("QR_VAR_TEST", "hello-ps1");

        run_rule_actions(&rule, &vars, &mock).expect("actions");

        let contents = fs::read_to_string(&out_path).expect("read out");
        assert!(contents.contains("hello-ps1"));
    }
}
