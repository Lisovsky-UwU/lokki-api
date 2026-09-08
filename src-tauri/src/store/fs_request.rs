use super::fs_collection::{is_request_file, REQUEST_EXT};
use super::format::{read_toml, write_toml};
use super::naming::unique_path;
use crate::domain::{HttpMethod, RequestFile};
use crate::error::{AppError, AppResult};
use std::fs;
use std::path::{Path, PathBuf};

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
            if is_request_file(&path) {
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

pub fn delete_request(request_path: &Path) -> AppResult<()> {
    fs::remove_file(request_path).map_err(|source| AppError::Io {
        path: request_path.display().to_string(),
        source,
    })
}

pub fn create_folder(parent_path: &Path, name: &str) -> AppResult<PathBuf> {
    let dir = unique_path(parent_path, name, "");
    fs::create_dir_all(&dir).map_err(|source| AppError::Io {
        path: dir.display().to_string(),
        source,
    })?;
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
    fn delete_request_removes_file() {
        let dir = tempfile::tempdir().unwrap();
        create_request(dir.path(), "Temp", HttpMethod::Get).unwrap();
        let path = dir.path().join(format!("Temp{}", REQUEST_EXT));
        assert!(path.is_file());
        delete_request(&path).unwrap();
        assert!(!path.is_file());
    }
}
