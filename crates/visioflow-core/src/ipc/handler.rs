//! Daemon-side request handling for IPC messages.
//!
//! [`ClientMessage::ExecuteRule`] always takes an explicit rule `name` (same as
//! `visioflow rule execute <NAME>`). Auto-routing (`route_payload` / omitting
//! `--trigger` on capture) is CLI-local today; clients that need auto mode should
//! run `capture` without `--ipc-socket`, or resolve the rule name client-side and
//! send `execute_rule` with that name.

use std::collections::BTreeMap;

use crate::ipc::{ClientMessage, RequestId, ServerMessage};
use crate::logging::{format_log_line, redact_env_map};
use crate::rules::{
    run_rule_actions, FileRuleStore, ResolvedVars, Rule, RuleEngine, RuleError, RuleResult,
    RuleStore, RoutedPayload,
};
use crate::sys::platform_executor;

/// In-memory rule store loaded from disk for the daemon hot path.
#[derive(Debug, Clone, Default)]
pub struct MemoryRuleStore {
    rules: BTreeMap<String, Rule>,
}

impl MemoryRuleStore {
    #[must_use]
    pub fn from_rules(rules: BTreeMap<String, Rule>) -> Self {
        Self { rules }
    }

    pub fn reload_from_file(&mut self, file_store: &FileRuleStore) -> RuleResult<()> {
        self.rules = file_store.load_all()?;
        Ok(())
    }

    #[must_use]
    pub fn rule_names(&self) -> Vec<String> {
        self.rules.keys().cloned().collect()
    }
}

impl RuleStore for MemoryRuleStore {
    fn load_all(&self) -> RuleResult<BTreeMap<String, Rule>> {
        Ok(self.rules.clone())
    }

    fn save_all(&self, _rules: &BTreeMap<String, Rule>) -> RuleResult<()> {
        Err(RuleError::StoreIo(
            "daemon memory store is read-only".into(),
        ))
    }

    fn get(&self, name: &str) -> RuleResult<Rule> {
        self.rules
            .get(name)
            .cloned()
            .ok_or_else(|| RuleError::NotFound(name.to_owned()))
    }

    fn upsert(&self, _rule: &Rule) -> RuleResult<()> {
        Err(RuleError::StoreIo(
            "daemon memory store is read-only".into(),
        ))
    }

    fn delete(&self, _name: &str) -> RuleResult<()> {
        Err(RuleError::StoreIo(
            "daemon memory store is read-only".into(),
        ))
    }
}

/// Daemon state: in-memory rules backed by a file store for reload.
pub struct DaemonHandler {
    file_store: FileRuleStore,
    memory: MemoryRuleStore,
}

impl DaemonHandler {
    pub fn new(file_store: FileRuleStore) -> RuleResult<Self> {
        let rules = file_store.load_all()?;
        Ok(Self {
            file_store,
            memory: MemoryRuleStore::from_rules(rules),
        })
    }

    pub fn reload(&mut self) -> RuleResult<()> {
        self.memory.reload_from_file(&self.file_store)
    }

    pub fn handle(&mut self, request: ClientMessage, verbose: bool) -> ServerMessage {
        let id = request.id();
        match request {
            ClientMessage::Ping { .. } => ServerMessage::Pong { id },
            ClientMessage::Reload { .. } => {
                if let Err(err) = self.reload() {
                    return ServerMessage::Error {
                        id,
                        message: err.to_string(),
                    };
                }
                if verbose {
                    eprintln!("daemon: rules reloaded");
                }
                ServerMessage::Pong { id }
            }
            ClientMessage::ListRules { .. } => ServerMessage::RulesList {
                id,
                names: self.memory.rule_names(),
            },
            ClientMessage::ExecuteRule { name, payload, .. } => {
                self.execute_rule(id, &name, &payload, verbose)
            }
        }
    }

    fn execute_rule(
        &self,
        id: RequestId,
        name: &str,
        payload: &str,
        verbose: bool,
    ) -> ServerMessage {
        let engine = RuleEngine::new(self.memory.clone());
        match engine.route_fully(name, payload) {
            Ok(RoutedPayload { rule, vars }) => {
                if verbose {
                    let map: std::collections::HashMap<String, String> = vars
                        .iter()
                        .map(|(k, v)| (k.to_owned(), v.to_owned()))
                        .collect();
                    for (key, value) in redact_env_map(&map) {
                        eprintln!("{}", format_log_line(&key, &value));
                    }
                }
                let exit_code = match run_rule_actions(&rule, &vars, &platform_executor()) {
                    Ok(code) => code,
                    Err(err) => return rule_error_response(id, err),
                };
                let var_map = vars
                    .iter()
                    .map(|(k, v)| (k.to_owned(), v.to_owned()))
                    .collect();
                ServerMessage::RuleResult {
                    id,
                    vars: var_map,
                    exit_code,
                }
            }
            Err(err) => rule_error_response(id, err),
        }
    }
}

fn rule_error_response(id: RequestId, err: RuleError) -> ServerMessage {
    match err {
        RuleError::NotFound(missing) => ServerMessage::Error {
            id,
            message: format!("rule not found: {missing}"),
        },
        other => ServerMessage::Error {
            id,
            message: other.to_string(),
        },
    }
}

/// Route a payload through a rule and merge native parser vars (shared with local execute).
pub fn route_with_native(
    engine: &RuleEngine<impl RuleStore>,
    name: &str,
    payload: &str,
) -> RuleResult<ResolvedVars> {
    Ok(engine.route_fully(name, payload)?.vars)
}
