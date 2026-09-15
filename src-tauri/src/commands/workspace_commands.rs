use crate::domain::{CollectionSummary, RecentWorkspace, StartupBehavior, WorkspaceFile};
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

/// Feeds both the recent list on the welcome screen and the workspace the
/// next launch reopens - the head of the list is that workspace.
fn remember_workspace(app: &AppHandle, path: String, name: String) -> AppResult<()> {
    let dir = app_local_data_dir(app)?;
    let mut state = fs_app_state::load(&dir);
    state.remember_workspace(path, name);
    fs_app_state::save(&dir, &state)
}

#[tauri::command]
pub fn open_workspace(app: AppHandle, path: String) -> AppResult<OpenWorkspaceResult> {
    let (workspace, collections) = fs_workspace::open_workspace(Path::new(&path))?;
    remember_workspace(&app, path, workspace.name.clone())?;
    Ok(OpenWorkspaceResult {
        workspace,
        collections,
    })
}

/// Asked right after the folder picker closes, so a folder that cannot hold
/// a new workspace is refused before the user is made to name one.
#[tauri::command]
pub fn check_new_workspace_folder(path: String) -> AppResult<()> {
    fs_workspace::check_new_workspace_folder(Path::new(&path))
}

/// Initializes a folder as a workspace and opens it. Separate from
/// `open_workspace` so picking the wrong folder reports "not a workspace"
/// instead of quietly creating one.
#[tauri::command]
pub fn create_workspace(app: AppHandle, path: String, name: String) -> AppResult<OpenWorkspaceResult> {
    let workspace = fs_workspace::create_workspace(Path::new(&path), &name)?;
    remember_workspace(&app, path, workspace.name.clone())?;
    Ok(OpenWorkspaceResult {
        workspace,
        collections: Vec::new(),
    })
}

/// The recent list carries its own copy of the name, so renaming a workspace
/// has to reach it - the entry would otherwise keep showing the old one until
/// the workspace was next opened. Re-recording it is safe rather than an
/// unwanted insert: only an open workspace can be renamed, and opening one
/// put it on the list.
#[tauri::command]
pub fn rename_workspace(app: AppHandle, path: String, new_name: String) -> AppResult<WorkspaceFile> {
    let workspace = fs_workspace::rename_workspace(Path::new(&path), &new_name)?;
    remember_workspace(&app, path, workspace.name.clone())?;
    Ok(workspace)
}

/// The recent workspaces, newest first. Entries whose folder is gone are
/// hidden rather than dropped: an unplugged drive or a network share that is
/// down would otherwise clear the list it is supposed to shorten the way
/// back to.
#[tauri::command]
pub fn list_recent_workspaces(app: AppHandle) -> AppResult<Vec<RecentWorkspace>> {
    let dir = app_local_data_dir(&app)?;
    Ok(fs_app_state::load(&dir)
        .recent_workspaces
        .into_iter()
        .filter(|entry| Path::new(&entry.path).is_dir())
        .collect())
}

/// Takes the workspace off the list. Nothing on disk is touched - the UI
/// wording has to keep promising that.
#[tauri::command]
pub fn forget_recent_workspace(app: AppHandle, path: String) -> AppResult<Vec<RecentWorkspace>> {
    let dir = app_local_data_dir(&app)?;
    let mut state = fs_app_state::load(&dir);
    state.forget_workspace(&path);
    fs_app_state::save(&dir, &state)?;
    list_recent_workspaces(app)
}

/// The workspace to open on launch, or `None` for the welcome screen. One
/// question with one answer, so the setting and the "does it still exist"
/// check cannot be applied in one place and forgotten in another.
#[tauri::command]
pub fn get_startup_workspace(app: AppHandle) -> AppResult<Option<String>> {
    let dir = app_local_data_dir(&app)?;
    let state = fs_app_state::load(&dir);
    if state.startup == StartupBehavior::Welcome {
        return Ok(None);
    }
    Ok(state
        .recent_workspaces
        .into_iter()
        .map(|entry| entry.path)
        .find(|path| Path::new(path).is_dir()))
}

#[tauri::command]
pub fn list_collections(workspace_path: String) -> AppResult<Vec<CollectionSummary>> {
    fs_collection::list_collections(Path::new(&workspace_path))
}
