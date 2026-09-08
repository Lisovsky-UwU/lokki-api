use crate::error::{AppError, AppResult};
use serde::{de::DeserializeOwned, Serialize};
use std::fs;
use std::path::Path;

pub fn read_toml<T: DeserializeOwned>(path: &Path) -> AppResult<T> {
    let raw = fs::read_to_string(path).map_err(|source| AppError::Io {
        path: path.display().to_string(),
        source,
    })?;
    toml::from_str(&raw).map_err(|source| AppError::Parse {
        path: path.display().to_string(),
        source,
    })
}

pub fn write_toml<T: Serialize>(path: &Path, value: &T) -> AppResult<()> {
    let raw = toml::to_string_pretty(value).map_err(|source| AppError::Serialize {
        path: path.display().to_string(),
        source,
    })?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| AppError::Io {
            path: parent.display().to_string(),
            source,
        })?;
    }
    fs::write(path, raw).map_err(|source| AppError::Io {
        path: path.display().to_string(),
        source,
    })
}
