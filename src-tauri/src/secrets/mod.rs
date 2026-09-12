pub mod local_file;

use crate::domain::Id;
use crate::error::AppResult;
use std::path::Path;

/// Values for variables marked `secret = true` never live in the
/// syncable workspace tree (see domain::environment docs) — they go
/// through this trait instead. Kept as a trait so a follow-up OS-keychain
/// implementation (via the `keyring` crate) can replace the local-file one
/// without touching call sites in commands/.
pub trait SecretStore: Send + Sync {
    fn get(&self, workspace_path: &Path, variable_id: &Id) -> AppResult<Option<String>>;
    fn set(&self, workspace_path: &Path, variable_id: &Id, value: &str) -> AppResult<()>;
    /// Drops the values for `variable_ids`. Deleting an environment has to
    /// call this: nothing else references those ids afterwards, so the
    /// secrets would otherwise stay on disk forever with no way to reach
    /// them from the UI.
    fn remove_many(&self, workspace_path: &Path, variable_ids: &[Id]) -> AppResult<()>;
}
