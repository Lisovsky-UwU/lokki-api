use super::ids::Id;
use super::sync_meta::SyncMeta;
use serde::{Deserialize, Serialize};

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EnvironmentScope {
    Global,
    Collection,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentMeta {
    #[serde(flatten)]
    pub sync: SyncMeta,
    pub name: String,
    pub scope: EnvironmentScope,
}

/// A variable's `value` is left empty in the on-disk file when
/// `secret == true`; the real value lives in the workspace's local,
/// gitignored secrets store (see secrets::SecretStore), keyed by `id`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Variable {
    pub id: Id,
    pub key: String,
    pub value: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub secret: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentFile {
    pub meta: EnvironmentMeta,
    #[serde(default)]
    pub variables: Vec<Variable>,
}

impl EnvironmentFile {
    pub fn new(name: impl Into<String>, scope: EnvironmentScope) -> Self {
        EnvironmentFile {
            meta: EnvironmentMeta {
                sync: SyncMeta::new(),
                name: name.into(),
                scope,
            },
            variables: Vec::new(),
        }
    }
}
