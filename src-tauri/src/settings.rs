use anyhow::{anyhow, Result};
use std::collections::HashMap;
use crate::ssh;

pub fn load_server_properties_str(content: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(idx) = line.find('=') {
            let key = line[..idx].trim().to_string();
            let val = line[idx + 1..].trim().to_string();
            map.insert(key, val);
        }
    }
    map
}

pub fn serialize_server_properties(props: &HashMap<String, String>) -> String {
    let mut lines: Vec<String> = props.iter()
        .map(|(k, v)| format!("{}={}", k, v))
        .collect();
    lines.sort();
    lines.join("\n") + "\n"
}

pub async fn read_server_properties(profile_id: i64, server_dir: &str) -> Result<HashMap<String, String>> {
    let path = format!("{}/server.properties", server_dir);
    let (out, stderr, code) = ssh::exec(profile_id, &format!("cat '{}'", path)).await?;
    if code != 0 {
        if stderr.contains("No such file") || out.is_empty() {
            return Err(anyhow!("server.properties not found"));
        }
        return Err(anyhow!("Failed to read server.properties: {}", stderr));
    }
    Ok(load_server_properties_str(&out))
}

pub async fn write_server_properties(
    profile_id: i64,
    server_dir: &str,
    props: &HashMap<String, String>,
) -> Result<()> {
    let content = serialize_server_properties(props);
    let path = format!("{}/server.properties", server_dir);
    // Write via heredoc to avoid shell quoting issues with special chars
    let escaped = content.replace('\\', "\\\\").replace('\'', "'\\''");
    let cmd = format!("printf '%s' '{}' > '{}'", escaped, path);
    let (_, stderr, code) = ssh::exec(profile_id, &cmd).await?;
    if code != 0 {
        return Err(anyhow!("Failed to write server.properties: {}", stderr));
    }
    Ok(())
}

pub async fn generate_default_properties(profile_id: i64, server_dir: &str) -> Result<()> {
    let defaults = HashMap::from([
        ("server-port".to_string(), "25565".to_string()),
        ("max-players".to_string(), "20".to_string()),
        ("online-mode".to_string(), "true".to_string()),
        ("pvp".to_string(), "true".to_string()),
        ("difficulty".to_string(), "normal".to_string()),
        ("gamemode".to_string(), "survival".to_string()),
        ("level-name".to_string(), "world".to_string()),
        ("motd".to_string(), "A Minecraft Server".to_string()),
        ("view-distance".to_string(), "10".to_string()),
        ("spawn-protection".to_string(), "16".to_string()),
    ]);
    write_server_properties(profile_id, server_dir, &defaults).await
}
