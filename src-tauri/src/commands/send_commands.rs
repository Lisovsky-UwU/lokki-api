use crate::domain::{BodySpec, EnvironmentFile, RequestFile, RequestSettings};
use crate::error::{AppError, AppResult};
use crate::exec::{
    resolve_http_request, ExecutionContext, ExecutionOutcome, ExecutionTrace, HttpExecutor, ProtocolExecutor,
    OutgoingMessage, ResolvedHttpRequest, SseExecutor, SseMessage, TraceRecorder, WsExecutor, WsMessage,
};
use crate::i18n::messages;
use crate::interpolate::{Resolver, VariableScope};
use crate::secrets::{local_file::LocalFileSecretStore, SecretStore};
use crate::store::{fs_app_state, fs_environment};
use serde::Serialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tokio::sync::{mpsc, oneshot};
use tauri::ipc::Channel;
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

    /// Returns false when there was nothing to cancel - the send had already
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

/// The way into every open WebSocket, keyed by the same `send_id` that
/// `InFlightSends` cancels it by. Holding the sender is what lets
/// `send_websocket_message` reach a connection that `open_websocket` is
/// still awaiting.
#[derive(Default)]
pub struct OpenSockets(Mutex<HashMap<String, mpsc::UnboundedSender<OutgoingMessage>>>);

impl OpenSockets {
    fn register(&self, send_id: String) -> mpsc::UnboundedReceiver<OutgoingMessage> {
        let (tx, rx) = mpsc::unbounded_channel();
        self.0.lock().expect("socket registry poisoned").insert(send_id, tx);
        rx
    }

    fn forget(&self, send_id: &str) {
        self.0.lock().expect("socket registry poisoned").remove(send_id);
    }

    /// Returns false when there is no such socket any more - it closed
    /// between the click and this call.
    fn send(&self, send_id: &str, message: OutgoingMessage) -> bool {
        let sockets = self.0.lock().expect("socket registry poisoned");
        sockets.get(send_id).is_some_and(|tx| tx.send(message).is_ok())
    }
}

/// `send_request`'s result: the HTTP outcome, where the time went, and any
/// `{{variable}}` names that couldn't be resolved (sent verbatim in the
/// request) - surfaced so the UI can warn the user rather than silently
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
/// on-disk `value` is blank - see domain::environment docs).
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

/// A request ready to go out: variables substituted, auth folded into
/// headers, the body read - plus what could not be substituted and the
/// settings to send it with. Shared by every command that sends something,
/// so a stream resolves its variables exactly the way a request does.
struct PreparedSend {
    resolved: ResolvedHttpRequest,
    unresolved_variables: Vec<String>,
    settings: RequestSettings,
}

/// The variables a send resolves against: the active global environment of
/// the workspace, and over it the active environment of the collection.
/// Either can be absent - see `send_request`.
fn build_resolver(
    app: &AppHandle,
    workspace_path: Option<&str>,
    collection_path: Option<&str>,
) -> AppResult<(Resolver, RequestSettings)> {
    let data_dir = app_local_data_dir(app)?;
    let state = fs_app_state::load(&data_dir);
    let secrets = LocalFileSecretStore;

    let workspace_dir = workspace_path.map(Path::new);
    let global_scope = match workspace_dir {
        Some(dir) => env_to_scope(active_environment_for(dir, &state.active_environments)?, dir, &secrets)?,
        None => VariableScope::default(),
    };

    let collection_scope = match (collection_path.map(Path::new), workspace_dir) {
        (Some(collection_dir), Some(workspace_dir)) => {
            match active_environment_for(collection_dir, &state.active_environments)? {
                Some(env) => Some(env_to_scope(Some(env), workspace_dir, &secrets)?),
                None => None,
            }
        }
        _ => None,
    };

    Ok((Resolver::new(global_scope, collection_scope), state.request_settings))
}

