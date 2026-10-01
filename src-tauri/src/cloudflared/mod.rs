pub mod config;
pub mod login;
pub mod metrics;
pub mod run;
pub mod tunnel;

use crate::error::{AppError, AppResult};
use crate::state::VersionInfo;
use regex::Regex;
use std::path::PathBuf;
use std::process::Stdio;
use tokio::process::Command;

pub async fn run(exe: &PathBuf, args: &[&str]) -> AppResult<std::process::Output> {
    let output = Command::new(exe)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .map_err(|e| AppError::CommandFailed(format!("spawn failed: {e}")))?;
    Ok(output)
}

pub async fn run_capture(exe: &PathBuf, args: &[&str]) -> AppResult<(String, String, i32)> {
    let output = run(exe, args).await?;
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    Ok((stdout, stderr, code))
}

pub async fn probe_version(exe: &PathBuf) -> AppResult<VersionInfo> {
    let (stdout, _stderr, code) = run_capture(exe, &["version"]).await?;
    if code != 0 {
        return Err(AppError::CommandFailed(format!("exit code {code}")));
    }
    let re = Regex::new(r"cloudflared version ([\d.]+)")?;
    let version = re
        .captures(&stdout)
        .and_then(|c| c.get(1).map(|m| m.as_str().to_string()))
        .unwrap_or_default();
    if version.is_empty() || !stdout.contains("cloudflared") {
        return Err(AppError::CommandFailed(
            "not a cloudflared binary (output missing 'cloudflared' marker)".to_string(),
        ));
    }
    Ok(VersionInfo {
        version,
        raw: stdout.trim().to_string(),
    })
}
