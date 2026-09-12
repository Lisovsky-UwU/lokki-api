use crate::domain::{CollectionSummary, WorkspaceFile};
use crate::error::{AppError, AppResult};
use crate::store::{fs_app_state, fs_collection, fs_workspace};
use serde::Serialize;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};

#[derive(Debug, Clone, Serialize)]
pub struct OpenWorkspaceResult {
    pub workspace: WorkspaceFile,
    pub collections: Vec<CollectionSummary>,
}

fn app_local_data_dir(app: &AppHandle) -> AppResult<PathBuf> {
    app.path()
        .app_local_data_dir()
        .map_err(|_| AppError::NotFound("app local data dir".to_string()))
}

/// Remembered so the next launch can skip the picker and reopen straight
/// into the workspace the user was last working in.
fn remember_last_workspace(app: &AppHandle, path: String) -> AppResult<()> {
    let dir = app_local_data_dir(app)?;
    let mut state = fs_app_state::load(&dir);
    state.last_workspace = Some(path);
    fs_app_state::save(&dir, &state)
}

#[tauri::command]
pub fn open_workspace(app: AppHandle, path: String) -> AppResult<OpenWorkspaceResult> {
    let (workspace, collections) = fs_workspace::open_workspace(Path::new(&path))?;
    remember_last_workspace(&app, path)?;
    Ok(OpenWorkspaceResult {
        workspace,
        collections,
    })
}

/// Initializes a folder as a workspace and opens it. Separate from
/// `open_workspace` so picking the wrong folder reports "not a workspace"
/// instead of quietly creating one.
#[tauri::command]
pub fn create_workspace(app: AppHandle, path: String, name: String) -> AppResult<OpenWorkspaceResult> {
    let workspace = fs_workspace::create_workspace(Path::new(&path), &name)?;
    remember_last_workspace(&app, path)?;
    Ok(OpenWorkspaceResult {
        workspace,
        collections: Vec::new(),
    })
}

#[tauri::command]
pub fn rename_workspace(path: String, new_name: String) -> AppResult<WorkspaceFile> {
    fs_workspace::rename_workspace(Path::new(&path), &new_name)
}

/// Path of the workspace opened last, if it still exists on disk.
#[tauri::command]
pub fn get_last_workspace(app: AppHandle) -> AppResult<Option<String>> {
    let dir = app_local_data_dir(&app)?;
    let last = fs_app_state::load(&dir)
        .last_workspace
        .filter(|path| Path::new(path).is_dir());
    Ok(last)
}

#[tauri::command]
pub fn list_collections(workspace_path: String) -> AppResult<Vec<CollectionSummary>> {
    fs_collection::list_collections(Path::new(&workspace_path))
}
