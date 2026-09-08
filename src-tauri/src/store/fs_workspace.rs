use super::fs_collection::list_collections;
use super::format::{read_toml, write_toml};
use crate::domain::{CollectionSummary, WorkspaceFile};
use crate::error::{AppError, AppResult};
use std::fs;
use std::path::{Path, PathBuf};

const LOKKI_DIR: &str = ".lokki";
const WORKSPACE_FILE: &str = "workspace.toml";
const GITIGNORE_FILE: &str = ".gitignore";
const SECRETS_FILE: &str = "secrets.local.toml";

fn lokki_dir(workspace_path: &Path) -> PathBuf {
    workspace_path.join(LOKKI_DIR)
}

fn workspace_file_path(workspace_path: &Path) -> PathBuf {
    lokki_dir(workspace_path).join(WORKSPACE_FILE)
}

/// Opens the workspace rooted at `workspace_path`, initializing it first
/// (writing `.lokki/workspace.toml` + `.gitignore`) if this folder hasn't
/// been used as a LokkiAPI workspace before.
pub fn open_workspace(workspace_path: &Path) -> AppResult<(WorkspaceFile, Vec<CollectionSummary>)> {
    let marker = workspace_file_path(workspace_path);
    let workspace = if marker.is_file() {
        read_toml(&marker)?
    } else {
        let name = workspace_path
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "workspace".to_string());
        init_workspace(workspace_path, &name)?
    };
    let collections = list_collections(workspace_path)?;
    Ok((workspace, collections))
}

fn init_workspace(workspace_path: &Path, name: &str) -> AppResult<WorkspaceFile> {
    fs::create_dir_all(&lokki_dir(workspace_path)).map_err(|source| AppError::Io {
        path: lokki_dir(workspace_path).display().to_string(),
        source,
    })?;
    let workspace = WorkspaceFile::new(name);
    write_toml(&workspace_file_path(workspace_path), &workspace)?;

    let gitignore_path = lokki_dir(workspace_path).join(GITIGNORE_FILE);
    if !gitignore_path.is_file() {
        fs::write(&gitignore_path, format!("{}\n", SECRETS_FILE)).map_err(|source| AppError::Io {
            path: gitignore_path.display().to_string(),
            source,
        })?;
    }
    Ok(workspace)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_workspace_initializes_on_first_open() {
        let dir = tempfile::tempdir().unwrap();
        let (workspace, collections) = open_workspace(dir.path()).unwrap();
        assert!(collections.is_empty());
        assert!(dir.path().join(LOKKI_DIR).join(WORKSPACE_FILE).is_file());
        assert!(dir.path().join(LOKKI_DIR).join(GITIGNORE_FILE).is_file());
        assert_eq!(workspace.schema_version, crate::domain::workspace::CURRENT_SCHEMA_VERSION);
    }

    #[test]
    fn open_workspace_is_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let (first, _) = open_workspace(dir.path()).unwrap();
        let (second, _) = open_workspace(dir.path()).unwrap();
        assert_eq!(first.sync.id, second.sync.id);
    }
}
