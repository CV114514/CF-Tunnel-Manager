use crate::error::{AppError, AppResult};
use crate::fs_paths;
use crate::state::CertInfo;
use chrono::{DateTime, Utc};
use std::path::PathBuf;
use std::time::Duration;
use tokio::process::Command;

pub fn check_cert() -> AppResult<CertInfo> {
    let path = fs_paths::cert_pem_path()
        .ok_or_else(|| AppError::Other("USERPROFILE not set".to_string()))?;
    if !path.exists() {
        return Ok(CertInfo {
            path: path.to_string_lossy().to_string(),
            logged_in: false,
            account_email: None,
            expires_at: None,
        });
    }
    let metadata = std::fs::metadata(&path).ok();
    let modified = metadata
        .and_then(|m| m.modified().ok())
        .and_then(|t| DateTime::from_timestamp(t.duration_since(std::time::UNIX_EPOCH).ok()?.as_secs() as i64, 0));
    Ok(CertInfo {
        path: path.to_string_lossy().to_string(),
        logged_in: true,
        account_email: None,
        expires_at: modified,
    })
}

pub async fn start_login_flow(exe: &PathBuf) -> AppResult<()> {
    let mut cmd = Command::new(exe);
    cmd.args(["tunnel", "login", "--no-autoupdate"]);
    cmd.stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .kill_on_drop(true);
    let _child = cmd
        .spawn()
        .map_err(|e| AppError::CommandFailed(format!("spawn login failed: {e}")))?;
    Ok(())
}

pub async fn wait_for_cert(timeout: Duration) -> AppResult<bool> {
    let cert_path = fs_paths::cert_pem_path()
        .ok_or_else(|| AppError::Other("USERPROFILE not set".to_string()))?;
    let start = std::time::Instant::now();
    while start.elapsed() < timeout {
        if cert_path.exists() {
            return Ok(true);
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    Ok(false)
}
