use std::path::PathBuf;
use tauri::{AppHandle, Manager};

pub fn app_data_dir(app: &AppHandle) -> PathBuf {
    let dir = app
        .path()
        .app_data_dir()
        .unwrap_or_else(|_| {
            dirs_fallback()
        });
    if !dir.exists() {
        let _ = std::fs::create_dir_all(&dir);
    }
    dir
}

pub fn tunnels_dir(app: &AppHandle) -> PathBuf {
    let dir = app_data_dir(app).join("tunnels");
    if !dir.exists() {
        let _ = std::fs::create_dir_all(&dir);
    }
    dir
}

pub fn state_file(app: &AppHandle) -> PathBuf {
    app_data_dir(app).join("state.json")
}

pub fn pid_file(app: &AppHandle) -> PathBuf {
    app_data_dir(app).join("pids.json")
}

pub fn log_dir(app: &AppHandle) -> PathBuf {
    let dir = app_data_dir(app).join("logs");
    if !dir.exists() {
        let _ = std::fs::create_dir_all(&dir);
    }
    dir
}

pub fn cloudflared_home() -> Option<PathBuf> {
    std::env::var_os("USERPROFILE")
        .map(|p| PathBuf::from(p).join(".cloudflared"))
}

pub fn cert_pem_path() -> Option<PathBuf> {
    cloudflared_home().map(|h| h.join("cert.pem"))
}

pub fn cloudflared_home_string() -> String {
    cloudflared_home()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default()
}

fn dirs_fallback() -> PathBuf {
    let base = std::env::var("APPDATA").unwrap_or_default();
    PathBuf::from(base).join("com.cf-tunnel-manager")
}
