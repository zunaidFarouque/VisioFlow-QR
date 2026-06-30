use visioflow_core::{platform_executor, run_rule_actions, ResolvedVars, Rule, RuleResult};

/// Spawn configured rule actions (WiFi connect, exec script) with resolved variables.
pub fn spawn_rule_actions(rule: &Rule, vars: &ResolvedVars) -> RuleResult<()> {
    run_rule_actions(rule, vars, &platform_executor())?;
    Ok(())
}
