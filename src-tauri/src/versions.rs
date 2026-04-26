use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use sha1::{Sha1, Digest};
use tauri::Emitter;
use crate::ssh;

#[derive(Debug, Clone, Serialize)]
pub struct LogLine {
    pub raw: String,
    pub level: LogLevel,
    pub timestamp: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub enum LogLevel {
    Info,
    Warn,
    Error,
    Other,
}

const VERSION_MANIFEST_URL: &str =
    "https://launchermeta.mojang.com/mc/game/version_manifest.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McVersion {
    pub id: String,
    #[serde(rename = "type")]
    pub version_type: String,
    pub url: String,
    #[serde(rename = "releaseTime")]
    pub release_time: String,
}

#[derive(Debug, Deserialize)]
struct VersionManifest {
    versions: Vec<McVersion>,
}

#[derive(Debug, Deserialize)]
struct VersionMeta {
    downloads: Downloads,
}

#[derive(Debug, Deserialize)]
struct Downloads {
    server: ServerDownload,
}

#[derive(Debug, Deserialize)]
struct ServerDownload {
    url: String,
    sha1: String,
    size: u64,
}

pub async fn list_versions(include_snapshots: bool) -> Result<Vec<McVersion>> {
    let client = reqwest::Client::new();
    let data = client.get(VERSION_MANIFEST_URL)
        .send().await?
        .text().await?;

    let manifest: VersionManifest = serde_json::from_str(&data)?;
    let versions = manifest.versions.into_iter()
        .filter(|v| include_snapshots || v.version_type == "release")
        .collect();

    Ok(versions)
}

fn emit_log(app: &tauri::AppHandle, msg: &str, level: LogLevel) {
    let line = LogLine {
        raw: msg.to_string(),
        level,
        timestamp: None,
    };
    let _ = app.emit("install_log", &line);
}

/// Run an SSH command, emit each stdout line as Info and each stderr line as Warn.
/// Returns Err if the command exits non-zero.
async fn exec_emit(app: &tauri::AppHandle, profile_id: i64, cmd: &str) -> Result<()> {
    let (stdout, stderr, code) = ssh::exec(profile_id, cmd).await?;
    for line in stdout.lines().filter(|l| !l.trim().is_empty()) {
        emit_log(app, line, LogLevel::Info);
    }
    for line in stderr.lines().filter(|l| !l.trim().is_empty()) {
        emit_log(app, line, LogLevel::Warn);
    }
    if code != 0 {
        return Err(anyhow!("Command exited with code {}: {}", code, cmd));
    }
    Ok(())
}

