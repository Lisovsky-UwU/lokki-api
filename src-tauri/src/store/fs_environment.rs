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

/// Lists environments directly inside `scope_path` (a workspace's
/// top-level `environments/` dir for global scope, or a collection's
/// `environments/` dir for collection scope), paired with each one's file
/// path - callers (the command layer) need the path to later save it.
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

pub fn create_environment(
    scope_path: &Path,
    name: &str,
    scope: EnvironmentScope,
) -> AppResult<(PathBuf, EnvironmentFile)> {
    let env = EnvironmentFile::new(name, scope);
    let path = super::naming::unique_path(scope_path, name, ENV_EXT);
    write_toml(&path, &env)?;
    Ok((path, env))
}

fn env_stem(path: &Path) -> String {
    let file_name = path
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    file_name.strip_suffix(ENV_EXT).map(str::to_string).unwrap_or(file_name)
}

/// Saves `environment` to `env_path`, bumping its sync version/timestamp.
/// A changed `meta.name` also renames the file, the same way requests and
/// folders work - the folder has to stay readable outside the app, so the
/// display name and the file name must not drift apart. Returns the path it
/// ended up at, which callers hold on to for the next save.
pub fn save_environment(env_path: &Path, mut environment: EnvironmentFile) -> AppResult<(PathBuf, EnvironmentFile)> {
    environment.meta.sync.touch();

    if env_stem(env_path) == super::naming::sanitize_file_stem(&environment.meta.name) {
        write_toml(env_path, &environment)?;
        return Ok((env_path.to_path_buf(), environment));
    }

    let parent = env_path
        .parent()
        .ok_or_else(|| AppError::NotFound(format!("no parent directory for {}", env_path.display())))?;
    let new_path = super::naming::unique_path(parent, &environment.meta.name, ENV_EXT);
    write_toml(&new_path, &environment)?;
    if env_path.exists() {
        fs::remove_file(env_path).map_err(|source| AppError::Io {
            path: env_path.display().to_string(),
            source,
        })?;
    }
    Ok((new_path, environment))
}

pub fn delete_environment(env_path: &Path) -> AppResult<()> {
    fs::remove_file(env_path).map_err(|source| AppError::Io {
        path: env_path.display().to_string(),
        source,
    })
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
        let (saved_path, saved) = save_environment(&path, env).unwrap();
        assert_eq!(saved_path, path);
        assert_eq!(saved.meta.sync.version, 2);
        assert_eq!(saved.variables.len(), 1);
    }

    #[test]
    fn saving_under_a_new_name_renames_the_file() {
        let dir = tempfile::tempdir().unwrap();
        let (path, mut env) = create_environment(dir.path(), "Dev", EnvironmentScope::Global).unwrap();
        let id = env.meta.sync.id.clone();

        env.meta.name = "Staging".to_string();
        let (new_path, saved) = save_environment(&path, env).unwrap();

        assert_eq!(new_path.file_name().unwrap(), "Staging.env.toml");
        assert!(!path.exists());
        // The id is what the active-environment setting points at, so a
        // rename must not disturb it.
        assert_eq!(saved.meta.sync.id, id);
        let listed = list_environments(dir.path()).unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].1.meta.name, "Staging");
    }

    #[test]
    fn delete_environment_removes_the_file_and_leaves_the_others() {
        let dir = tempfile::tempdir().unwrap();
        let (doomed, _) = create_environment(dir.path(), "Dev", EnvironmentScope::Global).unwrap();
        create_environment(dir.path(), "Prod", EnvironmentScope::Global).unwrap();

        delete_environment(&doomed).unwrap();

        assert!(!doomed.exists());
        let left = list_environments(dir.path()).unwrap();
        assert_eq!(left.len(), 1);
        assert_eq!(left[0].1.meta.name, "Prod");
    }

    #[test]
    fn renaming_onto_a_taken_name_does_not_overwrite_it() {
        let dir = tempfile::tempdir().unwrap();
        create_environment(dir.path(), "Prod", EnvironmentScope::Global).unwrap();
        let (path, mut env) = create_environment(dir.path(), "Dev", EnvironmentScope::Global).unwrap();

        env.meta.name = "Prod".to_string();
        let (new_path, _) = save_environment(&path, env).unwrap();

        assert_eq!(new_path.file_name().unwrap(), "Prod (2).env.toml");
        assert_eq!(list_environments(dir.path()).unwrap().len(), 2);
    }
}
