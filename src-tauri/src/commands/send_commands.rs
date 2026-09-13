use crate::domain::{EnvironmentFile, RequestFile};
use crate::error::{AppError, AppResult};
use crate::exec::{
    resolve_http_request, ExecutionContext, ExecutionOutcome, ExecutionTrace, HttpExecutor, ProtocolExecutor, TraceRecorder,
};
use crate::interpolate::{Resolver, VariableScope};
use crate::secrets::{local_file::LocalFileSecretStore, SecretStore};
use crate::store::{fs_app_state, fs_environment};
use serde::Serialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tokio::sync::oneshot;
use tauri::{AppHandle, Manager, State};

/// Sends that are currently in flight, keyed by the id the frontend made up
/// for this particular send. Dropping the sender (or firing it) is what
/// cancels: the executor's future is awaited inside a `select!`, and letting
/// that future go closes the connection.
///
/// Keyed by send rather than by request path so a rename mid-flight can't
/// orphan the entry, and so several sends of one request stay separable if
/// that is ever allowed.
#[derive(Default)]
pub struct InFlightSends(Mutex<HashMap<String, oneshot::Sender<()>>>);

impl InFlightSends {
    fn register(&self, send_id: String) -> oneshot::Receiver<()> {
        let (tx, rx) = oneshot::channel();
        self.0.lock().expect("in-flight registry poisoned").insert(send_id, tx);
        rx
    }

    fn forget(&self, send_id: &str) {
        self.0.lock().expect("in-flight registry poisoned").remove(send_id);
    }

    /// Returns false when there was nothing to cancel — the send had already
    /// finished, which is not an error worth surfacing.
    fn cancel(&self, send_id: &str) -> bool {
        let sender = self.0.lock().expect("in-flight registry poisoned").remove(send_id);
        match sender {
            // A closed receiver means the send is already past awaiting;
            // either way there is nothing left to stop.
            Some(sender) => sender.send(()).is_ok(),
            None => false,
        }
    }
}

/// `send_request`'s result: the HTTP outcome, where the time went, and any
/// `{{variable}}` names that couldn't be resolved (sent verbatim in the
/// request) — surfaced so the UI can warn the user rather than silently
/// sending literal `{{...}}`.
#[derive(Debug, Clone, Serialize)]
pub struct SendResult {
    #[serde(flatten)]
    pub outcome: ExecutionOutcome,
    pub trace: ExecutionTrace,
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
    in_flight: State<'_, InFlightSends>,
    request: RequestFile,
    collection_path: String,
    // Made up by the frontend for this send; `cancel_send` refers to it.
    send_id: String,
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

    // Checked before sending: a `{{var}}` left in the address produces a URL
    // that can't be parsed, and reqwest reports that as "relative URL without
    // a base" — true, and useless. Elsewhere (headers, body, auth) the
    // request is still sent and the unresolved names come back as a warning,
    // since a literal placeholder there may well be what the API is being
    // tested with.
    let (_, mut url_missing) = resolver.interpolate(&http_spec.url);
    url_missing.sort();
    url_missing.dedup();
    if !url_missing.is_empty() {
        let names = url_missing
            .iter()
            .map(|name| format!("{{{{{name}}}}}"))
            .collect::<Vec<_>>()
            .join(", ");
        return Err(AppError::Message(format!(
            "В адресе запроса не подставлены переменные: {names}. Проверьте активное окружение."
        )));
    }

    let (resolved, unresolved_variables) = resolve_http_request(&http_spec, &resolver);

    let settings = state.request_settings.clone();
    // The recorder is owned here, not by the executor, so a failed attempt
    // still has a timeline. Handing it back on the error path needs a
    // structured IPC error and is a separate change; for now it is finished
    // and dropped, and only the message reaches the UI.
    let mut recorder = TraceRecorder::start();
    for name in &unresolved_variables {
        recorder.warn(format!("Не подставлена переменная {{{{{name}}}}}"));
    }
    let cancelled = in_flight.register(send_id.clone());
    let ctx = ExecutionContext { settings };
    let result = tokio::select! {
        result = executor.execute(&resolved, &ctx, &mut recorder) => Some(result),
        // Dropping the executor's future is the cancellation: reqwest tears
        // the connection down as it unwinds.
        _ = cancelled => None,
    };
    in_flight.forget(&send_id);
    let trace = recorder.finish();

    let Some(result) = result else {
        return Err(AppError::Message("Запрос отменён".to_string()));
    };
    let outcome = result.map_err(|e| AppError::Message(e.to_string()))?;

    Ok(SendResult {
        outcome,
        trace,
        unresolved_variables,
    })
}

/// Stops a send that is still running. Cancelling something that already
/// finished is a no-op, not an error: the click and the response can always
/// cross paths.
#[tauri::command]
pub fn cancel_send(in_flight: State<'_, InFlightSends>, send_id: String) -> AppResult<bool> {
    Ok(in_flight.cancel(&send_id))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cancelling_a_registered_send_signals_it_once() {
        let registry = InFlightSends::default();
        let mut receiver = registry.register("send-1".to_string());

        assert!(registry.cancel("send-1"));
        assert_eq!(receiver.try_recv(), Ok(()));
        // The entry is gone, so a second click does nothing.
        assert!(!registry.cancel("send-1"));
    }

    #[test]
    fn cancelling_an_unknown_or_finished_send_is_a_no_op() {
        let registry = InFlightSends::default();
        assert!(!registry.cancel("never-started"));

        let _ = registry.register("send-2".to_string());
        registry.forget("send-2");
        assert!(!registry.cancel("send-2"));
    }
}
