use super::SecretStore;
use crate::domain::Id;
use crate::error::AppResult;
use crate::store::format::write_toml;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

const SECRETS_FILE: &str = "secrets.local.toml";

#[derive(Debug, Default, Serialize, Deserialize)]
struct SecretsFile {
    #[serde(default)]
    values: HashMap<String, String>,
}

fn secrets_path(workspace_path: &Path) -> PathBuf {
    workspace_path.join(".lokki").join(SECRETS_FILE)
}

/// Missing/corrupt secrets file is treated as "no secrets set yet" rather
/// than an error - the file is created lazily on first `set`.
fn load(workspace_path: &Path) -> SecretsFile {
    fs::read_to_string(secrets_path(workspace_path))
        .ok()
        .and_then(|raw| toml::from_str(&raw).ok())
        .unwrap_or_default()
}

/// Local, gitignored implementation of `SecretStore` (the `.lokki/.gitignore`
/// written by store::fs_workspace already excludes this file).
pub struct LocalFileSecretStore;

impl SecretStore for LocalFileSecretStore {
    fn get(&self, workspace_path: &Path, variable_id: &Id) -> AppResult<Option<String>> {
        Ok(load(workspace_path).values.get(variable_id.as_ref()).cloned())
    }

    fn set(&self, workspace_path: &Path, variable_id: &Id, value: &str) -> AppResult<()> {
        let mut file = load(workspace_path);
        file.values.insert(variable_id.as_ref().to_string(), value.to_string());
        write_toml(&secrets_path(workspace_path), &file)
    }

    fn remove_many(&self, workspace_path: &Path, variable_ids: &[Id]) -> AppResult<()> {
        let mut file = load(workspace_path);
        let before = file.values.len();
        file.values.retain(|key, _| !variable_ids.iter().any(|id| id.as_ref() == key));
        // Nothing matched: don't create a secrets file just to write an empty
        // one for a workspace that never had any.
        if file.values.len() == before {
            return Ok(());
        }
        write_toml(&secrets_path(workspace_path), &file)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::Id;

    #[test]
    fn set_then_get_round_trips_and_missing_key_is_none() {
        let dir = tempfile::tempdir().unwrap();
        let store = LocalFileSecretStore;
        let id = Id::new();
        assert_eq!(store.get(dir.path(), &id).unwrap(), None);

        store.set(dir.path(), &id, "s3cr3t").unwrap();
        assert_eq!(store.get(dir.path(), &id).unwrap(), Some("s3cr3t".to_string()));
    }

    #[test]
    fn remove_many_drops_only_the_listed_ids() {
        let dir = tempfile::tempdir().unwrap();
        let store = LocalFileSecretStore;
        let doomed = Id::new();
        let kept = Id::new();
        store.set(dir.path(), &doomed, "token").unwrap();
        store.set(dir.path(), &kept, "keep").unwrap();

        store.remove_many(dir.path(), &[doomed.clone()]).unwrap();

        assert_eq!(store.get(dir.path(), &doomed).unwrap(), None);
        assert_eq!(store.get(dir.path(), &kept).unwrap(), Some("keep".to_string()));
    }

    #[test]
    fn remove_many_on_a_workspace_without_secrets_writes_nothing() {
        let dir = tempfile::tempdir().unwrap();
        LocalFileSecretStore.remove_many(dir.path(), &[Id::new()]).unwrap();
        assert!(!dir.path().join(".lokki").join(SECRETS_FILE).exists());
    }
}
