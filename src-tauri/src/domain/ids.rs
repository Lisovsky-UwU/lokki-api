use serde::{Deserialize, Serialize};
use std::fmt;

/// Stable entity identifier, independent of file path or name.
/// Backed by a ULID so future change-log ordering can rely on
/// lexicographic == chronological order without a separate sequence counter.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Id(String);

impl Id {
    pub fn new() -> Self {
        Id(ulid::Ulid::new().to_string())
    }
}

impl Default for Id {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for Id {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<str> for Id {
    fn as_ref(&self) -> &str {
        &self.0
    }
}
