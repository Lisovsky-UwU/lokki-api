use super::sync_meta::SyncMeta;
use serde::{Deserialize, Serialize};

/// Root marker file for a workspace folder (`.lokki/workspace.toml`).
/// Kept intentionally thin: the folder tree on disk *is* the collection
/// structure, no separate index is maintained.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceFile {
    #[serde(flatten)]
    pub sync: SyncMeta,
    pub name: String,
    pub schema_version: u32,
}

pub const CURRENT_SCHEMA_VERSION: u32 = 1;

impl WorkspaceFile {
    pub fn new(name: impl Into<String>) -> Self {
        WorkspaceFile {
            sync: SyncMeta::new(),
            name: name.into(),
            schema_version: CURRENT_SCHEMA_VERSION,
        }
    }
}
