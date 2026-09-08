use crate::domain::CollectionSummary;
use crate::error::AppResult;
use crate::store::fs_collection::{self, CollectionTreeNode};
use std::path::Path;

#[tauri::command]
pub fn create_collection(workspace_path: String, name: String) -> AppResult<CollectionSummary> {
    fs_collection::create_collection(Path::new(&workspace_path), &name)
}

#[tauri::command]
pub fn load_collection_tree(collection_path: String) -> AppResult<CollectionTreeNode> {
    fs_collection::load_collection_tree(Path::new(&collection_path))
}