pub async fn install(profile_id: i64, version_id: &str, server_dir: &str, app_handle: tauri::AppHandle) -> Result<String> {
    // Expand ~ in server_dir using the remote shell
    let server_dir = if server_dir.starts_with('~') {
        let (expanded, _, _) = ssh::exec(profile_id, &format!("eval echo {}", server_dir)).await
            .unwrap_or_else(|_| (server_dir.to_string(), String::new(), 0));
        expanded.trim().to_string()
    } else {
        server_dir.to_string()
    };
    let server_dir = server_dir.as_str();

    emit_log(&app_handle, &format!("Using server directory: {}", server_dir), LogLevel::Info);

    // Pre-check: make sure we can write to the parent directory
    let parent_dir = std::path::Path::new(server_dir)
        .parent()
        .and_then(|p| p.to_str())
        .unwrap_or("/tmp");
    let (_, test_stderr, test_code) = ssh::exec(profile_id,
        &format!("test -w '{}' || (mkdir -p '{}' 2>&1 && test -w '{}')", server_dir, server_dir, server_dir)
    ).await.unwrap_or_default();
    if test_code != 0 {
        let msg = format!(
            "Cannot write to '{}': {}. Please go to Settings and set a writable server directory like ~/minecraft",
            server_dir, test_stderr.trim()
        );
        emit_log(&app_handle, &msg, LogLevel::Error);
        return Err(anyhow!("{}", msg));
    }

    emit_log(&app_handle, "Fetching version manifest from Mojang...", LogLevel::Info);

    // Fetch version metadata
    let versions = match list_versions(true).await {
        Ok(v) => v,
        Err(e) => {
            emit_log(&app_handle, &format!("Error fetching manifest: {}", e), LogLevel::Error);
            return Err(e);
        }
    };
    let version = match versions.iter().find(|v| v.id == version_id) {
        Some(v) => v,
        None => {
            let e = anyhow!("Version {} not found", version_id);
            emit_log(&app_handle, &format!("Error: {}", e), LogLevel::Error);
            return Err(e);
        }
    };

    let client = reqwest::Client::new();
    let meta: VersionMeta = match client.get(&version.url).send().await?.json().await {
        Ok(m) => m,
        Err(e) => {
            emit_log(&app_handle, &format!("Error fetching version metadata: {}", e), LogLevel::Error);
            return Err(e.into());
        }
    };
    let server_dl = &meta.downloads.server;
    let download_url = server_dl.url.clone();
    let expected_sha1 = server_dl.sha1.clone();
    let size_mb = server_dl.size as f64 / 1_048_576.0;

    // Each version gets its own directory: minecraft-server-1-20-4
    let version_dir_name = format!("minecraft-server-{}", version_id.replace('.', "-"));
    let version_dir = format!("{}/{}", server_dir, version_dir_name);
    let jar_name = format!("minecraft_server_{}.jar", version_id);
    let remote_path = format!("{}/{}", version_dir, jar_name);
    let eula_path = format!("{}/eula.txt", version_dir);

    emit_log(&app_handle, &format!("Version {} — JAR size: {:.1} MB", version_id, size_mb), LogLevel::Info);
    emit_log(&app_handle, &format!("Server directory: {}", version_dir), LogLevel::Info);

    // Create version-specific directory
    emit_log(&app_handle, &format!("$ mkdir -p {}", version_dir), LogLevel::Other);
    if let Err(e) = exec_emit(&app_handle, profile_id, &format!("mkdir -p '{}'", version_dir)).await {
        emit_log(&app_handle, &format!("Error creating directory: {}", e), LogLevel::Error);
        return Err(e);
    }

    // Download JAR on the remote server with wget or curl
    let (_, _, wget_code) = ssh::exec(profile_id, "which wget").await.unwrap_or_default();
    let (_, _, curl_code) = ssh::exec(profile_id, "which curl").await.unwrap_or_default();

    if wget_code == 0 {
        let cmd = format!("wget -O '{}' '{}' 2>&1", remote_path, download_url);
        emit_log(&app_handle, &format!("$ {}", cmd), LogLevel::Other);
        if let Err(e) = exec_emit(&app_handle, profile_id, &cmd).await {
            emit_log(&app_handle, &format!("wget failed: {}", e), LogLevel::Error);
            return Err(e);
        }
    } else if curl_code == 0 {
        let cmd = format!("curl -L --progress-bar -o '{}' '{}' 2>&1", remote_path, download_url);
        emit_log(&app_handle, &format!("$ {}", cmd), LogLevel::Other);
        if let Err(e) = exec_emit(&app_handle, profile_id, &cmd).await {
            emit_log(&app_handle, &format!("curl failed: {}", e), LogLevel::Error);
            return Err(e);
        }
    } else {
        let e = anyhow!("Neither wget nor curl found on remote server. Install with: sudo apt install wget");
        emit_log(&app_handle, &format!("Error: {}", e), LogLevel::Error);
        return Err(e);
    }

    // Verify SHA1 on remote
    emit_log(&app_handle, "Verifying SHA1 checksum...", LogLevel::Info);
    let (actual_sha1, _, _) = ssh::exec(profile_id,
        &format!("sha1sum '{}' | awk '{{print $1}}'", remote_path)).await?;
    let actual_sha1 = actual_sha1.trim().to_string();
    emit_log(&app_handle, &format!("SHA1: {}", actual_sha1), LogLevel::Info);
    if actual_sha1 != expected_sha1 {
        let e = anyhow!("SHA1 mismatch! Expected: {}, Got: {}", expected_sha1, actual_sha1);
        emit_log(&app_handle, &format!("Error: {}", e), LogLevel::Error);
        let _ = ssh::exec(profile_id, &format!("rm -f '{}'", remote_path)).await;
        return Err(e);
    }
    emit_log(&app_handle, "SHA1 checksum verified ✓", LogLevel::Info);

    // chmod +x
    let _ = exec_emit(&app_handle, profile_id, &format!("chmod +x '{}'", remote_path)).await;

    // Accept EULA automatically
    emit_log(&app_handle, "Accepting EULA (creating eula.txt)...", LogLevel::Info);
    let eula_cmd = format!("printf 'eula=true\\n' > '{}'", eula_path);
    emit_log(&app_handle, &format!("$ {}", eula_cmd), LogLevel::Other);
    if let Err(e) = exec_emit(&app_handle, profile_id, &eula_cmd).await {
        emit_log(&app_handle, &format!("Warning: could not create eula.txt: {}", e), LogLevel::Warn);
    } else {
        emit_log(&app_handle, "eula.txt created ✓", LogLevel::Info);
    }

    // Final listing
    emit_log(&app_handle, &format!("$ ls -lh '{}'", version_dir), LogLevel::Other);
    let _ = exec_emit(&app_handle, profile_id, &format!("ls -lh '{}'", version_dir)).await;

    emit_log(&app_handle, &format!("Installation complete! Server ready at: {}", version_dir), LogLevel::Info);
    Ok(version_dir)
}

