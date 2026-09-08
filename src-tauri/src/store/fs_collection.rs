use super::format::{read_toml, write_toml};
use crate::domain::{CollectionFile, CollectionSummary, FolderFile, Protocol, RequestFile};
use crate::error::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

pub const COLLECTION_FILE: &str = "collection.toml";
pub const FOLDER_FILE: &str = "folder.toml";
pub const REQUEST_EXT: &str = ".lokki.toml";
pub const ENVIRONMENTS_DIR: &str = "environments";

/// Reads a folder's `folder.toml`, falling back to the directory name and
/// `seq = 0` for folders created before folder metadata existed.
pub fn read_folder_meta(dir: &Path) -> FolderFile {
    let derived_name = dir
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    read_toml::<FolderFile>(&dir.join(FOLDER_FILE)).unwrap_or_else(|_| FolderFile::new(derived_name, 0))
}

pub fn is_request_file(path: &Path) -> bool {
    path.file_name()
        .and_then(|s| s.to_str())
        .map(|s| s.ends_with(REQUEST_EXT))
        .unwrap_or(false)
}

/// Directories that are structural, not part of the request/folder tree
/// the user sees in the sidebar (environments live alongside a collection,
/// `.lokki` is workspace-internal bookkeeping).
fn is_excluded_dir(name: &str) -> bool {
    name == ENVIRONMENTS_DIR || name.starts_with('.')
}

pub fn create_collection(workspace_path: &Path, name: &str) -> AppResult<CollectionSummary> {
    let dir = super::naming::unique_path(workspace_path, name, "");
    fs::create_dir_all(&dir).map_err(|source| AppError::Io {
        path: dir.display().to_string(),
        source,
    })?;
    let file = CollectionFile::new(name);
    write_toml(&dir.join(COLLECTION_FILE), &file)?;
    Ok(CollectionSummary {
        name: name.to_string(),
        path: dir.display().to_string(),
    })
}

pub fn list_collections(workspace_path: &Path) -> AppResult<Vec<CollectionSummary>> {
    let mut out = Vec::new();
    if !workspace_path.is_dir() {
        return Ok(out);
    }
    let entries = fs::read_dir(workspace_path).map_err(|source| AppError::Io {
        path: workspace_path.display().to_string(),
        source,
    })?;
    for entry in entries {
        let entry = entry.map_err(|source| AppError::Io {
            path: workspace_path.display().to_string(),
            source,
        })?;
        let path = entry.path();
        let marker = path.join(COLLECTION_FILE);
        if path.is_dir() && marker.is_file() {
            let file: CollectionFile = read_toml(&marker)?;
            out.push(CollectionSummary {
                name: file.name,
                path: path.display().to_string(),
            });
        }
    }
    out.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(out)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum CollectionTreeNode {
    Folder {
        name: String,
        path: String,
        seq: u32,
        children: Vec<CollectionTreeNode>,
    },
    Request {
        name: String,
        path: String,
        seq: u32,
        protocol: Protocol,
        method: Option<crate::domain::HttpMethod>,
    },
}

impl CollectionTreeNode {
    pub fn path(&self) -> &str {
        match self {
            CollectionTreeNode::Folder { path, .. } | CollectionTreeNode::Request { path, .. } => path,
        }
    }
}

pub fn load_collection_tree(collection_path: &Path) -> AppResult<CollectionTreeNode> {
    let name = collection_path
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    let children = load_children(collection_path)?;
    Ok(CollectionTreeNode::Folder {
        name,
        path: collection_path.display().to_string(),
        seq: 0,
        children,
    })
}

fn load_children(dir: &Path) -> AppResult<Vec<CollectionTreeNode>> {
    let mut out = Vec::new();
    let entries = fs::read_dir(dir).map_err(|source| AppError::Io {
        path: dir.display().to_string(),
        source,
    })?;
    for entry in entries {
        let entry = entry.map_err(|source| AppError::Io {
            path: dir.display().to_string(),
            source,
        })?;
        let path = entry.path();
        if path.is_dir() {
            let name = path
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();
            if is_excluded_dir(&name) {
                continue;
            }
            let meta = read_folder_meta(&path);
            let children = load_children(&path)?;
            out.push(CollectionTreeNode::Folder {
                name: meta.name,
                path: path.display().to_string(),
                seq: meta.seq,
                children,
            });
        } else if is_request_file(&path) {
            let request: RequestFile = read_toml(&path)?;
            out.push(CollectionTreeNode::Request {
                name: request.meta.name,
                path: path.display().to_string(),
                seq: request.meta.seq,
                protocol: request.meta.protocol,
                method: request.http.map(|h| h.method),
            });
        }
    }
    out.sort_by(|a, b| tree_sort_key(a).cmp(&tree_sort_key(b)));
    Ok(out)
}

/// Folders and requests share one ordering space so the user can arrange
/// them in any order by dragging; `name` only breaks ties (notably for
/// legacy folders that predate folder metadata and default to `seq = 0`).
fn tree_sort_key(node: &CollectionTreeNode) -> (u32, String) {
    match node {
        CollectionTreeNode::Folder { name, seq, .. } => (*seq, name.to_lowercase()),
        CollectionTreeNode::Request { name, seq, .. } => (*seq, name.to_lowercase()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::HttpMethod;

    #[test]
    fn create_and_list_collections_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        create_collection(dir.path(), "Petstore").unwrap();
        let collections = list_collections(dir.path()).unwrap();
        assert_eq!(collections.len(), 1);
        assert_eq!(collections[0].name, "Petstore");
    }

    #[test]
    fn tree_excludes_environments_and_hidden_dirs_but_includes_requests() {
        let dir = tempfile::tempdir().unwrap();
        let summary = create_collection(dir.path(), "Petstore").unwrap();
        let collection_path = Path::new(&summary.path);
        fs::create_dir_all(collection_path.join(ENVIRONMENTS_DIR)).unwrap();
        fs::create_dir_all(collection_path.join("Pets")).unwrap();
        let req = RequestFile::new_http("List Pets", 1, HttpMethod::Get);
        write_toml(
            &collection_path.join("Pets").join(format!("List Pets{}", REQUEST_EXT)),
            &req,
        )
        .unwrap();

        let tree = load_collection_tree(collection_path).unwrap();
        let CollectionTreeNode::Folder { children, .. } = tree else {
            panic!("expected folder root")
        };
        assert_eq!(children.len(), 1, "environments dir must be excluded");
        let CollectionTreeNode::Folder { name, children, .. } = &children[0] else {
            panic!("expected Pets folder")
        };
        assert_eq!(name, "Pets");
        assert_eq!(children.len(), 1);
        assert!(matches!(children[0], CollectionTreeNode::Request { .. }));
    }
}
