use super::fs_collection::{is_request_file, read_folder_meta, FOLDER_FILE, REQUEST_EXT};
use super::format::{read_toml, write_toml};
use super::naming::unique_path;
use crate::domain::{FolderFile, HttpMethod, RequestFile};
use crate::error::{AppError, AppResult};
use std::fs;
use std::path::{Path, PathBuf};

/// A request file is named `<name>.lokki.toml`; its "stem" for renaming and
/// collision-avoidance purposes is the part before that whole suffix, not
/// what `Path::file_stem` would give (which only strips `.toml`).
fn request_stem(path: &Path) -> String {
    let file_name = path
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    file_name
        .strip_suffix(REQUEST_EXT)
        .map(|s| s.to_string())
        .unwrap_or(file_name)
}

fn parent_of(path: &Path) -> AppResult<&Path> {
    path.parent()
        .ok_or_else(|| AppError::NotFound(format!("no parent directory for {}", path.display())))
}

pub fn load_request(request_path: &Path) -> AppResult<RequestFile> {
    read_toml(request_path)
}

/// Saves `request` to `request_path`, bumping its sync version/timestamp.
/// Returns the persisted file so the frontend gets the new version without
/// a second round trip.
pub fn save_request(request_path: &Path, mut request: RequestFile) -> AppResult<RequestFile> {
    request.meta.sync.touch();
    write_toml(request_path, &request)?;
    Ok(request)
}

/// Next free position among a folder's children. Folders and requests share
/// one ordering space, so both are considered.
fn next_seq(parent_path: &Path) -> AppResult<u32> {
    let mut max_seq = 0u32;
    if parent_path.is_dir() {
        let entries = fs::read_dir(parent_path).map_err(|source| AppError::Io {
            path: parent_path.display().to_string(),
            source,
        })?;
        for entry in entries {
            let entry = entry.map_err(|source| AppError::Io {
                path: parent_path.display().to_string(),
                source,
            })?;
            let path = entry.path();
            if path.is_dir() {
                max_seq = max_seq.max(read_folder_meta(&path).seq);
            } else if is_request_file(&path) {
                if let Ok(req) = load_request(&path) {
                    max_seq = max_seq.max(req.meta.seq);
                }
            }
        }
    }
    Ok(max_seq + 1)
}

pub fn create_request(parent_path: &Path, name: &str, method: HttpMethod) -> AppResult<RequestFile> {
    let seq = next_seq(parent_path)?;
    let request = RequestFile::new_http(name, seq, method);
    write_toml(&unique_path(parent_path, name, REQUEST_EXT), &request)?;
    Ok(request)
}

/// Writes a request that was built in memory (an incognito send) into a
/// folder, under a name that doesn't collide with what's already there.
/// Unlike `create_request` this keeps the request's own spec — and gives it
/// a fresh identity and position, because until now it existed nowhere.
pub fn adopt_request(parent_path: &Path, name: &str, mut request: RequestFile) -> AppResult<(PathBuf, RequestFile)> {
    request.meta.name = name.to_string();
    request.meta.seq = next_seq(parent_path)?;
    request.meta.sync = crate::domain::SyncMeta::new();
    let path = unique_path(parent_path, name, REQUEST_EXT);
    write_toml(&path, &request)?;
    Ok((path, request))
}

/// Copies a request next to itself. The copy is a new entity, not a second
/// name for the old one: `adopt_request` gives it its own id and position,
/// and the name gets a suffix so both are distinguishable in the tree.
pub fn clone_request(request_path: &Path) -> AppResult<(PathBuf, RequestFile)> {
    let request = load_request(request_path)?;
    let name = format!("{} (копия)", request.meta.name);
    adopt_request(parent_of(request_path)?, &name, request)
}

/// Writes a request to an arbitrary path the user picked in a save dialog.
/// The file keeps the same TOML shape as one inside a workspace, so it can
/// simply be dropped into a collection folder later.
pub fn export_request(file_path: &Path, mut request: RequestFile, name: &str) -> AppResult<RequestFile> {
    request.meta.name = name.to_string();
    request.meta.sync.touch();
    write_toml(file_path, &request)?;
    Ok(request)
}

pub fn delete_request(request_path: &Path) -> AppResult<()> {
    fs::remove_file(request_path).map_err(|source| AppError::Io {
        path: request_path.display().to_string(),
        source,
    })
}

pub fn create_folder(parent_path: &Path, name: &str) -> AppResult<PathBuf> {
    let seq = next_seq(parent_path)?;
    let dir = unique_path(parent_path, name, "");
    fs::create_dir_all(&dir).map_err(|source| AppError::Io {
        path: dir.display().to_string(),
        source,
    })?;
    write_toml(&dir.join(FOLDER_FILE), &FolderFile::new(name, seq))?;
    Ok(dir)
}

