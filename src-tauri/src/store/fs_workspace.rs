use super::fs_collection::list_collections;
use super::format::{read_toml, write_toml};
use crate::domain::{CollectionSummary, WorkspaceFile};
use crate::error::{AppError, AppResult};
use crate::i18n::messages;
use std::fs;
use std::path::{Path, PathBuf};

const LOKKI_DIR: &str = ".lokki";
const WORKSPACE_FILE: &str = "workspace.toml";
const GITIGNORE_FILE: &str = ".gitignore";
const SECRETS_FILE: &str = "secrets.local.toml";
/// A file manager writes these by itself, just from the folder having been
/// looked at. Refusing to create a workspace over one would read as a bug.
const IGNORED_WHEN_EMPTY: [&str; 3] = [".DS_Store", "Thumbs.db", "desktop.ini"];

fn lokki_dir(workspace_path: &Path) -> PathBuf {
    workspace_path.join(LOKKI_DIR)
}

fn workspace_file_path(workspace_path: &Path) -> PathBuf {
    lokki_dir(workspace_path).join(WORKSPACE_FILE)
}

pub fn is_workspace(workspace_path: &Path) -> bool {
    workspace_file_path(workspace_path).is_file()
}

/// Opens an existing workspace. Creating one is a separate, deliberate act
/// (`create_workspace`): opening a folder that was never a workspace used to
/// initialize it silently, which turned a mistyped or wrongly picked folder
/// into a new empty workspace.
pub fn open_workspace(workspace_path: &Path) -> AppResult<(WorkspaceFile, Vec<CollectionSummary>)> {
    let marker = workspace_file_path(workspace_path);
    if !marker.is_file() {
        return Err(AppError::Message(messages::no_workspace_in_folder(
            &workspace_path.display().to_string(),
        )));
    }
    let workspace = read_toml(&marker)?;
    let collections = list_collections(workspace_path)?;
    Ok((workspace, collections))
}

/// Whether a workspace may be created in this folder. Split out of
/// `create_workspace` so the picker can ask the moment a folder is chosen,
/// rather than making the user name a workspace that was never going to be
/// created. `create_workspace` still calls it: the two run at different
/// times, and the folder can gain files in between.
///
/// The folder has to be empty because a workspace takes it over whole
/// (collections become directories in it, `.lokki` sits at its root), and
/// the folder picker makes an unrelated folder - Documents, a source tree,
/// the home directory - one mis-click away. "Already a workspace" is checked
/// first, because "open it instead" is the useful thing to say about that
/// folder.
pub fn check_new_workspace_folder(workspace_path: &Path) -> AppResult<()> {
    if is_workspace(workspace_path) {
        return Err(AppError::Message(messages::workspace_already_exists(
            &workspace_path.display().to_string(),
        )));
    }
    if !is_effectively_empty(workspace_path)? {
        return Err(AppError::Message(messages::workspace_folder_not_empty(
            &workspace_path.display().to_string(),
        )));
    }
    Ok(())
}

/// Initializes `workspace_path` as a workspace named `name`.
pub fn create_workspace(workspace_path: &Path, name: &str) -> AppResult<WorkspaceFile> {
    check_new_workspace_folder(workspace_path)?;
    init_workspace(workspace_path, name)
}

fn is_effectively_empty(dir: &Path) -> AppResult<bool> {
    let entries = fs::read_dir(dir).map_err(|source| AppError::Io {
        path: dir.display().to_string(),
        source,
    })?;
    for entry in entries {
        let entry = entry.map_err(|source| AppError::Io {
            path: dir.display().to_string(),
            source,
        })?;
        let name = entry.file_name();
        if !IGNORED_WHEN_EMPTY.iter().any(|ignored| name.eq_ignore_ascii_case(ignored)) {
            return Ok(false);
        }
    }
    Ok(true)
}

/// The workspace name is metadata, not the folder name - renaming it leaves
/// the directory (and every path the app has cached) untouched.
pub fn rename_workspace(workspace_path: &Path, new_name: &str) -> AppResult<WorkspaceFile> {
    let marker = workspace_file_path(workspace_path);
    let mut workspace: WorkspaceFile = read_toml(&marker)?;
    workspace.name = new_name.to_string();
    workspace.sync.touch();
    write_toml(&marker, &workspace)?;
    Ok(workspace)
}

