use crate::ssh::{self, SshCredentials};

#[tauri::command]
pub async fn ssh_connect(
    profile_id: i64,
    host: String,
    port: u16,
    username: String,
    auth_method: String,
    password: Option<String>,
    key_path: Option<String>,
) -> Result<(), String> {
    let creds = SshCredentials {
        host,
        port,
        username,
        auth_method,
        password,
        key_path,
    };
    ssh::connect(profile_id, creds).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn ssh_disconnect(profile_id: i64) {
    ssh::disconnect(profile_id).await;
}

#[tauri::command]
pub async fn ssh_status(profile_id: i64) -> bool {
    ssh::is_connected(profile_id).await
}