/// Deletes a folder and everything inside it. Callers (the UI) are
/// responsible for confirming with the user first.
pub fn delete_folder(folder_path: &Path) -> AppResult<()> {
    fs::remove_dir_all(folder_path).map_err(|source| AppError::Io {
        path: folder_path.display().to_string(),
        source,
    })
}

/// Renames a request: the display name in `meta.name` and the file itself
/// (kept in sync so the folder stays readable outside the app).
pub fn rename_request(request_path: &Path, new_name: &str) -> AppResult<(PathBuf, RequestFile)> {
    let mut request = load_request(request_path)?;
    request.meta.name = new_name.to_string();
    request.meta.sync.touch();

    let parent = parent_of(request_path)?;
    if request_stem(request_path) == super::naming::sanitize_file_stem(new_name) {
        write_toml(request_path, &request)?;
        return Ok((request_path.to_path_buf(), request));
    }

    let new_path = unique_path(parent, new_name, REQUEST_EXT);
    write_toml(&new_path, &request)?;
    fs::remove_file(request_path).map_err(|source| AppError::Io {
        path: request_path.display().to_string(),
        source,
    })?;
    Ok((new_path, request))
}

/// Renames a folder: the directory and the `name` in its metadata.
pub fn rename_folder(folder_path: &Path, new_name: &str) -> AppResult<PathBuf> {
    let mut meta = read_folder_meta(folder_path);
    meta.name = new_name.to_string();
    meta.sync.touch();

    let parent = parent_of(folder_path)?;
    let current_dir_name = folder_path
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();

    let target = if current_dir_name == super::naming::sanitize_file_stem(new_name) {
        folder_path.to_path_buf()
    } else {
        let new_path = unique_path(parent, new_name, "");
        fs::rename(folder_path, &new_path).map_err(|source| AppError::Io {
            path: folder_path.display().to_string(),
            source,
        })?;
        new_path
    };

    write_toml(&target.join(FOLDER_FILE), &meta)?;
    Ok(target)
}

/// Moves a request or folder into `target_parent`, returning its new path.
/// Name collisions in the destination are resolved the same way creation
/// resolves them, so a move never clobbers an existing entry.
pub fn move_node(source_path: &Path, target_parent: &Path) -> AppResult<PathBuf> {
    if !target_parent.is_dir() {
        return Err(AppError::NotFound(target_parent.display().to_string()));
    }
    // Moving a folder inside itself (or its own descendant) would detach the
    // whole subtree from the workspace.
    if source_path.is_dir() && target_parent.starts_with(source_path) {
        return Err(AppError::Message(
            "Нельзя переместить папку внутрь самой себя.".to_string(),
        ));
    }
    if parent_of(source_path)? == target_parent {
        return Ok(source_path.to_path_buf());
    }

    let destination = if source_path.is_dir() {
        let name = read_folder_meta(source_path).name;
        unique_path(target_parent, &name, "")
    } else {
        unique_path(target_parent, &request_stem(source_path), REQUEST_EXT)
    };

    fs::rename(source_path, &destination).map_err(|source| AppError::Io {
        path: source_path.display().to_string(),
        source,
    })?;
    Ok(destination)
}

