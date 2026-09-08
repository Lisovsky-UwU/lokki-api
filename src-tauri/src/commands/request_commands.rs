use crate::domain::{HttpMethod, RequestFile};
use crate::error::AppResult;
use crate::store::fs_request;
use std::path::Path;

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
pub fn delete_request(request_path: String) -> AppResult<()> {
    fs_request::delete_request(Path::new(&request_path))
}

#[tauri::command]
pub fn create_folder(parent_path: String, name: String) -> AppResult<String> {
    fs_request::create_folder(Path::new(&parent_path), &name).map(|p| p.display().to_string())
}

#[tauri::command]
pub fn delete_folder(folder_path: String) -> AppResult<()> {
    fs_request::delete_folder(Path::new(&folder_path))
}
