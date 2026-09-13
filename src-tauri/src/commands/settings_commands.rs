use crate::domain::RequestSettings;
use crate::error::{AppError, AppResult};
use crate::store::fs_app_state;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

fn app_local_data_dir(app: &AppHandle) -> AppResult<PathBuf> {
    app.path()
        .app_local_data_dir()
        .map_err(|_| AppError::NotFound("app local data dir".to_string()))
}

#[tauri::command]
pub fn get_request_settings(app: AppHandle) -> AppResult<RequestSettings> {
    Ok(fs_app_state::load(&app_local_data_dir(&app)?).request_settings)
}

/// Takes effect on the next send: the executor keys its cached HTTP client
/// by these settings and rebuilds it when they change.
#[tauri::command]
pub fn save_request_settings(app: AppHandle, settings: RequestSettings) -> AppResult<RequestSettings> {
    let dir = app_local_data_dir(&app)?;
    let mut state = fs_app_state::load(&dir);
    state.request_settings = settings;
    fs_app_state::save(&dir, &state)?;
    Ok(state.request_settings)
}