pub async fn upload_bytes_via_sftp_pub(profile_id: i64, data: &[u8], remote_path: &str) -> Result<()> {
    upload_bytes_via_sftp(profile_id, data, remote_path).await
}

async fn upload_bytes_via_sftp_emitting(profile_id: i64, data: &[u8], remote_path: &str, app: &tauri::AppHandle) -> Result<()> {
    let b64 = base64_encode(data);
    let chunk_size = 50_000;
    let chunks: Vec<&str> = b64.as_bytes().chunks(chunk_size)
        .map(|c| std::str::from_utf8(c).unwrap())
        .collect();
    let total = chunks.len();

    // Create/truncate file
    ssh::exec(profile_id, &format!("> '{}'", remote_path)).await?;

    for (i, chunk) in chunks.iter().enumerate() {
        if i % 10 == 0 || i == total - 1 {
            let pct = (i + 1) * 100 / total;
            emit_log(app, &format!("Uploading... {}/{} chunks ({}%)", i + 1, total, pct), LogLevel::Info);
        }
        let cmd = format!("echo '{}' | base64 -d >> '{}'", chunk, remote_path);
        let (stdout, stderr, code) = ssh::exec(profile_id, &cmd).await?;
        if !stdout.trim().is_empty() {
            emit_log(app, stdout.trim(), LogLevel::Info);
        }
        if !stderr.trim().is_empty() {
            emit_log(app, stderr.trim(), LogLevel::Warn);
        }
        if code != 0 {
            return Err(anyhow!("Upload chunk {} failed (exit {}): {}", i + 1, code, stderr));
        }
    }
    Ok(())
}

async fn upload_bytes_via_sftp(profile_id: i64, data: &[u8], remote_path: &str) -> Result<()> {
    // Encode as base64 and write via SSH pipeline
    use std::fmt::Write as FmtWrite;
    let b64 = base64_encode(data);
    let chunk_size = 50_000;
    let chunks: Vec<&str> = b64.as_bytes().chunks(chunk_size)
        .map(|c| std::str::from_utf8(c).unwrap())
        .collect();

    // Create/truncate file
    ssh::exec(profile_id, &format!("> '{}'", remote_path)).await?;

    for chunk in chunks {
        let cmd = format!("echo '{}' | base64 -d >> '{}'", chunk, remote_path);
        let (_, stderr, code) = ssh::exec(profile_id, &cmd).await?;
        if code != 0 {
            return Err(anyhow!("SFTP upload failed: {}", stderr));
        }
    }
    Ok(())
}

fn base64_encode(data: &[u8]) -> String {
    use std::fmt::Write;
    // Simple base64 using the standard alphabet
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity((data.len() + 2) / 3 * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(CHARS[((n >> 18) & 0x3f) as usize] as char);
        out.push(CHARS[((n >> 12) & 0x3f) as usize] as char);
        out.push(if chunk.len() > 1 { CHARS[((n >> 6) & 0x3f) as usize] as char } else { '=' });
        out.push(if chunk.len() > 2 { CHARS[(n & 0x3f) as usize] as char } else { '=' });
    }
    out
}
