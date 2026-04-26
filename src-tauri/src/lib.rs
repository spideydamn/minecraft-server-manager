#![allow(unused)]

mod db;
mod ssh;
mod versions;
mod settings;
mod commands;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            let app_data_dir = app.path().app_data_dir()
                .expect("failed to resolve app data dir");
            std::fs::create_dir_all(&app_data_dir)
                .expect("failed to create app data dir");
            let db_path = app_data_dir.join("msm.db");
            db::init(db_path.to_str().unwrap())
                .expect("failed to initialize database");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // SSH / connection profile commands
            commands::ssh::ssh_connect,
            commands::ssh::ssh_disconnect,
            commands::ssh::ssh_status,
            commands::profiles::list_profiles,
            commands::profiles::create_profile,
            commands::profiles::update_profile,
            commands::profiles::delete_profile,
            // Version management commands
            commands::versions::list_mc_versions,
            commands::versions::install_server_version,
            commands::versions::list_installed_versions,
            commands::versions::delete_version,
            commands::versions::reinstall_version,
            // Settings commands
            commands::settings::load_server_settings,
            commands::settings::save_server_settings,
            commands::settings::generate_default_settings,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
