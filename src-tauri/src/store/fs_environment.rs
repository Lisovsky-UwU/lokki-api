use super::format::{read_toml, write_toml};
use crate::domain::{EnvironmentFile, EnvironmentScope};
use crate::error::{AppError, AppResult};
use std::fs;
use std::path::{Path, PathBuf};

pub const ENV_EXT: &str = ".env.toml";

fn is_env_file(path: &Path) -> bool {
    path.file_name()
        .and_then(|s| s.to_str())
        .map(|s| s.ends_with(ENV_EXT))
        .unwrap_or(false)
}

fn sanitize_file_stem(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            other => other,
        })
        .collect()
}

/// Lists environments directly inside `scope_path` (a workspace's
/// top-level `environments/` dir for global scope, or a collection's
/// `environments/` dir for collection scope), paired with each one's file
/// path — callers (the command layer) need the path to later save it.
pub fn list_environments(scope_path: &Path) -> AppResult<Vec<(PathBuf, EnvironmentFile)>> {
    let mut out = Vec::new();
    if !scope_path.is_dir() {
        return Ok(out);
    }
    let entries = fs::read_dir(scope_path).map_err(|source| AppError::Io {
        path: scope_path.display().to_string(),
        source,
    })?;
    for entry in entries {
        let entry = entry.map_err(|source| AppError::Io {
            path: scope_path.display().to_string(),
            source,
        })?;
        let path = entry.path();
        if path.is_file() && is_env_file(&path) {
            let file = read_toml(&path)?;
            out.push((path, file));
        }
    }
    out.sort_by(|a, b| a.1.meta.name.to_lowercase().cmp(&b.1.meta.name.to_lowercase()));
    Ok(out)
}

fn env_path(scope_path: &Path, name: &str) -> PathBuf {
    scope_path.join(format!("{}{}", sanitize_file_stem(name), ENV_EXT))
}

pub fn create_environment(
    scope_path: &Path,
    name: &str,
    scope: EnvironmentScope,
) -> AppResult<(PathBuf, EnvironmentFile)> {
    let env = EnvironmentFile::new(name, scope);
    let path = env_path(scope_path, name);
    write_toml(&path, &env)?;
    Ok((path, env))
}

/// Saves `environment` to `env_path`, bumping its sync version/timestamp.
pub fn save_environment(env_path: &Path, mut environment: EnvironmentFile) -> AppResult<EnvironmentFile> {
    environment.meta.sync.touch();
    write_toml(env_path, &environment)?;
    Ok(environment)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_list_and_save_round_trips_and_bumps_version() {
        let dir = tempfile::tempdir().unwrap();
        create_environment(dir.path(), "Dev", EnvironmentScope::Collection).unwrap();
        let envs = list_environments(dir.path()).unwrap();
        assert_eq!(envs.len(), 1);
        assert_eq!(envs[0].1.meta.sync.version, 1);

        let (path, mut env) = envs.into_iter().next().unwrap();
        env.variables.push(crate::domain::Variable {
            id: crate::domain::Id::new(),
            key: "baseUrl".into(),
            value: "https://example.com".into(),
            enabled: true,
            secret: false,
        });
        let saved = save_environment(&path, env).unwrap();
        assert_eq!(saved.meta.sync.version, 2);
        assert_eq!(saved.variables.len(), 1);
    }
}
