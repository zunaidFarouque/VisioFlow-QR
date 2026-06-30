//! Post-route rule actions: WiFi connect and child-process exec.

use std::process::Command;

use crate::sys::SystemExecutor;

use super::error::{Result as RuleResult, RuleError};
use super::model::{ResolvedVars, Rule};

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

    let mut command = Command::new(exec_path);
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
}
