use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("io error at {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse {path}: {source}")]
    Parse {
        path: String,
        #[source]
        source: toml::de::Error,
    },
    #[error("failed to serialize {path}: {source}")]
    Serialize {
        path: String,
        #[source]
        source: toml::ser::Error,
    },
    #[error("not found: {0}")]
    NotFound(String),
    /// A message written for the user and shown verbatim in the UI - unlike
    /// the variants above, it carries no English prefix to prepend to a
    /// Russian sentence.
    #[error("{0}")]
    Message(String),
    #[error("request execution failed: {0}")]
    Execution(String),
}

// Tauri commands need their error type to serialize across IPC.
impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

pub type AppResult<T> = Result<T, AppError>;
