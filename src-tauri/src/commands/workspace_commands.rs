use crate::domain::{CollectionSummary, WorkspaceFile};
use crate::error::AppResult;
use crate::store::{fs_collection, fs_workspace};
use serde::Serialize;
use std::path::Path;

#[derive(Debug, Clone, Serialize)]
pub struct OpenWorkspaceResult {
    pub workspace: WorkspaceFile,
    pub collections: Vec<CollectionSummary>,
}

#[tauri::command]
pub fn open_workspace(path: String) -> AppResult<OpenWorkspaceResult> {
    let (workspace, collections) = fs_workspace::open_workspace(Path::new(&path))?;
    Ok(OpenWorkspaceResult {
        workspace,
        collections,
    })
}

#[tauri::command]
pub fn list_collections(workspace_path: String) -> AppResult<Vec<CollectionSummary>> {
    fs_collection::list_collections(Path::new(&workspace_path))
}
