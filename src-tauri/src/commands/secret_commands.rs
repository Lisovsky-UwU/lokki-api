use crate::domain::Id;
use crate::error::AppResult;
use crate::secrets::{local_file::LocalFileSecretStore, SecretStore};
use std::path::Path;

#[tauri::command]
pub fn set_secret(workspace_path: String, variable_id: Id, value: String) -> AppResult<()> {
    LocalFileSecretStore.set(Path::new(&workspace_path), &variable_id, &value)
}

#[tauri::command]
pub fn reveal_secret(workspace_path: String, variable_id: Id) -> AppResult<Option<String>> {
    LocalFileSecretStore.get(Path::new(&workspace_path), &variable_id)
}
