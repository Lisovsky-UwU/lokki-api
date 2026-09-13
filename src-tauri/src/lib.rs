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
        // One HTTP client for the whole app: reqwest pools connections and
        // builds its TLS root store once, instead of per request.
        .manage(exec::HttpExecutor::new())
        .invoke_handler(tauri::generate_handler![
            commands::app_commands::app_info,
            commands::workspace_commands::open_workspace,
            commands::workspace_commands::create_workspace,
            commands::workspace_commands::rename_workspace,
            commands::workspace_commands::get_last_workspace,
            commands::workspace_commands::list_collections,
            commands::collection_commands::create_collection,
            commands::collection_commands::load_collection_tree,
            commands::collection_commands::rename_collection,
            commands::request_commands::load_request,
            commands::request_commands::save_request,
            commands::request_commands::create_request,
            commands::request_commands::delete_request,
            commands::request_commands::rename_request,
            commands::request_commands::create_folder,
            commands::request_commands::delete_folder,
            commands::request_commands::rename_folder,
            commands::request_commands::move_node,
            commands::request_commands::reorder_children,
            commands::environment_commands::list_environments,
            commands::environment_commands::create_environment,
            commands::environment_commands::save_environment,
            commands::environment_commands::delete_environment,
            commands::environment_commands::set_active_environment,
            commands::environment_commands::get_active_environment,
            commands::send_commands::send_request,
            commands::settings_commands::get_request_settings,
            commands::settings_commands::save_request_settings,
            commands::secret_commands::set_secret,
            commands::secret_commands::reveal_secret,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
