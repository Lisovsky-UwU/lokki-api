use super::fs_collection::{self, FOLDER_FILE, REQUEST_EXT};
use super::fs_environment::ENV_EXT;
use super::format::write_toml;
use super::naming::unique_path;
use crate::domain::{CollectionSummary, EnvironmentFile, EnvironmentScope, FolderFile, Id, RequestFile};
use crate::error::{AppError, AppResult};
use crate::import::ImportPlan;
use std::fs;
use std::path::Path;

/// What an import left on disk. The environment ids come back because the
/// caller makes the first one active - a collection whose requests all
/// start with `{{baseUrl}}` is useless until one is.
#[derive(Debug, Clone)]
pub struct WrittenImport {
    pub collection: CollectionSummary,
    pub folders: usize,
    pub requests: usize,
    pub environments: Vec<Id>,
}

/// Writes a parsed import into a new collection of its own.
///
/// Never into an existing one: a re-import would otherwise have to decide
/// what to do about every request that is already there, and the user can
/// merge two collections by dragging, which is a decision they can see.
pub fn write_plan(workspace_path: &Path, plan: &ImportPlan) -> AppResult<WrittenImport> {
    let collection = fs_collection::create_collection(workspace_path, &plan.collection_name)?;
    let root = Path::new(&collection.path);

    // Folders and requests share one ordering space per directory (see
    // store::fs_request), so the root requests carry on where the folders
    // left off instead of restarting at 1.
    let mut seq = 0u32;
    for folder in &plan.folders {
        seq += 1;
        let dir = unique_path(root, &folder.name, "");
        fs::create_dir_all(&dir).map_err(|source| AppError::Io {
            path: dir.display().to_string(),
            source,
        })?;
        write_toml(&dir.join(FOLDER_FILE), &FolderFile::new(folder.name.clone(), seq))?;
        write_requests(&dir, &folder.requests, 0)?;
    }
    write_requests(root, &plan.requests, seq)?;

    let mut environments = Vec::new();
    for environment in &plan.environments {
        let mut file = EnvironmentFile::new(environment.name.clone(), EnvironmentScope::Collection);
        file.variables = environment.variables.clone();
        let path = unique_path(
            &root.join(fs_collection::ENVIRONMENTS_DIR),
            &environment.name,
            ENV_EXT,
        );
        write_toml(&path, &file)?;
        environments.push(file.meta.sync.id);
    }

    Ok(WrittenImport {
        collection,
        folders: plan.folders.len(),
        requests: plan.request_count(),
        environments,
    })
}

/// Positions are handed out here rather than by `fs_request::create_request`:
/// that one rereads the directory for every file it adds, which a
/// specification with a few hundred operations would feel.
fn write_requests(dir: &Path, requests: &[RequestFile], start_seq: u32) -> AppResult<()> {
    for (index, request) in requests.iter().enumerate() {
        let mut request = request.clone();
        request.meta.seq = start_seq + index as u32 + 1;
        write_toml(&unique_path(dir, &request.meta.name, REQUEST_EXT), &request)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::import::{ImportedEnvironment, ImportedFolder};
    use crate::domain::{HttpMethod, Variable};
    use crate::store::fs_request::load_request;

    fn plan() -> ImportPlan {
        ImportPlan {
            collection_name: "Petstore".to_string(),
            environments: vec![ImportedEnvironment {
                name: "Production".to_string(),
                variables: vec![Variable {
                    id: Id::new(),
                    key: "baseUrl".to_string(),
                    value: "https://api.example.com".to_string(),
                    enabled: true,
                    secret: false,
                }],
            }],
            folders: vec![ImportedFolder {
                name: "pets".to_string(),
                requests: vec![
                    RequestFile::new_http("List pets", 0, HttpMethod::Get),
                    RequestFile::new_http("Create a pet", 0, HttpMethod::Post),
                ],
            }],
            requests: vec![RequestFile::new_http("Health check", 0, HttpMethod::Get)],
            warnings: Vec::new(),
        }
    }

    #[test]
    fn writes_a_collection_with_folders_requests_and_environments() {
        let workspace = tempfile::tempdir().unwrap();
        let written = write_plan(workspace.path(), &plan()).unwrap();

        assert_eq!(written.folders, 1);
        assert_eq!(written.requests, 3);
        assert_eq!(written.environments.len(), 1);

        let root = Path::new(&written.collection.path);
        assert!(root.join("collection.toml").is_file());
        assert!(root.join("pets").join("folder.toml").is_file());
        assert!(root.join("environments").join("Production.env.toml").is_file());

        let first = load_request(&root.join("pets").join("List pets.lokki.toml")).unwrap();
        assert_eq!(first.meta.seq, 1);
        assert_eq!(load_request(&root.join("pets").join("Create a pet.lokki.toml")).unwrap().meta.seq, 2);
        // The root request shares the folder's ordering space, so it comes
        // after the one folder rather than on top of it.
        assert_eq!(load_request(&root.join("Health check.lokki.toml")).unwrap().meta.seq, 2);
    }

    #[test]
    fn importing_the_same_specification_twice_keeps_both_collections() {
        let workspace = tempfile::tempdir().unwrap();
        let first = write_plan(workspace.path(), &plan()).unwrap();
        let second = write_plan(workspace.path(), &plan()).unwrap();

        assert_ne!(first.collection.path, second.collection.path);
        assert!(Path::new(&second.collection.path).ends_with("Petstore (2)"));
        assert_eq!(fs_collection::list_collections(workspace.path()).unwrap().len(), 2);
    }
}
