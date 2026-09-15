use crate::domain::{Language, RequestSettings, StartupBehavior};
use crate::error::{AppError, AppResult};
use crate::i18n;
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

#[tauri::command]
pub fn get_startup_behavior(app: AppHandle) -> AppResult<StartupBehavior> {
    Ok(fs_app_state::load(&app_local_data_dir(&app)?).startup)
}

/// Read back on the next launch only - `get_startup_workspace` is what asks.
#[tauri::command]
pub fn set_startup_behavior(app: AppHandle, behavior: StartupBehavior) -> AppResult<()> {
    let dir = app_local_data_dir(&app)?;
    let mut state = fs_app_state::load(&dir);
    state.startup = behavior;
    fs_app_state::save(&dir, &state)
}

/// What the user picked in settings, or `None` for "follow the OS". Only the
/// frontend can resolve that second case - the webview is what knows the
/// system display language - so the preference is handed over unresolved.
#[tauri::command]
pub fn get_language_preference(app: AppHandle) -> AppResult<Option<Language>> {
    Ok(fs_app_state::load(&app_local_data_dir(&app)?).language)
}

/// Stores the preference and sets the language the core writes its messages
/// in. The two are separate arguments because they differ whenever the
/// preference is "follow the OS": `effective` is then what the webview
/// resolved that to.
#[tauri::command]
pub fn set_language(app: AppHandle, preference: Option<Language>, effective: Language) -> AppResult<()> {
    let dir = app_local_data_dir(&app)?;
    let mut state = fs_app_state::load(&dir);
    state.language = preference;
    fs_app_state::save(&dir, &state)?;
    i18n::set_current(effective);
    Ok(())
}

/// Applies the stored preference at start-up, before any command can fail in
/// the wrong language. "Follow the OS" is left at the English fallback until
/// the frontend reports what the webview resolved it to - and an unreadable
/// app-local-data dir is not worth failing the launch over, since English is
/// where it would land anyway.
pub fn apply_stored_language(app: &AppHandle) {
    if let Ok(dir) = app_local_data_dir(app) {
        if let Some(language) = fs_app_state::load(&dir).language {
            i18n::set_current(language);
        }
    }
}
