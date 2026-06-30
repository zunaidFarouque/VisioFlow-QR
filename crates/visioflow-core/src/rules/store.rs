use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::rules::error::{Result, RuleError};
use crate::rules::model::Rule;

/// Persistence layer for user-defined rules.
#[cfg_attr(test, mockall::automock)]
pub trait RuleStore: Send + Sync {
    fn load_all(&self) -> Result<BTreeMap<String, Rule>>;
    fn save_all(&self, rules: &BTreeMap<String, Rule>) -> Result<()>;
    fn get(&self, name: &str) -> Result<Rule>;
    fn upsert(&self, rule: &Rule) -> Result<()>;
    fn delete(&self, name: &str) -> Result<()>;
}

/// JSON-backed rule store at a caller-supplied path.
#[derive(Debug, Clone)]
pub struct FileRuleStore {
    path: PathBuf,
}

impl FileRuleStore {
    #[must_use]
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn default_path() -> Result<PathBuf> {
        let config_dir = dirs::config_dir().ok_or(RuleError::ConfigDirUnavailable)?;
        Ok(config_dir.join("visioflow").join("rules.json"))
    }

    #[must_use]
    pub fn with_default_path() -> Self {
        Self::new(Self::default_path().unwrap_or_else(|_| PathBuf::from("rules.json")))
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    fn read_rules(&self) -> Result<BTreeMap<String, Rule>> {
        if !self.path.exists() {
            return Ok(BTreeMap::new());
        }
        let contents =
            fs::read_to_string(&self.path).map_err(|e| RuleError::StoreIo(e.to_string()))?;
        if contents.trim().is_empty() {
            return Ok(BTreeMap::new());
        }
        serde_json::from_str(&contents).map_err(|e| RuleError::StoreParse(e.to_string()))
    }

    fn write_rules(&self, rules: &BTreeMap<String, Rule>) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(|e| RuleError::StoreIo(e.to_string()))?;
        }
        let contents = serde_json::to_string_pretty(rules)
            .map_err(|e| RuleError::StoreParse(e.to_string()))?;
        fs::write(&self.path, contents).map_err(|e| RuleError::StoreIo(e.to_string()))
    }
}

impl RuleStore for FileRuleStore {
    fn load_all(&self) -> Result<BTreeMap<String, Rule>> {
        self.read_rules()
    }

    fn save_all(&self, rules: &BTreeMap<String, Rule>) -> Result<()> {
        self.write_rules(rules)
    }

    fn get(&self, name: &str) -> Result<Rule> {
        let rules = self.read_rules()?;
        rules
            .get(name)
            .cloned()
            .ok_or_else(|| RuleError::NotFound(name.to_owned()))
    }

    fn upsert(&self, rule: &Rule) -> Result<()> {
        let mut rules = self.read_rules()?;
        rules.insert(rule.name.clone(), rule.clone());
        self.write_rules(&rules)
    }

    fn delete(&self, name: &str) -> Result<()> {
        let mut rules = self.read_rules()?;
        if rules.remove(name).is_none() {
            return Err(RuleError::NotFound(name.to_owned()));
        }
        self.write_rules(&rules)
    }
}
