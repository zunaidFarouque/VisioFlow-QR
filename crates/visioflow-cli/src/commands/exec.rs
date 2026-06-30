use std::process::Command;

use visioflow_core::{ResolvedVars, Rule, RuleError, RuleResult};

/// Spawn the rule's exec action with resolved variables in the child environment.
pub fn spawn_rule_exec(rule: &Rule, vars: &ResolvedVars) -> RuleResult<()> {
    let Some(exec) = rule.exec.as_ref() else {
        return Ok(());
    };

    let mut cmd = Command::new(exec);
    for (key, value) in vars.iter() {
        cmd.env(key, value);
    }

    cmd.status()
        .map_err(|e| RuleError::ExecFailed(e.to_string()))?;
    Ok(())
}
