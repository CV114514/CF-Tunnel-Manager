use crate::error::AppResult;
use crate::fs_paths;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::AppHandle;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PidEntry {
    pub tunnel_id: String,
    pub pid: u32,
    pub tunnel_name: String,
    pub started_at: String,
}

pub fn read_pids(app: &AppHandle) -> Vec<PidEntry> {
    let path = fs_paths::pid_file(app);
    if !path.exists() {
        return Vec::new();
    }
    let data = match std::fs::read_to_string(&path) {
        Ok(d) => d,
        Err(_) => return Vec::new(),
    };
    serde_json::from_str(&data).unwrap_or_default()
}

pub fn write_pids(app: &AppHandle, entries: &[PidEntry]) -> AppResult<()> {
    let path = fs_paths::pid_file(app);
    let data = serde_json::to_string_pretty(entries)?;
    std::fs::write(&path, data)?;
    Ok(())
}

pub fn upsert_pid(app: &AppHandle, entry: PidEntry) -> AppResult<()> {
    let mut entries = read_pids(app);
    entries.retain(|e| e.tunnel_id != entry.tunnel_id);
    entries.push(entry);
    write_pids(app, &entries)
}

pub fn remove_pid(app: &AppHandle, tunnel_id: &str) -> AppResult<()> {
    let mut entries = read_pids(app);
    entries.retain(|e| e.tunnel_id != tunnel_id);
    write_pids(app, &entries)
}

#[cfg(windows)]
pub fn is_pid_cloudflared(pid: u32) -> bool {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Process32First, Process32Next, PROCESSENTRY32,
        TH32CS_SNAPPROCESS,
    };
    use windows::Win32::System::Threading::OpenProcess;

    unsafe {
        let snapshot = match CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) {
            Ok(s) => s,
            Err(_) => return false,
        };
        let mut entry = PROCESSENTRY32 {
            dwSize: std::mem::size_of::<PROCESSENTRY32>() as u32,
            ..Default::default()
        };
        let mut found_pid = false;
        let mut image_name = String::new();
        if Process32First(snapshot, &mut entry).is_ok() {
            loop {
                if entry.th32ProcessID == pid {
                    found_pid = true;
                    let bytes: Vec<u8> = entry
                        .szExeFile
                        .iter()
                        .take_while(|c| **c != 0)
                        .map(|c| *c as u8)
                        .collect();
                    image_name = String::from_utf8_lossy(&bytes).to_string();
                    break;
                }
                if Process32Next(snapshot, &mut entry).is_err() {
                    break;
                }
            }
        }
        let _ = CloseHandle(snapshot);
        if !found_pid {
            return false;
        }
        let _ = OpenProcess(Default::default(), false, pid);
        image_name.to_lowercase().contains("cloudflared")
    }
}

#[cfg(not(windows))]
pub fn is_pid_cloudflared(_pid: u32) -> bool {
    false
}

#[cfg(windows)]
pub fn kill_pid(pid: u32) -> bool {
    use std::process::Command;
    let out = Command::new("taskkill")
        .args(["/F", "/T", "/PID", &pid.to_string()])
        .output();
    matches!(out, Ok(o) if o.status.success())
}

#[cfg(not(windows))]
pub fn kill_pid(_pid: u32) -> bool {
    false
}

pub fn find_orphans(app: &AppHandle) -> Vec<PidEntry> {
    read_pids(app)
        .into_iter()
        .filter(|e| is_pid_cloudflared(e.pid))
        .collect()
}

#[allow(dead_code)]
pub fn _cloudflared_home_path() -> Option<PathBuf> {
    fs_paths::cloudflared_home()
}
