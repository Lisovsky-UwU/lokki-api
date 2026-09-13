use crate::domain::{EnvironmentFile, EnvironmentScope, Id};
use crate::error::{AppError, AppResult};
use crate::secrets::{local_file::LocalFileSecretStore, SecretStore};
use crate::store::{fs_app_state, fs_environment};
use serde::Serialize;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};

/// An environment paired with its on-disk path - the frontend needs the
/// path to later call `save_environment`, but the path itself isn't part
/// of the persisted entity (see domain::environment).
#[derive(Debug, Clone, Serialize)]
pub struct EnvironmentEntry {
    pub path: String,
    #[serde(flatten)]
    pub file: EnvironmentFile,
}

/// `root_path` is a workspace root (global scope) or a collection root
/// (collection scope); environments live in its `environments/` subfolder.
fn environments_dir(root_path: &str) -> PathBuf {
    Path::new(root_path).join("environments")
}

fn app_local_data_dir(app: &AppHandle) -> AppResult<PathBuf> {
    app.path()
        .app_local_data_dir()
        .map_err(|_| AppError::NotFound("app local data dir".to_string()))
}

#[tauri::command]
pub fn list_environments(root_path: String) -> AppResult<Vec<EnvironmentEntry>> {
    let entries = fs_environment::list_environments(&environments_dir(&root_path))?;
    Ok(entries
        .into_iter()
        .map(|(path, file)| EnvironmentEntry {
            path: path.display().to_string(),
            file,
        })
        .collect())
}

#[tauri::command]
pub fn create_environment(
    root_path: String,
    name: String,
    scope: EnvironmentScope,
) -> AppResult<EnvironmentEntry> {
    let (path, file) = fs_environment::create_environment(&environments_dir(&root_path), &name, scope)?;
    Ok(EnvironmentEntry {
        path: path.display().to_string(),
        file,
    })
}

/// Saving can move the file (renaming an environment renames it), so the
/// entry comes back with the path the caller should use from now on.
#[tauri::command]
pub fn save_environment(env_path: String, environment: EnvironmentFile) -> AppResult<EnvironmentEntry> {
    let (path, file) = fs_environment::save_environment(Path::new(&env_path), environment)?;
    Ok(EnvironmentEntry {
        path: path.display().to_string(),
        file,
    })
}

/// Deletes an environment along with everything that only it referenced:
/// the secret values of its variables (nothing else can reach them
/// afterwards) and the "active environment" setting of any root that
/// pointed at it.
#[tauri::command]
pub fn delete_environment(app: AppHandle, workspace_path: String, env_path: String) -> AppResult<()> {
    let path = Path::new(&env_path);
    // Read before deleting: the file is the only place these ids live. An
    // unreadable file is still deleted - leaving a corrupt environment
    // undeletable would be worse than leaking its secret values.
    let file = crate::store::format::read_toml::<EnvironmentFile>(path).ok();
    let secret_ids: Vec<Id> = file
        .as_ref()
        .map(|f| f.variables.iter().filter(|v| v.secret).map(|v| v.id.clone()).collect())
        .unwrap_or_default();

    fs_environment::delete_environment(path)?;

    if !secret_ids.is_empty() {
        LocalFileSecretStore.remove_many(Path::new(&workspace_path), &secret_ids)?;
    }
    if let Some(id) = file.map(|f| f.meta.sync.id) {
        let dir = app_local_data_dir(&app)?;
        let mut state = fs_app_state::load(&dir);
        let before = state.active_environments.len();
        state.active_environments.retain(|_, active| active != &id);
        if state.active_environments.len() != before {
            fs_app_state::save(&dir, &state)?;
        }
    }
    Ok(())
}

#[tauri::command]
pub fn set_active_environment(
    app: AppHandle,
    root_path: String,
    environment_id: Option<Id>,
) -> AppResult<()> {
    let dir = app_local_data_dir(&app)?;
    let mut state = fs_app_state::load(&dir);
    match environment_id {
        Some(id) => {
            state.active_environments.insert(root_path, id);
        }
        None => {
            state.active_environments.remove(&root_path);
        }
    }
    fs_app_state::save(&dir, &state)
}

#[tauri::command]
pub fn get_active_environment(app: AppHandle, root_path: String) -> AppResult<Option<EnvironmentEntry>> {
    let dir = app_local_data_dir(&app)?;
    let state = fs_app_state::load(&dir);
    let Some(active_id) = state.active_environments.get(&root_path) else {
        return Ok(None);
    };
    let entries = fs_environment::list_environments(&environments_dir(&root_path))?;
    Ok(entries
        .into_iter()
        .find(|(_, file)| &file.meta.sync.id == active_id)
        .map(|(path, file)| EnvironmentEntry {
            path: path.display().to_string(),
            file,
        }))
}
