use std::collections::HashMap;
use crate::settings;

#[tauri::command]
pub async fn load_server_settings(profile_id: i64) -> Result<HashMap<String, String>, String> {
    // Get server_dir from profile
    let server_dir = crate::db::with_conn(|conn| {
        let dir: rusqlite::Result<String> = conn.query_row(
            "SELECT server_dir FROM connection_profiles WHERE id = ?1",
            rusqlite::params![profile_id],
            |row| row.get(0),
        );
        dir.map_err(|e| anyhow::anyhow!("Failed to get server_dir: {}", e))
    }).map_err(|e| e.to_string())?;

    settings::read_server_properties(profile_id, &server_dir)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn save_server_settings(
    profile_id: i64,
    properties: HashMap<String, String>,
) -> Result<(), String> {
    // Get server_dir from profile
    let server_dir = crate::db::with_conn(|conn| {
        let dir: rusqlite::Result<String> = conn.query_row(
            "SELECT server_dir FROM connection_profiles WHERE id = ?1",
            rusqlite::params![profile_id],
            |row| row.get(0),
        );
        dir.map_err(|e| anyhow::anyhow!("Failed to get server_dir: {}", e))
    }).map_err(|e| e.to_string())?;

    settings::write_server_properties(profile_id, &server_dir, &properties)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn generate_default_settings(profile_id: i64) -> Result<(), String> {
    // Get server_dir from profile
    let server_dir = crate::db::with_conn(|conn| {
        let dir: rusqlite::Result<String> = conn.query_row(
            "SELECT server_dir FROM connection_profiles WHERE id = ?1",
            rusqlite::params![profile_id],
            |row| row.get(0),
        );
        dir.map_err(|e| anyhow::anyhow!("Failed to get server_dir: {}", e))
    }).map_err(|e| e.to_string())?;

    settings::generate_default_properties(profile_id, &server_dir)
        .await
        .map_err(|e| e.to_string())
}
