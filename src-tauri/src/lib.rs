pub mod commands;
pub mod domain;
pub mod error;
pub mod exec;
pub mod interpolate;
pub mod secrets;
pub mod store;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            commands::workspace_commands::open_workspace,
            commands::workspace_commands::list_collections,
            commands::collection_commands::create_collection,
            commands::collection_commands::load_collection_tree,
            commands::request_commands::load_request,
            commands::request_commands::save_request,
            commands::request_commands::create_request,
            commands::request_commands::delete_request,
            commands::request_commands::create_folder,
            commands::environment_commands::list_environments,
            commands::environment_commands::create_environment,
            commands::environment_commands::save_environment,
            commands::environment_commands::set_active_environment,
            commands::environment_commands::get_active_environment,
            commands::send_commands::send_request,
            commands::secret_commands::set_secret,
            commands::secret_commands::reveal_secret,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
