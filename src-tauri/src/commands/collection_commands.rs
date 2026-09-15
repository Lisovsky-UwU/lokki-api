use crate::domain::CollectionSummary;
use crate::error::{AppError, AppResult};
use crate::store::fs_collection::{self, CollectionTreeNode};
use crate::store::fs_app_state;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};

#[tauri::command]
pub fn create_collection(workspace_path: String, name: String) -> AppResult<CollectionSummary> {
    fs_collection::create_collection(Path::new(&workspace_path), &name)
}

#[tauri::command]
pub fn load_collection_tree(collection_path: String) -> AppResult<CollectionTreeNode> {
    fs_collection::load_collection_tree(Path::new(&collection_path))
}

/// Writes the order the sidebar was dragged into onto the collections
/// themselves, so it travels with the workspace like every other ordering.
#[tauri::command]
pub fn reorder_collections(ordered_paths: Vec<String>) -> AppResult<()> {
    let paths: Vec<PathBuf> = ordered_paths.into_iter().map(PathBuf::from).collect();
    fs_collection::reorder_collections(&paths)
}

fn app_local_data_dir(app: &AppHandle) -> AppResult<PathBuf> {
    app.path()
        .app_local_data_dir()
        .map_err(|_| AppError::NotFound("app local data dir".to_string()))
}

/// Renames a collection and carries its per-device settings over to the new
/// path, so the collection keeps the environment it had selected.
#[tauri::command]
pub fn rename_collection(app: AppHandle, collection_path: String, new_name: String) -> AppResult<CollectionSummary> {
    let summary = fs_collection::rename_collection(Path::new(&collection_path), &new_name)?;
    if summary.path != collection_path {
        let dir = app_local_data_dir(&app)?;
        let mut state = fs_app_state::load(&dir);
        state.rebase_root(&collection_path, &summary.path);
        fs_app_state::save(&dir, &state)?;
    }
    Ok(summary)
}

/// Deletes a collection and forgets the per-device settings that pointed at
/// it.
#[tauri::command]
pub fn delete_collection(app: AppHandle, collection_path: String) -> AppResult<()> {
    fs_collection::delete_collection(Path::new(&collection_path))?;
    let dir = app_local_data_dir(&app)?;
    let mut state = fs_app_state::load(&dir);
    state.forget_root(&collection_path);
    fs_app_state::save(&dir, &state)
}
