mod actions;
mod engine;
mod error;
mod model;
mod store;

#[cfg(test)]
mod engine_test;
#[cfg(test)]
mod store_test;

pub use actions::{connect_wifi_from_vars, run_rule_actions};
pub use engine::{
    apply_rule, merge_native_vars, resolve_payload_fully, PayloadRouter, RoutedPayload,
    RuleEngine,
};

pub use error::{Result as RuleResult, RuleError};
pub use model::{ResolvedVars, Rule};
pub use store::{FileRuleStore, RuleStore};
