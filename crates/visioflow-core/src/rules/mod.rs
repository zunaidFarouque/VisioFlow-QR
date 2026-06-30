mod actions;
pub mod auto;
mod builtins;
mod engine;
mod error;
mod model;
mod notify;
mod store;

#[cfg(test)]
mod auto_test;
#[cfg(test)]
mod builtins_test;
#[cfg(test)]
mod engine_test;
#[cfg(test)]
mod notify_test;
#[cfg(test)]
mod store_test;

pub use actions::{connect_wifi_from_vars, run_rule_actions};
pub use auto::{route_payload, AutoRouteOptions, RouteMode, RouteResult, RoutingEvent};
pub use builtins::{
    is_builtin_trigger, is_excluded_from_auto_scan, is_reserved_rule_name, RESERVED_RULE_NAMES,
};
pub use engine::{
    apply_rule, merge_native_vars, resolve_payload_fully, PayloadRouter, RoutedPayload, RuleEngine,
};
pub use error::{Result as RuleResult, RuleError};
pub use model::{ResolvedVars, Rule};
pub use notify::{format_routing_json_line, format_routing_message};
pub use store::{FileRuleStore, RuleStore};
