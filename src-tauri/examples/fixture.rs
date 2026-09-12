// Builds a small persistent fixture workspace at the given path (or a
// default scratch path) so its on-disk TOML files can be eyeballed for
// readability/git-diff-friendliness. Run: cargo run --example fixture -- <dir>
use lokki_api_lib::domain::{EnvironmentScope, HttpMethod};
use lokki_api_lib::store::{fs_collection, fs_environment, fs_request, fs_workspace};
use std::path::PathBuf;

fn main() {
    let dir: PathBuf = std::env::args().nth(1).map(PathBuf::from).expect("pass a target dir");
    std::fs::create_dir_all(&dir).unwrap();

    // Re-runnable against the same directory: only initialize it once.
    if !fs_workspace::is_workspace(&dir) {
        fs_workspace::create_workspace(&dir, "Fixture").unwrap();
    }
    let collection = fs_collection::create_collection(&dir, "Petstore").unwrap();
    let collection_path = std::path::Path::new(&collection.path);

    fs_environment::create_environment(&collection_path.join("environments"), "Dev", EnvironmentScope::Collection)
        .unwrap();

    let pets_dir = fs_request::create_folder(collection_path, "Pets").unwrap();
    let created = fs_request::create_request(&pets_dir, "List Pets", HttpMethod::Get).unwrap();
    let req_path = pets_dir.join("List Pets.lokki.toml");
    let mut req = fs_request::load_request(&req_path).unwrap();
    let http = req.http.as_mut().unwrap();
    http.url = "{{baseUrl}}/pets".to_string();
    http.headers.push(lokki_api_lib::domain::KeyValue {
        key: "Accept".to_string(),
        value: "application/json".to_string(),
        enabled: true,
    });
    http.auth = lokki_api_lib::domain::AuthSpec::Bearer { token: "{{authToken}}".to_string() };
    fs_request::save_request(&req_path, req).unwrap();
    assert_eq!(created.meta.seq, 1);

    println!("fixture written to {}", dir.display());
}
