use crate::versions;

#[tauri::command]
pub async fn list_mc_versions(include_snapshots: bool) -> Result<Vec<versions::McVersion>, String> {
    versions::list_versions(include_snapshots).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn install_server_version(app_handle: tauri::AppHandle, profile_id: i64, version_id: String) -> Result<String, String> {
    // Get server_dir from profile
    let server_dir = crate::db::with_conn(|conn| {
        let dir: rusqlite::Result<String> = conn.query_row(
            "SELECT server_dir FROM connection_profiles WHERE id = ?1",
            rusqlite::params![profile_id],
            |row| row.get(0),
        );
        dir.map_err(|e| anyhow::anyhow!("Failed to get server_dir: {}", e))
    }).map_err(|e| e.to_string())?;

    let jar_name = versions::install(profile_id, &version_id, &server_dir, app_handle)
        .await
        .map_err(|e| e.to_string())?;

    // Save installed version to db
    crate::db::with_conn(|conn| {
        conn.execute(
            "INSERT OR REPLACE INTO minecraft_versions (profile_id, version_id, jar_name, server_dir)
             VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![profile_id, version_id, jar_name, server_dir],
        )?;
        Ok(())
    }).map_err(|e: anyhow::Error| e.to_string())?;

    Ok(jar_name)
}

#[tauri::command]
pub fn list_installed_versions(profile_id: i64) -> Result<Vec<(String, String, String)>, String> {
    crate::db::with_conn(|conn| {
        let mut stmt = conn.prepare(
            "SELECT version_id, jar_name, server_dir FROM minecraft_versions WHERE profile_id = ?1"
        )?;
        let versions = stmt.query_map(rusqlite::params![profile_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?;
        versions.collect::<rusqlite::Result<Vec<_>>>().map_err(|e| e.into())
    }).map_err(|e| e.to_string())
}
