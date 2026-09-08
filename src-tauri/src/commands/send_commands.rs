use crate::domain::{EnvironmentFile, RequestFile};
use crate::error::{AppError, AppResult};
use crate::exec::{resolve_http_request, ExecutionContext, ExecutionOutcome, HttpExecutor, ProtocolExecutor};
use crate::interpolate::{Resolver, VariableScope};
use crate::secrets::{local_file::LocalFileSecretStore, SecretStore};
use crate::store::{fs_app_state, fs_environment};
use serde::Serialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager, State};

/// `send_request`'s result: the HTTP outcome plus any `{{variable}}` names
/// that couldn't be resolved (sent verbatim in the request) — surfaced so
/// the UI can warn the user rather than silently sending literal `{{...}}`.
#[derive(Debug, Clone, Serialize)]
pub struct SendResult {
    #[serde(flatten)]
    pub outcome: ExecutionOutcome,
    pub unresolved_variables: Vec<String>,
}

fn app_local_data_dir(app: &AppHandle) -> AppResult<PathBuf> {
    app.path()
        .app_local_data_dir()
        .map_err(|_| AppError::NotFound("app local data dir".to_string()))
}

fn active_environment_for(
    root_path: &Path,
    active_ids: &HashMap<String, crate::domain::Id>,
) -> AppResult<Option<EnvironmentFile>> {
    let Some(active_id) = active_ids.get(&root_path.display().to_string()) else {
        return Ok(None);
    };
    let envs = fs_environment::list_environments(&root_path.join("environments"))?;
    Ok(envs.into_iter().map(|(_, file)| file).find(|e| &e.meta.sync.id == active_id))
}

/// Builds a `{{var}} -> value` map from an environment's enabled
/// variables, resolving `secret = true` values via `SecretStore` (their
/// on-disk `value` is blank — see domain::environment docs).
fn env_to_scope(
    env: Option<EnvironmentFile>,
    workspace_path: &Path,
    secrets: &dyn SecretStore,
) -> AppResult<VariableScope> {
    let mut map = HashMap::new();
    if let Some(env) = env {
        for var in env.variables {
            if !var.enabled {
                continue;
            }
            let value = if var.secret {
                secrets.get(workspace_path, &var.id)?.unwrap_or_default()
            } else {
                var.value
            };
            map.insert(var.key, value);
        }
    }
    Ok(VariableScope(map))
}

#[tauri::command]
pub async fn send_request(
    app: AppHandle,
    // Shared across sends: building a reqwest::Client per request would
    // rebuild the whole rustls/TLS root store every time and throw away
    // connection pooling.
    executor: State<'_, HttpExecutor>,
    request: RequestFile,
    collection_path: String,
) -> AppResult<SendResult> {
    let http_spec = request
        .http
        .ok_or_else(|| AppError::NotFound("request has no http spec".to_string()))?;

    let collection_dir = Path::new(&collection_path);
    let workspace_dir = collection_dir.parent().unwrap_or(collection_dir);

    let data_dir = app_local_data_dir(&app)?;
    let state = fs_app_state::load(&data_dir);
    let secrets = LocalFileSecretStore;

    let global_env = active_environment_for(workspace_dir, &state.active_environments)?;
    let collection_env = active_environment_for(collection_dir, &state.active_environments)?;
    let has_collection_env = collection_env.is_some();

    let global_scope = env_to_scope(global_env, workspace_dir, &secrets)?;
    let collection_scope = if has_collection_env {
        Some(env_to_scope(collection_env, workspace_dir, &secrets)?)
    } else {
        None
    };

    let resolver = Resolver::new(global_scope, collection_scope);
    let (resolved, unresolved_variables) = resolve_http_request(&http_spec, &resolver);

    let outcome = executor
        .execute(&resolved, &ExecutionContext::default())
        .await
        .map_err(|e| AppError::Execution(e.to_string()))?;

    Ok(SendResult {
        outcome,
        unresolved_variables,
    })
}
