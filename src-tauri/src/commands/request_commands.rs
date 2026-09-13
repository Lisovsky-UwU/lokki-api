use crate::domain::{HttpMethod, RequestFile};
use crate::error::AppResult;
use crate::store::fs_request;
use serde::Serialize;
use std::path::{Path, PathBuf};

/// A request plus the path it now lives at — renaming moves the file, so the
/// frontend needs the new path to keep pointing at the right thing.
#[derive(Debug, Clone, Serialize)]
pub struct RequestAtPath {
    pub path: String,
    #[serde(flatten)]
    pub request: RequestFile,
}

#[tauri::command]
pub fn load_request(request_path: String) -> AppResult<RequestFile> {
    fs_request::load_request(Path::new(&request_path))
}

#[tauri::command]
pub fn save_request(request_path: String, request: RequestFile) -> AppResult<RequestFile> {
    fs_request::save_request(Path::new(&request_path), request)
}

#[tauri::command]
pub fn create_request(parent_path: String, name: String, method: HttpMethod) -> AppResult<RequestFile> {
    fs_request::create_request(Path::new(&parent_path), &name, method)
}

#[tauri::command]
pub fn clone_request(request_path: String, new_name: String) -> AppResult<RequestAtPath> {
    let (path, request) = fs_request::clone_request(Path::new(&request_path), &new_name)?;
    Ok(RequestAtPath {
        path: path.display().to_string(),
        request,
    })
}

/// Saves an incognito request into a workspace folder, where it becomes an
/// ordinary request.
#[tauri::command]
pub fn adopt_request(parent_path: String, name: String, request: RequestFile) -> AppResult<RequestAtPath> {
    let (path, request) = fs_request::adopt_request(Path::new(&parent_path), &name, request)?;
    Ok(RequestAtPath {
        path: path.display().to_string(),
        request,
    })
}

/// Saves an incognito request to a file the user picked, outside any
/// workspace.
#[tauri::command]
pub fn export_request(file_path: String, name: String, request: RequestFile) -> AppResult<RequestFile> {
    fs_request::export_request(Path::new(&file_path), request, &name)
}

#[tauri::command]
pub fn delete_request(request_path: String) -> AppResult<()> {
    fs_request::delete_request(Path::new(&request_path))
}

#[tauri::command]
pub fn rename_request(request_path: String, new_name: String) -> AppResult<RequestAtPath> {
    let (path, request) = fs_request::rename_request(Path::new(&request_path), &new_name)?;
    Ok(RequestAtPath {
        path: path.display().to_string(),
        request,
    })
}

#[tauri::command]
pub fn create_folder(parent_path: String, name: String) -> AppResult<String> {
    fs_request::create_folder(Path::new(&parent_path), &name).map(|p| p.display().to_string())
}

#[tauri::command]
pub fn delete_folder(folder_path: String) -> AppResult<()> {
    fs_request::delete_folder(Path::new(&folder_path))
}

#[tauri::command]
pub fn rename_folder(folder_path: String, new_name: String) -> AppResult<String> {
    fs_request::rename_folder(Path::new(&folder_path), &new_name).map(|p| p.display().to_string())
}

/// Moves a request or folder into another folder (drag-and-drop between
/// folders). Returns the moved entry's new path.
#[tauri::command]
pub fn move_node(source_path: String, target_parent: String) -> AppResult<String> {
    fs_request::move_node(Path::new(&source_path), Path::new(&target_parent))
        .map(|p| p.display().to_string())
}

/// Applies an explicit sibling order after a drag-and-drop reorder.
#[tauri::command]
pub fn reorder_children(ordered_paths: Vec<String>) -> AppResult<()> {
    let paths: Vec<PathBuf> = ordered_paths.into_iter().map(PathBuf::from).collect();
    fs_request::reorder_children(&paths)
}
