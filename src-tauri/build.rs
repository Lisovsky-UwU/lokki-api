use std::process::Command;

/// Facts that only exist while building: the toolchains involved and the
/// commit the binary came from. Captured here as compile-time env vars,
/// since nothing in the running app can look them up afterwards. Anything
/// unavailable (no git, no node on PATH) becomes an empty string — the
/// About dialog words the "unknown" case, this file doesn't.
fn main() {
    println!("cargo:rustc-env=LOKKI_BUILD_DATE={}", chrono::Utc::now().format("%Y-%m-%d %H:%M UTC"));
    println!("cargo:rustc-env=LOKKI_COMMIT={}", git_commit());
    println!("cargo:rustc-env=LOKKI_RUSTC_VERSION={}", rustc_version());
    println!("cargo:rustc-env=LOKKI_NODE_VERSION={}", node_version());

    // Printing any rerun-if-changed replaces cargo's default of watching the
    // whole package, so `src` has to be named explicitly. Without all three
    // the commit and the build date would freeze at whatever they were the
    // last time cargo happened to re-run this script.
    println!("cargo:rerun-if-changed=src");
    println!("cargo:rerun-if-changed=../.git/HEAD");
    println!("cargo:rerun-if-changed=../.git/index");

    tauri_build::build()
}

fn output_of(program: &str, args: &[&str]) -> Option<String> {
    let out = Command::new(program).args(args).output().ok()?;
    if !out.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// Short hash, suffixed with `-dirty` when the working tree carries changes
/// that aren't in it — a build from uncommitted code is not the commit it
/// names, and that is exactly what a bug report needs to say.
fn git_commit() -> String {
    let Some(hash) = output_of("git", &["rev-parse", "--short=12", "HEAD"]) else {
        return String::new();
    };
    match output_of("git", &["status", "--porcelain"]) {
        Some(status) if !status.is_empty() => format!("{hash}-dirty"),
        _ => hash,
    }
}

/// `rustc -V` prints "rustc 1.98.1 (48a229cea 2026-09-01)"; only the number
/// is worth showing.
fn rustc_version() -> String {
    let rustc = std::env::var("RUSTC").unwrap_or_else(|_| "rustc".to_string());
    output_of(&rustc, &["-V"])
        .and_then(|line| line.split_whitespace().nth(1).map(str::to_string))
        .unwrap_or_default()
}

/// The Node that builds the frontend — Tauri drives npm, so it is on PATH
/// during any real build of the app.
fn node_version() -> String {
    output_of("node", &["-v"])
        .map(|v| v.trim_start_matches('v').to_string())
        .unwrap_or_default()
}
