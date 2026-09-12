use serde::Serialize;
use tauri::AppHandle;

/// What the About section shows. Build-time values are baked in by build.rs,
/// the rest is read from the running host; the frontend renders the list
/// as-is and never has to know which is which. Empty strings mean "couldn't
/// be determined" — the UI words that, so this layer stays language-neutral.
#[derive(Debug, Clone, Serialize)]
pub struct AppInfo {
    pub name: String,
    pub version: String,
    pub build_date: String,
    pub commit: String,
    pub os: String,
    pub arch: String,
    pub rust_version: String,
    pub tauri_version: String,
    pub node_version: String,
    pub webview_version: String,
}

#[tauri::command]
pub fn app_info(app: AppHandle) -> AppInfo {
    let package = app.package_info();
    AppInfo {
        // productName from tauri.conf.json ("LokkiAPI"), not the crate name.
        name: package.name.clone(),
        version: package.version.to_string(),
        build_date: env!("LOKKI_BUILD_DATE").to_string(),
        commit: env!("LOKKI_COMMIT").to_string(),
        os: std::env::consts::OS.to_string(),
        arch: std::env::consts::ARCH.to_string(),
        rust_version: env!("LOKKI_RUSTC_VERSION").to_string(),
        tauri_version: tauri::VERSION.to_string(),
        node_version: env!("LOKKI_NODE_VERSION").to_string(),
        // The system WebView is what actually renders the UI and differs per
        // machine, so it belongs in anything the user copies into a report.
        webview_version: tauri::webview_version().unwrap_or_default(),
    }
}

#[cfg(test)]
mod tests {
    /// Guards the build.rs wiring: a typo in an env var name only shows up
    /// as a blank line in the About dialog otherwise. Commit and node are
    /// left out on purpose — both are legitimately empty when building
    /// outside a git checkout or without node on PATH.
    #[test]
    fn build_metadata_is_captured() {
        assert!(!env!("LOKKI_BUILD_DATE").is_empty());
        assert!(!env!("LOKKI_RUSTC_VERSION").is_empty());
    }
}
