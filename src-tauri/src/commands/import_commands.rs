use crate::domain::{CollectionSummary, HttpMethod, KeyValue};
use crate::error::{AppError, AppResult};
use crate::exec::{ExecutionContext, HttpExecutor, ProtocolExecutor, ResolvedHttpRequest, TraceRecorder};
use crate::i18n::messages;
use crate::import::openapi;
use crate::store::{fs_app_state, fs_import};
use base64::Engine;
use serde::Serialize;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager, State};

/// What the import created, for the dialog to report. The warnings are the
/// part worth reading: each one is something the specification asked for
/// that the collection could not reproduce.
#[derive(Debug, Clone, Serialize)]
pub struct ImportResult {
    pub collection: CollectionSummary,
    pub requests: usize,
    pub folders: usize,
    pub environments: usize,
    pub warnings: Vec<String>,
}

fn app_local_data_dir(app: &AppHandle) -> AppResult<PathBuf> {
    app.path()
        .app_local_data_dir()
        .map_err(|_| AppError::NotFound("app local data dir".to_string()))
}

#[tauri::command]
pub fn import_openapi_file(app: AppHandle, workspace_path: String, file_path: String) -> AppResult<ImportResult> {
    let text = std::fs::read_to_string(&file_path)
        .map_err(|e| AppError::Message(messages::read_spec_failed(&file_path, &e.to_string())))?;
    import(&app, &workspace_path, &text, None)
}

#[tauri::command]
pub async fn import_openapi_url(
    app: AppHandle,
    executor: State<'_, HttpExecutor>,
    workspace_path: String,
    url: String,
) -> AppResult<ImportResult> {
    let text = download(&app, executor.inner(), &url).await?;
    // The link is passed on: a specification is allowed to give a relative
    // server address, and the host it means is the one that just served it.
    import(&app, &workspace_path, &text, Some(&url))
}

fn import(app: &AppHandle, workspace_path: &str, text: &str, origin: Option<&str>) -> AppResult<ImportResult> {
    let plan = openapi::parse_with_origin(text, origin)?;
    // Refused rather than left as an empty collection: a specification with
    // no operations in it is almost always the wrong file.
    if plan.request_count() == 0 {
        return Err(AppError::Message(messages::spec_has_no_operations()));
    }

    let written = fs_import::write_plan(Path::new(workspace_path), &plan)?;

    // The collection opens with its first environment already selected:
    // every imported request starts with `{{baseUrl}}`, and until something
    // resolves it none of them can be sent.
    if let Some(first) = written.environments.first() {
        let dir = app_local_data_dir(app)?;
        let mut state = fs_app_state::load(&dir);
        state
            .active_environments
            .insert(written.collection.path.clone(), first.clone());
        fs_app_state::save(&dir, &state)?;
    }

    Ok(ImportResult {
        collection: written.collection,
        requests: written.requests,
        folders: written.folders,
        environments: written.environments.len(),
        warnings: plan.warnings,
    })
}

/// Fetches a specification through the app's own HTTP executor rather than
/// a client of its own: the TLS setting, the timeouts and the translated
/// network failures are the ones the user already configured for sending.
async fn download(app: &AppHandle, executor: &HttpExecutor, url: &str) -> AppResult<String> {
    let settings = fs_app_state::load(&app_local_data_dir(app)?).request_settings;
    let request = ResolvedHttpRequest {
        method: HttpMethod::Get,
        url: url.to_string(),
        headers: vec![KeyValue {
            key: "Accept".to_string(),
            value: "application/json, application/yaml;q=0.9, */*;q=0.8".to_string(),
            enabled: true,
        }],
        body: None,
    };
    // Nothing shows the trace of a download, but the executor needs one to
    // report into.
    let mut recorder = TraceRecorder::start();
    let outcome = executor
        .execute(&request, &ExecutionContext { settings }, &mut recorder)
        .await
        .map_err(|e| AppError::Message(messages::download_spec_failed(url, &e.to_string())))?;

    if !(200..300).contains(&outcome.status) {
        return Err(AppError::Message(messages::download_spec_status(url, outcome.status)));
    }

    let bytes = base64::engine::general_purpose::STANDARD
        .decode(outcome.body_base64.as_bytes())
        .map_err(|e| AppError::Message(messages::decode_body_failed(&e.to_string())))?;
    String::from_utf8(bytes).map_err(|_| AppError::Message(messages::spec_not_text()))
}
