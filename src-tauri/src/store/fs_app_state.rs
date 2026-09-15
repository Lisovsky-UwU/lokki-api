use crate::domain::{Id, Language, RecentWorkspace, RequestSettings, StartupBehavior};
use crate::error::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

const STATE_FILE: &str = "app_state.json";

/// How many workspaces the welcome screen offers. Long enough to cover the
/// handful of projects anyone switches between, short enough that the list
/// stays a shortcut rather than a history to read through.
pub const RECENT_WORKSPACES_LIMIT: usize = 10;

/// Local, per-device UI state (active environment per workspace/collection
/// root, recently opened workspaces). Deliberately stored outside the workspace
/// folder - it is not shareable/syncable data, see domain::sync_meta docs.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppState {
    #[serde(default)]
    pub active_environments: HashMap<String, Id>,
    /// Most recently opened first, capped at `RECENT_WORKSPACES_LIMIT`. The
    /// head of this list *is* the last-opened workspace - there is no second
    /// field for it, or the two could drift apart.
    #[serde(default)]
    pub recent_workspaces: Vec<RecentWorkspace>,
    /// What `recent_workspaces` replaced. Read once so an app that has been
    /// updated still reopens the folder it was left in, then never written
    /// again - hence `skip_serializing`.
    #[serde(default, skip_serializing)]
    pub last_workspace: Option<String>,
    #[serde(default)]
    pub request_settings: RequestSettings,
    #[serde(default)]
    pub startup: StartupBehavior,
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

    /// Records a workspace as the most recently opened one. Re-opening one
    /// already on the list moves it to the front rather than duplicating it,
    /// and picks up a name that changed since (`rename_workspace`).
    pub fn remember_workspace(&mut self, path: String, name: String) {
        self.recent_workspaces.retain(|entry| !same_path(&entry.path, &path));
        self.recent_workspaces.insert(0, RecentWorkspace { path, name });
        self.recent_workspaces.truncate(RECENT_WORKSPACES_LIMIT);
    }

    /// Drops one entry from the list. Only the list: the folder and
    /// everything in it stay exactly where they are.
    pub fn forget_workspace(&mut self, path: &str) {
        self.recent_workspaces.retain(|entry| !same_path(&entry.path, path));
    }

    /// Roots are keyed by path, so a renamed collection would otherwise
    /// silently lose its active environment.
    pub fn rebase_root(&mut self, old_root: &str, new_root: &str) {
        if let Some(id) = self.active_environments.remove(old_root) {
            self.active_environments.insert(new_root.to_string(), id);
        }
    }
}

/// Windows paths differ in case without naming different folders, and the
/// same workspace reached through the picker twice must not show up twice.
/// Deliberately not a full canonicalization: a folder that is gone (an
/// unplugged drive) still has to match its own entry so it can be removed.
fn same_path(a: &str, b: &str) -> bool {
    if cfg!(windows) {
        a.eq_ignore_ascii_case(b)
    } else {
        a == b
    }
}

fn state_path(app_local_data_dir: &Path) -> PathBuf {
    app_local_data_dir.join(STATE_FILE)
}

/// Corrupt or unreadable state is non-fatal: this is disposable local UI
/// state, so we just fall back to defaults rather than surface an error.
pub fn load(app_local_data_dir: &Path) -> AppState {
    let mut state: AppState = fs::read_to_string(state_path(app_local_data_dir))
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default();
    migrate_last_workspace(&mut state);
    state
}

/// Seeds the recent list from the single path older versions stored. The
/// name has to be guessed from the folder, since that is all the old field
/// held - the first `open_workspace` replaces it with the real one.
fn migrate_last_workspace(state: &mut AppState) {
    let Some(path) = state.last_workspace.take() else {
        return;
    };
    if state.recent_workspaces.is_empty() {
        let name = Path::new(&path)
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.clone());
        state.recent_workspaces.push(RecentWorkspace { path, name });
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remembering_a_workspace_moves_it_to_the_front_without_duplicating_it() {
        let mut state = AppState::default();
        state.remember_workspace("/a".into(), "A".into());
        state.remember_workspace("/b".into(), "B".into());
        // Same folder, renamed since: one entry, under the new name.
        state.remember_workspace("/a".into(), "A2".into());

        let entries: Vec<_> = state
            .recent_workspaces
            .iter()
            .map(|entry| (entry.path.as_str(), entry.name.as_str()))
            .collect();
        assert_eq!(entries, vec![("/a", "A2"), ("/b", "B")]);
    }

    #[test]
    fn the_recent_list_is_capped() {
        let mut state = AppState::default();
        for index in 0..RECENT_WORKSPACES_LIMIT + 5 {
            state.remember_workspace(format!("/ws{index}"), format!("W{index}"));
        }
        assert_eq!(state.recent_workspaces.len(), RECENT_WORKSPACES_LIMIT);
        // The oldest are the ones dropped.
        assert_eq!(state.recent_workspaces[0].path, format!("/ws{}", RECENT_WORKSPACES_LIMIT + 4));
        assert!(!state.recent_workspaces.iter().any(|entry| entry.path == "/ws0"));
    }

    #[test]
    fn forgetting_a_workspace_removes_only_that_entry() {
        let mut state = AppState::default();
        state.remember_workspace("/a".into(), "A".into());
        state.remember_workspace("/b".into(), "B".into());
        state.forget_workspace("/a");
        assert_eq!(state.recent_workspaces.len(), 1);
        assert_eq!(state.recent_workspaces[0].path, "/b");
    }

    /// State written by a version that only knew `last_workspace` must still
    /// reopen that folder, or updating the app loses the user's place.
    #[test]
    fn the_old_last_workspace_field_seeds_the_recent_list() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            state_path(dir.path()),
            r#"{"last_workspace":"/home/u/api-tests"}"#,
        )
        .unwrap();

        let state = load(dir.path());
        assert_eq!(state.recent_workspaces.len(), 1);
        assert_eq!(state.recent_workspaces[0].path, "/home/u/api-tests");
        // Only the folder name is recoverable from the old format.
        assert_eq!(state.recent_workspaces[0].name, "api-tests");
        assert!(state.last_workspace.is_none());

        // And it is not migrated a second time on top of a real list.
        save(dir.path(), &state).unwrap();
        assert_eq!(load(dir.path()).recent_workspaces.len(), 1);
    }

    #[test]
    fn startup_defaults_to_reopening_the_last_workspace() {
        assert_eq!(AppState::default().startup, StartupBehavior::LastWorkspace);
    }
}
