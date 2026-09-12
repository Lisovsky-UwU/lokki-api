// Manual end-to-end smoke test of the non-Tauri-specific pipeline (store +
// interpolate + exec), exercising the same functions the IPC commands call.
// Hits the real network (httpbin.org) — not part of `cargo test`, run
// manually via `cargo run --example smoke`.
use lokki_api_lib::domain::{EnvironmentScope, HttpMethod};
use lokki_api_lib::exec::{resolve_http_request, ExecutionContext, HttpExecutor, ProtocolExecutor};
use lokki_api_lib::interpolate::{Resolver, VariableScope};
use lokki_api_lib::store::{fs_collection, fs_environment, fs_request, fs_workspace};
use std::collections::HashMap;

#[tokio::main]
async fn main() {
    let dir = tempfile::tempdir().unwrap();
    println!("workspace dir: {}", dir.path().display());

    fs_workspace::create_workspace(dir.path(), "Smoke").unwrap();
    let (workspace, collections) = fs_workspace::open_workspace(dir.path()).unwrap();
    println!("opened workspace '{}', {} collections", workspace.name, collections.len());

    let collection = fs_collection::create_collection(dir.path(), "Demo").unwrap();
    println!("created collection at {}", collection.path);

    let (_env_path, env) = fs_environment::create_environment(
        &std::path::Path::new(&collection.path).join("environments"),
        "Dev",
        EnvironmentScope::Collection,
    )
    .unwrap();
    println!("created environment '{}'", env.meta.name);

    let created = fs_request::create_request(std::path::Path::new(&collection.path), "Get Ip", HttpMethod::Get)
        .unwrap();
    let request_path = std::path::Path::new(&collection.path).join("Get Ip.lokki.toml");

    let mut request = fs_request::load_request(&request_path).unwrap();
    request.http.as_mut().unwrap().url = "https://httpbin.org/get".to_string();
    let saved = fs_request::save_request(&request_path, request).unwrap();
    println!("saved request '{}' (version {})", saved.meta.name, saved.meta.sync.version);
    assert_eq!(created.meta.sync.version, 1);

    let tree = fs_collection::load_collection_tree(std::path::Path::new(&collection.path)).unwrap();
    println!("tree: {:?}", tree);

    let resolver = Resolver::new(VariableScope(HashMap::new()), None);
    let (resolved, unresolved) = resolve_http_request(saved.http.as_ref().unwrap(), &resolver);
    println!("unresolved vars: {:?}", unresolved);

    let executor = HttpExecutor::new();
    let outcome = executor
        .execute(&resolved, &ExecutionContext::default())
        .await
        .expect("http request should succeed");

    println!("status: {} {}", outcome.status, outcome.status_text);
    println!("duration: {} ms", outcome.duration_ms);
    let body = String::from_utf8_lossy(
        &base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &outcome.body_base64).unwrap(),
    )
    .to_string();
    println!("body (first 300 chars): {}", &body[..body.len().min(300)]);

    assert_eq!(outcome.status, 200);
    println!("\nSMOKE TEST PASSED");
}
