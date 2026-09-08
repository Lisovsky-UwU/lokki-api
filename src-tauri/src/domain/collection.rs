use super::sync_meta::SyncMeta;
use serde::{Deserialize, Serialize};

/// A collection = a directory directly under the workspace root, marked by
/// a `collection.toml` file. Folders inside it carry no metadata file of
/// their own in MVP (the filesystem tree is the folder tree); a
/// `folder.toml` can be introduced later for ordering/rename-tracking
/// without breaking this format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionFile {
    #[serde(flatten)]
    pub sync: SyncMeta,
    pub name: String,
}

impl CollectionFile {
    pub fn new(name: impl Into<String>) -> Self {
        CollectionFile {
            sync: SyncMeta::new(),
            name: name.into(),
        }
    }
}

/// Lightweight summary used when listing collections in a workspace,
/// without loading each collection's full request tree.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionSummary {
    pub name: String,
    pub path: String,
}
