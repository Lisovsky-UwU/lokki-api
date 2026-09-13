use super::ids::Id;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Common metadata every mutable entity carries. This is the entire cost
/// paid today toward a future sync engine: a stable id independent of file
/// path/name, plus enough version/timestamp info for later conflict
/// resolution. No outbox/changelog is implemented yet - see the "sync
/// seam" note in store::fs_request for where that would attach.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncMeta {
    pub id: Id,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub version: u32,
}

impl SyncMeta {
    pub fn new() -> Self {
        let now = Utc::now();
        SyncMeta {
            id: Id::new(),
            created_at: now,
            updated_at: now,
            version: 1,
        }
    }

    pub fn touch(&mut self) {
        self.updated_at = Utc::now();
        self.version += 1;
    }
}

impl Default for SyncMeta {
    fn default() -> Self {
        Self::new()
    }
}
