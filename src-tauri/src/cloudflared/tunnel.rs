use crate::error::{AppError, AppResult};
use crate::state::RemoteTunnel;
use regex::Regex;
use std::path::PathBuf;
use std::process::Stdio;
use tokio::process::Command;

pub async fn create_tunnel(exe: &PathBuf, name: &str) -> AppResult<(String, String)> {
    let (stdout, stderr, code) = crate::cloudflared::run_capture(exe, &["tunnel", "--no-autoupdate", "create", name]).await?;
    if code != 0 {
        let combined = format!("{}\n{}", stdout, stderr);
        return Err(AppError::CommandFailed(format!(
            "tunnel create failed (exit {code}): {combined}"
        )));
    }
    let re = Regex::new(r"([0-9a-fA-F-]{36})\.json")?;
    let uuid = re
        .captures(&stdout)
        .and_then(|c| c.get(1).map(|m| m.as_str().to_string()))
        .ok_or_else(|| {
            AppError::CommandFailed(format!(
                "could not parse tunnel UUID from output: {stdout}"
            ))
        })?;
    let home = crate::fs_paths::cloudflared_home()
        .ok_or_else(|| AppError::Other("USERPROFILE not set".to_string()))?;
    let creds_path = home.join(format!("{uuid}.json"));
    Ok((uuid, creds_path.to_string_lossy().to_string()))
}

pub async fn route_dns(exe: &PathBuf, name: &str, hostname: &str) -> AppResult<()> {
    let (stdout, stderr, code) = crate::cloudflared::run_capture(
        exe,
        &["tunnel", "--no-autoupdate", "route", "dns", name, hostname],
    )
    .await?;
    if code == 0 {
        return Ok(());
    }
    let combined = format!("{stdout}\n{stderr}").to_lowercase();
    if combined.contains("already exists") || combined.contains("record already") {
        return Ok(());
    }
    Err(AppError::CommandFailed(format!(
        "route dns failed (exit {code}): {stdout}\n{stderr}"
    )))
}

pub async fn delete_tunnel(exe: &PathBuf, name: &str) -> AppResult<()> {
    let (stdout, stderr, code) = crate::cloudflared::run_capture(exe, &["tunnel", "--no-autoupdate", "delete", name]).await?;
    if code != 0 {
        let combined = format!("{stdout}\n{stderr}");
        return Err(AppError::CommandFailed(format!(
            "tunnel delete failed (exit {code}): {combined}"
        )));
    }
    Ok(())
}

pub async fn list_tunnels(exe: &PathBuf) -> AppResult<Vec<RemoteTunnel>> {
    let (stdout, stderr, code) = crate::cloudflared::run_capture(exe, &["tunnel", "--no-autoupdate", "list", "--output", "json"]).await?;
    if code != 0 {
        let combined = format!("{stdout}\n{stderr}");
        if combined.to_lowercase().contains("no tunnels") || stdout.trim().is_empty() {
            return Ok(Vec::new());
        }
        return Err(AppError::CommandFailed(format!(
            "tunnel list failed (exit {code}): {combined}"
        )));
    }
    let trimmed = stdout.trim();
    if trimmed.is_empty() || trimmed == "null" {
        return Ok(Vec::new());
    }
    let parsed: Vec<serde_json::Value> = match serde_json::from_str(trimmed) {
        Ok(v) => v,
        Err(_) => return Ok(Vec::new()),
    };
    let result = parsed
        .into_iter()
        .map(|v| RemoteTunnel {
            id: v.get("id").and_then(|x| x.as_str()).unwrap_or("").to_string(),
            name: v
                .get("name")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string(),
            created_at: v
                .get("createdAt")
                .and_then(|x| x.as_str())
                .map(|s| s.to_string()),
            connections: v
                .get("connections")
                .and_then(|x| x.as_array())
                .map(|a| a.len() as u32),
        })
        .collect();
    Ok(result)
}

#[allow(dead_code)]
pub async fn _spawn_run(exe: &PathBuf, config_path: &str) -> AppResult<tokio::process::Child> {
    let mut cmd = Command::new(exe);
    cmd.args([
        "tunnel",
        "--no-autoupdate",
        "--config",
        config_path,
        "--metrics",
        "localhost:0",
        "--loglevel",
        "info",
        "run",
    ]);
    cmd.stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let child = cmd
        .spawn()
        .map_err(|e| AppError::CommandFailed(format!("spawn tunnel run failed: {e}")))?;
    Ok(child)
}