fn init_workspace(workspace_path: &Path, name: &str) -> AppResult<WorkspaceFile> {
    fs::create_dir_all(&lokki_dir(workspace_path)).map_err(|source| AppError::Io {
        path: lokki_dir(workspace_path).display().to_string(),
        source,
    })?;
    let workspace = WorkspaceFile::new(name);
    write_toml(&workspace_file_path(workspace_path), &workspace)?;

    let gitignore_path = lokki_dir(workspace_path).join(GITIGNORE_FILE);
    if !gitignore_path.is_file() {
        fs::write(&gitignore_path, format!("{}\n", SECRETS_FILE)).map_err(|source| AppError::Io {
            path: gitignore_path.display().to_string(),
            source,
        })?;
    }
    Ok(workspace)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_workspace_writes_marker_and_gitignore() {
        let dir = tempfile::tempdir().unwrap();
        let workspace = create_workspace(dir.path(), "Моё пространство").unwrap();
        assert_eq!(workspace.name, "Моё пространство");
        assert!(dir.path().join(LOKKI_DIR).join(WORKSPACE_FILE).is_file());
        assert!(dir.path().join(LOKKI_DIR).join(GITIGNORE_FILE).is_file());
        assert_eq!(workspace.schema_version, crate::domain::workspace::CURRENT_SCHEMA_VERSION);
    }

    #[test]
    fn create_workspace_refuses_a_folder_that_is_already_one() {
        let dir = tempfile::tempdir().unwrap();
        create_workspace(dir.path(), "First").unwrap();
        assert!(create_workspace(dir.path(), "Second").is_err());
        // The first workspace is left intact rather than half-overwritten.
        assert_eq!(open_workspace(dir.path()).unwrap().0.name, "First");
    }

    #[test]
    fn create_workspace_refuses_a_folder_that_already_holds_files() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("notes.txt"), "unrelated").unwrap();
        assert!(create_workspace(dir.path(), "Demo").is_err());
        assert!(!is_workspace(dir.path()));
    }

    #[test]
    fn create_workspace_refuses_a_folder_holding_only_a_subdirectory() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir(dir.path().join("src")).unwrap();
        assert!(create_workspace(dir.path(), "Demo").is_err());
    }

    /// The picker asks before prompting for a name, so the two have to agree
    /// on every folder - a check that passed and a create that then failed
    /// would put the refusal back after the name prompt.
    #[test]
    fn the_pre_check_and_create_agree() {
        let empty = tempfile::tempdir().unwrap();
        assert!(check_new_workspace_folder(empty.path()).is_ok());
        assert!(create_workspace(empty.path(), "Demo").is_ok());
        // Now occupied by the workspace just made.
        assert!(check_new_workspace_folder(empty.path()).is_err());

        let occupied = tempfile::tempdir().unwrap();
        fs::write(occupied.path().join("notes.txt"), "unrelated").unwrap();
        assert!(check_new_workspace_folder(occupied.path()).is_err());
        assert!(create_workspace(occupied.path(), "Demo").is_err());
    }

    /// Opening the folder in Explorer or Finder before picking it must not
    /// be enough to make it look occupied.
    #[test]
    fn os_metadata_files_do_not_count_as_content() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join(".DS_Store"), "").unwrap();
        fs::write(dir.path().join("desktop.ini"), "").unwrap();
        assert!(create_workspace(dir.path(), "Demo").is_ok());
    }

    #[test]
    fn open_workspace_fails_on_a_plain_folder_and_keeps_identity_afterwards() {
        let dir = tempfile::tempdir().unwrap();
        assert!(open_workspace(dir.path()).is_err());

        let created = create_workspace(dir.path(), "Demo").unwrap();
        let (first, collections) = open_workspace(dir.path()).unwrap();
        let (second, _) = open_workspace(dir.path()).unwrap();
        assert!(collections.is_empty());
        assert_eq!(created.sync.id, first.sync.id);
        assert_eq!(first.sync.id, second.sync.id);
    }

    #[test]
    fn rename_workspace_changes_the_name_but_not_the_folder() {
        let dir = tempfile::tempdir().unwrap();
        let created = create_workspace(dir.path(), "Old").unwrap();
        let renamed = rename_workspace(dir.path(), "New").unwrap();
        assert_eq!(renamed.name, "New");
        assert_eq!(renamed.sync.id, created.sync.id);
        assert!(renamed.sync.version > created.sync.version);
        // Same folder, so nothing the app has cached by path goes stale.
        assert_eq!(open_workspace(dir.path()).unwrap().0.name, "New");
    }
}
