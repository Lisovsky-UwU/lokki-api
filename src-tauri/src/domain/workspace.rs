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

/// An entry in the "recent workspaces" list on the welcome screen. The name
/// is stored alongside the path rather than read from the folder each time:
/// the list has to render before - and even when - the workspaces on it can
/// be opened, and a folder on an unplugged drive would otherwise be nameless.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecentWorkspace {
    pub path: String,
    pub name: String,
}
