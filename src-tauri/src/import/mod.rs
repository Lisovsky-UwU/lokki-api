//! Building a collection out of somebody else's format.
//!
//! An import is deliberately two halves: a parser turns the source document
//! into an `ImportPlan` - plain domain entities, nothing written yet - and
//! `store::fs_import` lays that plan out on disk. Only the first half is
//! format-specific, so Insomnia, Postman and the rest attach as siblings of
//! `openapi` without the writer or the command layer knowing about them.

pub mod openapi;

use crate::domain::{RequestFile, Variable};

/// A group of requests that becomes one folder in the collection.
#[derive(Debug, Clone)]
pub struct ImportedFolder {
    pub name: String,
    pub requests: Vec<RequestFile>,
}

#[derive(Debug, Clone)]
pub struct ImportedEnvironment {
    pub name: String,
    pub variables: Vec<Variable>,
}

/// Everything an import would create, before any of it is written.
///
/// `requests` are the ones that belong in no folder - they land at the
/// collection root.
#[derive(Debug, Clone, Default)]
pub struct ImportPlan {
    pub collection_name: String,
    pub environments: Vec<ImportedEnvironment>,
    pub folders: Vec<ImportedFolder>,
    pub requests: Vec<RequestFile>,
    /// What the source asked for and the import could not reproduce. Every
    /// one of them is something the user may have to finish by hand, so
    /// they are shown when the import is done rather than logged.
    pub warnings: Vec<String>,
}

impl ImportPlan {
    pub fn request_count(&self) -> usize {
        self.requests.len() + self.folders.iter().map(|f| f.requests.len()).sum::<usize>()
    }
}
