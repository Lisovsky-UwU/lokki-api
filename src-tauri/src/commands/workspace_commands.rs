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

#[tauri::command]
pub fn open_workspace(app: AppHandle, path: String) -> AppResult<OpenWorkspaceResult> {
    let (workspace, collections) = fs_workspace::open_workspace(Path::new(&path))?;

    // Remembered so the next launch can skip the picker and reopen straight
    // into the workspace the user was last working in.
    let dir = app_local_data_dir(&app)?;
    let mut state = fs_app_state::load(&dir);
    state.last_workspace = Some(path);
    fs_app_state::save(&dir, &state)?;

    Ok(OpenWorkspaceResult {
        workspace,
        collections,
    })
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
