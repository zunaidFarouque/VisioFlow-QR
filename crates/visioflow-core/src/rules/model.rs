use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// User-defined routing rule: optional regex, capture mappings, and exec action.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rule {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub regex: Option<String>,
    /// Capture group name → env var suffix (e.g. `"asset"` → `"ASSET"` → `QR_VAR_ASSET`).
    #[serde(default)]
    pub captures: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exec: Option<PathBuf>,
    /// When true, connect to WiFi using `QR_NATIVE_WIFI_*` vars after routing.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub wifi_connect: bool,
}

impl Rule {
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            regex: None,
            captures: BTreeMap::new(),
            exec: None,
            wifi_connect: false,
        }
    }
}

/// Resolved environment variables for a routed payload.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ResolvedVars {
    vars: BTreeMap<String, String>,
}

impl ResolvedVars {
    pub const QR_RAW: &'static str = "QR_RAW";
    pub const QR_VAR_PREFIX: &'static str = "QR_VAR_";

    #[must_use]
    pub fn new() -> Self {
        Self {
            vars: BTreeMap::new(),
        }
    }

    pub fn insert(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.vars.insert(key.into(), value.into());
    }

    #[must_use]
    pub fn get(&self, key: &str) -> Option<&str> {
        self.vars.get(key).map(String::as_str)
    }

    #[must_use]
    pub fn raw(&self) -> Option<&str> {
        self.get(Self::QR_RAW)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.vars.iter().map(|(k, v)| (k.as_str(), v.as_str()))
    }

    #[must_use]
    pub fn into_inner(self) -> BTreeMap<String, String> {
        self.vars
    }
}
