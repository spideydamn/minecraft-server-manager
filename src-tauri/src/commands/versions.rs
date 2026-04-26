use crate::versions;
use tauri::Emitter;

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

    let jar_name = versions::install(profile_id, &version_id, &server_dir, app_handle.clone())
        .await
        .map_err(|e| e.to_string())?;

    // Calculate version directory path
    let version_dir_name = format!("minecraft-server-{}", version_id.replace('.', "-"));
    let version_dir = format!("{}/{}", server_dir, version_dir_name);

    // Save installed version to db with installation date
    crate::db::with_conn(|conn| {
        conn.execute(
            "INSERT OR REPLACE INTO minecraft_versions (profile_id, version_id, jar_name, server_dir, in_use, installation_date)
             VALUES (?1, ?2, ?3, ?4, 0, datetime('now'))",
            rusqlite::params![profile_id, version_id, jar_name, version_dir],
        )?;
        Ok(())
    }).map_err(|e: anyhow::Error| e.to_string())?;

    // Emit version-changed event
    let _ = app_handle.emit("version_changed", &versions::VersionChangedEvent {
        version_id: version_id.clone(),
        change_type: "installed".to_string(),
    });

    Ok(jar_name)
}

#[tauri::command]
pub fn list_installed_versions(profile_id: i64) -> Result<Vec<versions::InstalledVersion>, String> {
    crate::db::with_conn(|conn| {
        let mut stmt = conn.prepare(
            "SELECT version_id, jar_name, server_dir, in_use, installation_date
             FROM minecraft_versions
             WHERE profile_id = ?1
               AND jar_name IS NOT NULL
               AND jar_name != ''
               AND installation_date IS NOT NULL
               AND installation_date != ''"
        )?;
        let versions = stmt.query_map(rusqlite::params![profile_id], |row| {
            let version_id: String = row.get(0)?;
            let jar_name: String = row.get(1)?;
            let server_dir: String = row.get(2)?;
            let in_use: bool = row.get::<_, i64>(3)? == 1;
            let installation_date: String = row.get(4)?;
            Ok(versions::InstalledVersion {
                version_id,
                jar_name,
                server_dir,
                in_use,
                installation_date,
            })
        })?;
        versions.collect::<rusqlite::Result<Vec<_>>>().map_err(|e| e.into())
    }).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_version(app_handle: tauri::AppHandle, profile_id: i64, version_id: String) -> Result<String, String> {
    // Check if version is in use
    let in_use = crate::db::is_version_in_use(profile_id, &version_id)
        .map_err(|e| e.to_string())?;

    if in_use {
        let servers = crate::db::get_servers_using_version(profile_id, &version_id)
            .map_err(|e| e.to_string())?;
        return Err(format!("Cannot delete version {} - it is in use by: {}", version_id, servers.join(", ")));
    }

    // Get version directory from database
    let version_dir = crate::db::with_conn(|conn| {
        let dir: rusqlite::Result<String> = conn.query_row(
            "SELECT server_dir FROM minecraft_versions WHERE profile_id = ?1 AND version_id = ?2",
            rusqlite::params![profile_id, version_id],
            |row| row.get(0),
        );
        dir.map_err(|e| anyhow::anyhow!("Failed to get version_dir: {}", e))
    }).map_err(|e| e.to_string())?;

    // Delete version from remote server
    versions::delete_version(profile_id, &version_id, &version_dir, app_handle.clone())
        .await
        .map_err(|e| e.to_string())?;

    // Remove from database
    crate::db::with_conn(|conn| {
        conn.execute(
            "DELETE FROM minecraft_versions WHERE profile_id = ?1 AND version_id = ?2",
            rusqlite::params![profile_id, version_id],
        )?;
        Ok(())
    }).map_err(|e: anyhow::Error| e.to_string())?;

    // Emit version-changed event
    let _ = app_handle.emit("version_changed", &versions::VersionChangedEvent {
        version_id: version_id.clone(),
        change_type: "deleted".to_string(),
    });

    Ok(format!("Version {} deleted successfully", version_id))
}

#[tauri::command]
pub async fn reinstall_version(app_handle: tauri::AppHandle, profile_id: i64, version_id: String) -> Result<String, String> {
    // Check if version exists
    let exists = crate::db::with_conn(|conn| {
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM minecraft_versions WHERE profile_id = ?1 AND version_id = ?2",
            rusqlite::params![profile_id, version_id],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }).map_err(|e| e.to_string())?;

    if !exists {
        return Err(format!("Version {} not found", version_id));
    }

    // Check if version is in use
    let in_use = crate::db::is_version_in_use(profile_id, &version_id)
        .map_err(|e| e.to_string())?;

    if in_use {
        let servers = crate::db::get_servers_using_version(profile_id, &version_id)
            .map_err(|e| e.to_string())?;
        // Block reinstall when version is in use to prevent affecting running servers
        return Err(format!("Warning: Version {} is in use by: {}. Reinstalling may affect running servers.", version_id, servers.join(", ")));
    }

    // Get version directory from database
    let version_dir = crate::db::with_conn(|conn| {
        let dir: rusqlite::Result<String> = conn.query_row(
            "SELECT server_dir FROM minecraft_versions WHERE profile_id = ?1 AND version_id = ?2",
            rusqlite::params![profile_id, version_id],
            |row| row.get(0),
        );
        dir.map_err(|e| anyhow::anyhow!("Failed to get version_dir: {}", e))
    }).map_err(|e| e.to_string())?;

    // Reinstall version
    versions::reinstall_version(profile_id, &version_id, &version_dir, app_handle.clone())
        .await
        .map_err(|e| e.to_string())?;

    // Update installation date in database
    crate::db::with_conn(|conn| {
        conn.execute(
            "UPDATE minecraft_versions SET installation_date = datetime('now') WHERE profile_id = ?1 AND version_id = ?2",
            rusqlite::params![profile_id, version_id],
        )?;
        Ok(())
    }).map_err(|e: anyhow::Error| e.to_string())?;

    // Emit version-changed event
    let _ = app_handle.emit("version_changed", &versions::VersionChangedEvent {
        version_id: version_id.clone(),
        change_type: "reinstalled".to_string(),
    });

    Ok(format!("Version {} reinstalled successfully", version_id))
}