fn prepare_send(
    app: &AppHandle,
    request: RequestFile,
    workspace_path: Option<String>,
    collection_path: Option<String>,
) -> AppResult<PreparedSend> {
    let http_spec = request
        .http
        .ok_or_else(|| AppError::NotFound("request has no http spec".to_string()))?;

    let (resolver, settings) = build_resolver(app, workspace_path.as_deref(), collection_path.as_deref())?;

    // Checked before sending: a `{{var}}` left in the address produces a URL
    // that can't be parsed, and reqwest reports that as "relative URL without
    // a base" - true, and useless. Elsewhere (headers, body, auth) the
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
        return Err(AppError::Message(messages::unresolved_url_variables(&names)));
    }

    // Can fail before anything is sent: a file body that isn't readable is
    // the user's mistake to fix, not a transport failure.
    let (resolved, unresolved_variables) =
        resolve_http_request(&http_spec, &resolver).map_err(|e| AppError::Message(e.to_string()))?;

    Ok(PreparedSend {
        resolved,
        unresolved_variables,
        settings,
    })
}

/// A recorder that already carries the unresolved-variable warnings, so the
/// log starts with them whatever happens next.
fn start_trace(unresolved_variables: &[String]) -> TraceRecorder {
    let mut recorder = TraceRecorder::start();
    for name in unresolved_variables {
        recorder.warn(messages::unresolved_variable(name));
    }
    recorder
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
    // Absent for an incognito send started from the welcome screen: there is
    // no workspace, so no variables and no secrets at all.
    workspace_path: Option<String>,
    // Absent for any incognito send - a request that lives nowhere has no
    // collection environment to inherit.
    collection_path: Option<String>,
    // Made up by the frontend for this send; `cancel_send` refers to it.
    send_id: String,
) -> AppResult<SendResult> {
    let PreparedSend {
        resolved,
        unresolved_variables,
        settings,
    } = prepare_send(&app, request, workspace_path, collection_path)?;

    // The recorder is owned here, not by the executor, so a failed attempt
    // still has a timeline. Handing it back on the error path needs a
    // structured IPC error and is a separate change; for now it is finished
    // and dropped, and only the message reaches the UI.
    let mut recorder = start_trace(&unresolved_variables);
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
        return Err(AppError::Message(messages::request_cancelled()));
    };
    let outcome = result.map_err(|e| AppError::Message(e.to_string()))?;

    Ok(SendResult {
        outcome,
        trace,
        unresolved_variables,
    })
}

/// Opens a Server-Sent Events stream and forwards it to `on_message` until
/// it ends. Takes the same arguments as `send_request`, because the request
/// that opens a stream is an ordinary HTTP request.
///
/// Everything the UI learns comes through the channel, the closing
/// `SseMessage::End` included, and the command itself returns nothing. It
/// fails only when the stream never started - an unresolvable URL, a refused
/// connection, a cancel while still waiting for the server. From the
/// response head on, every ending, a disconnect included, arrives as `End`.
///
/// Stopping it is `cancel_send` with the same `send_id`, as for a request.
// `send_request`'s arguments plus the channel; a struct would only rename
// them, since Tauri takes command arguments one by one.
#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub async fn open_sse_stream(
    app: AppHandle,
    executor: State<'_, SseExecutor>,
    in_flight: State<'_, InFlightSends>,
    request: RequestFile,
    workspace_path: Option<String>,
    collection_path: Option<String>,
    send_id: String,
    on_message: Channel<SseMessage>,
) -> AppResult<()> {
    let PreparedSend {
        resolved,
        unresolved_variables,
        settings,
    } = prepare_send(&app, request, workspace_path, collection_path)?;

    let mut recorder = start_trace(&unresolved_variables);
    let cancelled = in_flight.register(send_id.clone());
    let ctx = ExecutionContext { settings };
    // A send fails only when the webview is gone (reloaded, closed): there is
    // nobody left to tell.
    let mut emit = |message: SseMessage| {
        let _ = on_message.send(message);
    };
    let result = executor
        .stream(
            &resolved,
            &ctx,
            &mut recorder,
            async {
                let _ = cancelled.await;
            },
            &mut emit,
        )
        .await;
    in_flight.forget(&send_id);
    let trace = recorder.finish();

    let outcome = result.map_err(|e| AppError::Message(e.to_string()))?;
    let _ = on_message.send(SseMessage::End {
        outcome,
        trace,
        unresolved_variables,
    });
    Ok(())
}

