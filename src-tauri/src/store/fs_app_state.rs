use crate::domain::{Id, Language, RequestSettings};
use crate::error::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

const STATE_FILE: &str = "app_state.json";

/// Local, per-device UI state (active environment per workspace/collection
/// root, last-opened workspace). Deliberately stored outside the workspace
/// folder - it is not shareable/syncable data, see domain::sync_meta docs.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppState {
    #[serde(default)]
    pub active_environments: HashMap<String, Id>,
    #[serde(default)]
    pub last_workspace: Option<String>,
    #[serde(default)]
    pub request_settings: RequestSettings,
    /// The UI language the user picked, or `None` for "follow the OS" -
    /// which is the default, and what an app that has never been to the
    /// settings dialog stays on.
    #[serde(default)]
    pub language: Option<Language>,
}

impl AppState {
    /// Forgets per-root settings for a root and anything under it. Used when
    /// a collection is deleted: its entry would otherwise linger in
    /// app_state.json pointing at a path that no longer exists.
    pub fn forget_root(&mut self, root: &str) {
        self.active_environments
            .retain(|path, _| path != root && !path.starts_with(&format!("{root}\\")) && !path.starts_with(&format!("{root}/")));
    }

    /// Roots are keyed by path, so a renamed collection would otherwise
    /// silently lose its active environment.
    pub fn rebase_root(&mut self, old_root: &str, new_root: &str) {
        if let Some(id) = self.active_environments.remove(old_root) {
            self.active_environments.insert(new_root.to_string(), id);
        }
    }
}

fn state_path(app_local_data_dir: &Path) -> PathBuf {
    app_local_data_dir.join(STATE_FILE)
}

/// Corrupt or unreadable state is non-fatal: this is disposable local UI
/// state, so we just fall back to defaults rather than surface an error.
pub fn load(app_local_data_dir: &Path) -> AppState {
    fs::read_to_string(state_path(app_local_data_dir))
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

pub fn save(app_local_data_dir: &Path, state: &AppState) -> AppResult<()> {
    let path = state_path(app_local_data_dir);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| AppError::Io {
            path: parent.display().to_string(),
            source,
        })?;
    }
    let raw = serde_json::to_string_pretty(state).expect("AppState always serializes");
    fs::write(&path, raw).map_err(|source| AppError::Io {
        path: path.display().to_string(),
        source,
    })
}
