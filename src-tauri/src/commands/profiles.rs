use serde::{Deserialize, Serialize};
use crate::db;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionProfileRow {
    pub id: i64,
    pub name: String,
    pub host: String,
    pub port: i64,
    pub username: String,
    pub auth_method: String,
    pub key_path: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileUpdate {
    pub name: String,
    pub host: String,
    pub port: i64,
    pub username: String,
    pub auth_method: String,
    pub password: Option<String>,
    pub key_path: Option<String>,
}

#[tauri::command]
pub fn list_profiles() -> Result<Vec<ConnectionProfileRow>, String> {
    db::with_conn(|conn| {
        let mut stmt = conn.prepare(
            "SELECT id, name, host, port, username, auth_method, key_path FROM connection_profiles ORDER BY id"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(ConnectionProfileRow {
                id: row.get(0)?,
                name: row.get(1)?,
                host: row.get(2)?,
                port: row.get(3)?,
                username: row.get(4)?,
                auth_method: row.get(5)?,
                key_path: row.get(6)?,
            })
        })?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    })
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_profile(profile: ProfileUpdate) -> Result<i64, String> {
    db::with_conn(|conn| {
        conn.execute(
            "INSERT INTO connection_profiles (name, host, port, username, auth_method, password, key_path)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![profile.name, profile.host, profile.port, profile.username, profile.auth_method, profile.password, profile.key_path],
        )?;
        Ok(conn.last_insert_rowid())
    })
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_profile(id: i64, profile: ProfileUpdate) -> Result<(), String> {
    db::with_conn(|conn| {
        conn.execute(
            "UPDATE connection_profiles SET name=?1, host=?2, port=?3, username=?4,
             auth_method=?5, password=?6, key_path=?7 WHERE id=?8",
            rusqlite::params![profile.name, profile.host, profile.port, profile.username, profile.auth_method, profile.password, profile.key_path, id],
        )?;
        Ok(())
    })
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_profile(id: i64) -> Result<(), String> {
    db::with_conn(|conn| {
        conn.execute("DELETE FROM connection_profiles WHERE id=?1", rusqlite::params![id])?;
        Ok(())
    })
    .map_err(|e| e.to_string())
}
