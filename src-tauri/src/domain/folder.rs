use super::sync_meta::SyncMeta;
use serde::{Deserialize, Serialize};

/// Metadata file inside a folder (`folder.toml`).
///
/// Folders originally carried no metadata at all — the filesystem tree *was*
/// the structure. This file exists so a folder can hold a stable id (for the
/// planned sync) and an explicit position among its siblings, which
/// drag-and-drop ordering needs. Folders created before it existed simply
/// have no file; they are read with a derived name and `seq = 0`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FolderFile {
    #[serde(flatten)]
    pub sync: SyncMeta,
    pub name: String,
    #[serde(default)]
    pub seq: u32,
}

impl FolderFile {
    pub fn new(name: impl Into<String>, seq: u32) -> Self {
        FolderFile {
            sync: SyncMeta::new(),
            name: name.into(),
            seq,
        }
    }
}