/// Writes an explicit order onto a folder's children: each entry's `seq`
/// becomes its index in `ordered_paths`. Entries not listed keep whatever
/// position they had.
///
/// Paths that no longer exist are skipped rather than failing the whole
/// call: the caller builds the order from the tree it has rendered, which
/// can legitimately lag behind disk right after a move. Failing here used
/// to abort the drop half-way, leaving the entry moved on disk but the
/// sidebar still showing it in its old place.
pub fn reorder_children(ordered_paths: &[PathBuf]) -> AppResult<()> {
    for (index, path) in ordered_paths.iter().enumerate() {
        let seq = index as u32 + 1;
        if !path.exists() {
            continue;
        }
        if path.is_dir() {
            let mut meta = read_folder_meta(path);
            if meta.seq == seq {
                continue;
            }
            meta.seq = seq;
            meta.sync.touch();
            write_toml(&path.join(FOLDER_FILE), &meta)?;
        } else if is_request_file(path) {
            let mut request = load_request(path)?;
            if request.meta.seq == seq {
                continue;
            }
            request.meta.seq = seq;
            request.meta.sync.touch();
            write_toml(path, &request)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_save_and_load_round_trips_and_bumps_version() {
        let dir = tempfile::tempdir().unwrap();
        let created = create_request(dir.path(), "List Pets", HttpMethod::Get).unwrap();
        assert_eq!(created.meta.seq, 1);
        assert_eq!(created.meta.sync.version, 1);

        let path = dir.path().join(format!("List Pets{}", REQUEST_EXT));
        let mut loaded = load_request(&path).unwrap();
        loaded.http.as_mut().unwrap().url = "{{baseUrl}}/pets".to_string();
        let saved = save_request(&path, loaded).unwrap();
        assert_eq!(saved.meta.sync.version, 2);
        assert_eq!(saved.http.as_ref().unwrap().url, "{{baseUrl}}/pets");

        let reloaded = load_request(&path).unwrap();
        assert_eq!(reloaded.meta.sync.version, 2);
        assert_eq!(reloaded.http.as_ref().unwrap().url, "{{baseUrl}}/pets");
    }

    #[test]
    fn create_request_increments_seq_per_sibling() {
        let dir = tempfile::tempdir().unwrap();
        create_request(dir.path(), "First", HttpMethod::Get).unwrap();
        let second = create_request(dir.path(), "Second", HttpMethod::Post).unwrap();
        assert_eq!(second.meta.seq, 2);
    }

    #[test]
    fn creating_a_request_with_a_taken_name_does_not_overwrite_it() {
        let dir = tempfile::tempdir().unwrap();
        let first = create_request(dir.path(), "List Pets", HttpMethod::Get).unwrap();
        let second = create_request(dir.path(), "List Pets", HttpMethod::Post).unwrap();

        assert!(dir.path().join(format!("List Pets{}", REQUEST_EXT)).is_file());
        assert!(dir.path().join(format!("List Pets (2){}", REQUEST_EXT)).is_file());
        assert_ne!(first.meta.sync.id, second.meta.sync.id);

        // The original file must still hold the original request.
        let reloaded = load_request(&dir.path().join(format!("List Pets{}", REQUEST_EXT))).unwrap();
        assert_eq!(reloaded.meta.sync.id, first.meta.sync.id);
    }

    #[test]
    fn delete_folder_removes_folder_and_contents() {
        let dir = tempfile::tempdir().unwrap();
        let folder = create_folder(dir.path(), "Pets").unwrap();
        create_request(&folder, "List Pets", HttpMethod::Get).unwrap();
        assert!(folder.is_dir());

        delete_folder(&folder).unwrap();
        assert!(!folder.exists());
    }

    #[test]
    fn move_node_relocates_a_request_into_a_folder() {
        let dir = tempfile::tempdir().unwrap();
        create_request(dir.path(), "List Pets", HttpMethod::Get).unwrap();
        let folder = create_folder(dir.path(), "Pets").unwrap();
        let source = dir.path().join(format!("List Pets{}", REQUEST_EXT));

        let moved = move_node(&source, &folder).unwrap();

        assert!(!source.exists());
        assert_eq!(moved, folder.join(format!("List Pets{}", REQUEST_EXT)));
        assert_eq!(load_request(&moved).unwrap().meta.name, "List Pets");
    }

    #[test]
    fn move_node_refuses_to_move_a_folder_into_itself() {
        let dir = tempfile::tempdir().unwrap();
        let folder = create_folder(dir.path(), "Pets").unwrap();
        let nested = create_folder(&folder, "Inner").unwrap();
        assert!(move_node(&folder, &nested).is_err());
        assert!(folder.is_dir());
    }

    #[test]
    fn reorder_children_skips_paths_that_no_longer_exist() {
        let dir = tempfile::tempdir().unwrap();
        create_request(dir.path(), "Kept", HttpMethod::Get).unwrap();
        let kept = dir.path().join(format!("Kept{}", REQUEST_EXT));
        let vanished = dir.path().join(format!("Moved Away{}", REQUEST_EXT));

        // A stale entry (already moved elsewhere) must not abort the reorder.
        reorder_children(&[vanished, kept.clone()]).unwrap();
        assert_eq!(load_request(&kept).unwrap().meta.seq, 2);
    }

    #[test]
    fn reorder_children_assigns_positions_across_folders_and_requests() {
        let dir = tempfile::tempdir().unwrap();
        create_request(dir.path(), "First", HttpMethod::Get).unwrap();
        create_request(dir.path(), "Second", HttpMethod::Get).unwrap();
        let folder = create_folder(dir.path(), "Pets").unwrap();
        let first = dir.path().join(format!("First{}", REQUEST_EXT));
        let second = dir.path().join(format!("Second{}", REQUEST_EXT));

        // Folder first, then Second, then First.
        reorder_children(&[folder.clone(), second.clone(), first.clone()]).unwrap();

        assert_eq!(read_folder_meta(&folder).seq, 1);
        assert_eq!(load_request(&second).unwrap().meta.seq, 2);
        assert_eq!(load_request(&first).unwrap().meta.seq, 3);
    }

    #[test]
    fn rename_request_updates_name_and_file() {
        let dir = tempfile::tempdir().unwrap();
        create_request(dir.path(), "Old Name", HttpMethod::Get).unwrap();
        let old_path = dir.path().join(format!("Old Name{}", REQUEST_EXT));

        let (new_path, renamed) = rename_request(&old_path, "New Name").unwrap();

        assert!(!old_path.exists());
        assert_eq!(new_path, dir.path().join(format!("New Name{}", REQUEST_EXT)));
        assert_eq!(renamed.meta.name, "New Name");
        assert_eq!(load_request(&new_path).unwrap().meta.name, "New Name");
    }

    #[test]
    fn rename_folder_updates_directory_and_metadata() {
        let dir = tempfile::tempdir().unwrap();
        let folder = create_folder(dir.path(), "Old").unwrap();
        create_request(&folder, "Kept", HttpMethod::Get).unwrap();

        let renamed = rename_folder(&folder, "New").unwrap();

        assert!(!folder.exists());
        assert_eq!(renamed, dir.path().join("New"));
        assert_eq!(read_folder_meta(&renamed).name, "New");
        // Contents travel with the folder.
        assert!(renamed.join(format!("Kept{}", REQUEST_EXT)).is_file());
    }

    #[test]
    fn adopt_request_keeps_the_spec_but_gives_it_a_new_identity_and_place() {
        let dir = tempfile::tempdir().unwrap();
        create_request(dir.path(), "Existing", HttpMethod::Get).unwrap();

        let mut in_memory = RequestFile::new_http("Инкогнито", 0, HttpMethod::Post);
        in_memory.http.as_mut().unwrap().url = "https://example.com/pets".to_string();
        let original_id = in_memory.meta.sync.id.clone();

        let (path, saved) = adopt_request(dir.path(), "Создать питомца", in_memory).unwrap();

        assert_eq!(path.file_name().unwrap(), "Создать питомца.lokki.toml");
        assert_eq!(saved.meta.name, "Создать питомца");
        assert_eq!(saved.http.as_ref().unwrap().url, "https://example.com/pets");
        assert_eq!(saved.http.as_ref().unwrap().method, HttpMethod::Post);
        // It lands after what is already in the folder, with an identity of
        // its own rather than the one it carried in memory.
        assert_eq!(saved.meta.seq, 2);
        assert_ne!(saved.meta.sync.id, original_id);
        assert_eq!(load_request(&path).unwrap().meta.name, "Создать питомца");
    }

    #[test]
    fn clone_request_leaves_the_original_alone_and_makes_a_separate_entity() {
        let dir = tempfile::tempdir().unwrap();
        let created = create_request(dir.path(), "Список питомцев", HttpMethod::Get).unwrap();
        let original_path = dir.path().join("Список питомцев.lokki.toml");
        let mut original = load_request(&original_path).unwrap();
        original.http.as_mut().unwrap().url = "https://example.com/pets".to_string();
        save_request(&original_path, original).unwrap();

        let (clone_path, clone) = clone_request(&original_path).unwrap();

        assert_eq!(clone.meta.name, "Список питомцев (копия)");
        assert_eq!(clone_path.file_name().unwrap(), "Список питомцев (копия).lokki.toml");
        assert_eq!(clone.http.as_ref().unwrap().url, "https://example.com/pets");
        assert_ne!(clone.meta.sync.id, created.meta.sync.id);
        // The original stays exactly as it was, copy included in the folder.
        assert!(original_path.is_file());
        assert_eq!(load_request(&original_path).unwrap().meta.sync.id, created.meta.sync.id);

        // Cloning twice must not overwrite the first copy.
        let (second_path, _) = clone_request(&original_path).unwrap();
        assert_eq!(second_path.file_name().unwrap(), "Список питомцев (копия) (2).lokki.toml");
    }

    #[test]
    fn adopt_request_does_not_overwrite_a_taken_name() {
        let dir = tempfile::tempdir().unwrap();
        create_request(dir.path(), "Питомцы", HttpMethod::Get).unwrap();

        let (path, _) = adopt_request(dir.path(), "Питомцы", RequestFile::new_http("x", 0, HttpMethod::Get)).unwrap();

        assert_eq!(path.file_name().unwrap(), "Питомцы (2).lokki.toml");
    }

    #[test]
    fn export_request_writes_a_loadable_file_anywhere() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("вне-пространства.lokki.toml");

        export_request(&target, RequestFile::new_http("x", 0, HttpMethod::Delete), "Удалить").unwrap();

        let loaded = load_request(&target).unwrap();
        assert_eq!(loaded.meta.name, "Удалить");
        assert_eq!(loaded.http.unwrap().method, HttpMethod::Delete);
    }

    #[test]
    fn delete_request_removes_file() {
        let dir = tempfile::tempdir().unwrap();
        create_request(dir.path(), "Temp", HttpMethod::Get).unwrap();
        let path = dir.path().join(format!("Temp{}", REQUEST_EXT));
        assert!(path.is_file());
        delete_request(&path).unwrap();
        assert!(!path.is_file());
    }
}
