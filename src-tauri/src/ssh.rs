use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::Mutex;
use once_cell::sync::Lazy;

use russh::{client, ChannelMsg};
use russh_keys::key::PublicKey;
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SshCredentials {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub auth_method: String,
    pub password: Option<String>,
    pub key_path: Option<String>,
}

pub(crate) struct ClientHandler;

#[async_trait::async_trait]
impl client::Handler for ClientHandler {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        _server_public_key: &PublicKey,
    ) -> Result<bool, Self::Error> {
        // Accept any host key (TOFU); production should verify stored fingerprints
        Ok(true)
    }
}

pub struct SshSession {
    pub handle: client::Handle<ClientHandler>,
}

static SESSIONS: Lazy<Mutex<HashMap<i64, SshSession>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

pub async fn connect(profile_id: i64, creds: SshCredentials) -> Result<()> {
    let config = Arc::new(client::Config::default());
    let addr = format!("{}:{}", creds.host, creds.port);

    // Apply a 10-second connection timeout
    let mut session = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        client::connect(config, addr, ClientHandler),
    )
    .await
    .map_err(|_| anyhow!("Connection timed out after 10 seconds"))?
    .map_err(|e| anyhow!("SSH connect failed: {e}"))?;

    let authenticated = match creds.auth_method.as_str() {
        "password" => {
            let pw = creds.password.ok_or_else(|| anyhow!("No password provided"))?;
            session.authenticate_password(&creds.username, pw).await?
        }
        "key" => {
            let key_path = creds.key_path.ok_or_else(|| anyhow!("No key path provided"))?;
            let key = russh_keys::load_secret_key(&key_path, None)
                .map_err(|e| anyhow!("Failed to load key: {e}"))?;
            session.authenticate_publickey(&creds.username, Arc::new(key)).await?
        }
        other => return Err(anyhow!("Unknown auth method: {other}")),
    };

    if !authenticated {
        return Err(anyhow!("Authentication failed — check credentials"));
    }

    SESSIONS.lock().await.insert(profile_id, SshSession { handle: session });
    Ok(())
}

pub async fn disconnect(profile_id: i64) {
    SESSIONS.lock().await.remove(&profile_id);
}

pub async fn is_connected(profile_id: i64) -> bool {
    SESSIONS.lock().await.contains_key(&profile_id)
}

/// Execute a command on the remote VM, returning (stdout, stderr, exit_code).
pub async fn exec(profile_id: i64, command: &str) -> Result<(String, String, u32)> {
    let mut sessions = SESSIONS.lock().await;
    let session = sessions
        .get_mut(&profile_id)
        .ok_or_else(|| anyhow!("Not connected to profile {profile_id}"))?;

    let mut channel = session.handle.channel_open_session().await?;
    channel.exec(true, command).await?;

    let mut stdout: Vec<u8> = Vec::new();
    let mut stderr: Vec<u8> = Vec::new();
    let mut exit_code: u32 = 0;

    loop {
        match channel.wait().await {
            Some(ChannelMsg::Data { data }) => stdout.extend_from_slice(&data),
            Some(ChannelMsg::ExtendedData { data, .. }) => stderr.extend_from_slice(&data),
            Some(ChannelMsg::ExitStatus { exit_status }) => { exit_code = exit_status; }
            Some(ChannelMsg::Eof) | None => break,
            _ => {}
        }
    }

    Ok((
        String::from_utf8_lossy(&stdout).to_string(),
        String::from_utf8_lossy(&stderr).to_string(),
        exit_code,
    ))
}

/// Send input to a tmux session pane.
pub async fn tmux_send_keys(profile_id: i64, session_name: &str, keys: &str) -> Result<()> {
    let escaped = keys.replace('\'', "'\\''");
    let cmd = format!("tmux send-keys -t {} '{}' Enter", session_name, escaped);
    exec(profile_id, &cmd).await?;
    Ok(())
}