/// Opens a WebSocket and forwards everything that crosses it to
/// `on_message` until it closes. Takes the same arguments as `send_request`:
/// the handshake is an HTTP request, and it is resolved like one.
///
/// Like `open_sse_stream`, it reports everything through the channel, the
/// closing `WsMessage::End` included, and fails only when the socket never
/// opened. Messages go in through `send_websocket_message` and the socket is
/// closed with `cancel_send`, both by the same `send_id`.
#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub async fn open_websocket(
    app: AppHandle,
    executor: State<'_, WsExecutor>,
    in_flight: State<'_, InFlightSends>,
    sockets: State<'_, OpenSockets>,
    mut request: RequestFile,
    workspace_path: Option<String>,
    collection_path: Option<String>,
    send_id: String,
    on_message: Channel<WsMessage>,
) -> AppResult<()> {
    // A handshake is a GET without a body. A body left over from when this
    // was an HTTP request is not sent, so it must not be read either - a
    // file body that has since gone missing would fail the connection.
    if let Some(http) = request.http.as_mut() {
        http.body = BodySpec::None;
    }
    let PreparedSend {
        resolved,
        unresolved_variables,
        settings,
    } = prepare_send(&app, request, workspace_path, collection_path)?;

    let mut recorder = start_trace(&unresolved_variables);
    let cancelled = in_flight.register(send_id.clone());
    let mut outgoing = sockets.register(send_id.clone());
    let ctx = ExecutionContext { settings };
    let mut emit = |message: WsMessage| {
        let _ = on_message.send(message);
    };
    let result = executor
        .connect(
            &resolved,
            &ctx,
            &mut recorder,
            async {
                let _ = cancelled.await;
            },
            &mut outgoing,
            &mut emit,
        )
        .await;
    sockets.forget(&send_id);
    in_flight.forget(&send_id);
    let trace = recorder.finish();

    let outcome = result.map_err(|e| AppError::Message(e.to_string()))?;
    let _ = on_message.send(WsMessage::End {
        outcome,
        trace,
        unresolved_variables,
    });
    Ok(())
}

/// Sends a text message over the socket `send_id` opened. Its `{{variables}}`
/// are resolved now rather than when the socket opened, so an environment
/// edited in between applies to the next message. The message shows up in
/// the log through the socket's channel once it has actually gone out.
#[tauri::command]
pub fn send_websocket_message(
    app: AppHandle,
    sockets: State<'_, OpenSockets>,
    send_id: String,
    message: String,
    workspace_path: Option<String>,
    collection_path: Option<String>,
) -> AppResult<()> {
    let (resolver, _) = build_resolver(&app, workspace_path.as_deref(), collection_path.as_deref())?;
    let (text, mut unresolved_variables) = resolver.interpolate(&message);
    unresolved_variables.sort();
    unresolved_variables.dedup();
    let outgoing = OutgoingMessage {
        text,
        unresolved_variables,
    };
    if sockets.send(&send_id, outgoing) {
        Ok(())
    } else {
        Err(AppError::Message(messages::ws_not_connected()))
    }
}

/// Writes a response body to a file the user picked. The body crosses IPC
/// base64-encoded (see ExecutionOutcome), so it is decoded here: doing it in
/// the webview would hold the whole file in memory twice and can't write to
/// disk anyway.
#[tauri::command]
pub fn save_response_body(file_path: String, body_base64: String) -> AppResult<()> {
    use base64::Engine;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(body_base64.as_bytes())
        .map_err(|e| AppError::Message(messages::decode_body_failed(&e.to_string())))?;
    std::fs::write(&file_path, bytes).map_err(|source| AppError::Io {
        path: file_path,
        source,
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
    fn a_message_reaches_an_open_socket_and_not_a_closed_one() {
        let sockets = OpenSockets::default();
        let mut receiver = sockets.register("socket-1".to_string());
        let message = |text: &str| OutgoingMessage {
            text: text.to_string(),
            unresolved_variables: Vec::new(),
        };

        assert!(sockets.send("socket-1", message("hello")));
        assert_eq!(receiver.try_recv().unwrap().text, "hello");

        sockets.forget("socket-1");
        assert!(!sockets.send("socket-1", message("too late")));
        assert!(!sockets.send("never-opened", message("x")));
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
