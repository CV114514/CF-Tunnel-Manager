use crate::error::{AppError, AppResult};
use crate::state::{PersistedState, TunnelProfile, TunnelStatus};
use crate::{cloudflared, fs_paths, pid_file, state};
use chrono::Utc;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tokio::sync::Mutex;
use uuid::Uuid;

#[tauri::command]
pub async fn get_state(
    app: AppHandle,
    persisted: tauri::State<'_, Arc<Mutex<PersistedState>>>,
    session: tauri::State<'_, Arc<Mutex<state::Session>>>,
) -> Result<state::AppStateSnapshot, String> {
    let p = persisted.lock().await;
    let s = session.lock().await;
    let mut running = std::collections::HashMap::new();
    for (id, rt) in s.running.iter() {
        running.insert(id.to_string(), rt.status.clone());
    }
    let cert = cloudflared::login::check_cert().ok();
    let _ = app;
    Ok(state::AppStateSnapshot {
        settings: p.settings.clone(),
        tunnels: p.tunnels.clone(),
        cert,
        running,
    })
}

#[tauri::command]
pub async fn set_cloudflared_exe(
    app: AppHandle,
    path: String,
    persisted: tauri::State<'_, Arc<Mutex<PersistedState>>>,
) -> Result<state::VersionInfo, String> {
    let p = PathBuf::from(&path);
    if !p.exists() {
        return Err(format!("file not found: {path}"));
    }
    let version = cloudflared::probe_version(&p).await.map_err(|e| e.to_string())?;
    {
        let mut st = persisted.lock().await;
        st.settings.cloudflared_exe_path = Some(p);
        save_state(&app, &st).await.ok();
    }
    let _ = app.emit("global:state_changed", ());
    Ok(version)
}

#[tauri::command]
pub async fn test_cloudflared_version(
    persisted: tauri::State<'_, Arc<Mutex<PersistedState>>>,
) -> Result<state::VersionInfo, String> {
    let p = persisted.lock().await;
    let exe = p
        .settings
        .cloudflared_exe_path
        .clone()
        .ok_or_else(|| "exe path not set".to_string())?;
    drop(p);
    cloudflared::probe_version(&exe).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn check_login() -> Result<state::CertInfo, String> {
    cloudflared::login::check_cert().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn start_login(
    app: AppHandle,
    persisted: tauri::State<'_, Arc<Mutex<PersistedState>>>,
) -> Result<bool, String> {
    let exe = {
        let p = persisted.lock().await;
        p.settings
            .cloudflared_exe_path
            .clone()
            .ok_or_else(|| "exe path not set".to_string())?
    };
    cloudflared::login::start_login_flow(&exe)
        .await
        .map_err(|e| e.to_string())?;
    let appeared = cloudflared::login::wait_for_cert(std::time::Duration::from_secs(300))
        .await
        .map_err(|e| e.to_string())?;
    if appeared {
        let _ = app.emit("login:cert_changed", ());
    }
    Ok(appeared)
}

#[tauri::command]
pub async fn list_tunnels_remote(
    persisted: tauri::State<'_, Arc<Mutex<PersistedState>>>,
) -> Result<Vec<state::RemoteTunnel>, String> {
    let exe = {
        let p = persisted.lock().await;
        p.settings
            .cloudflared_exe_path
            .clone()
            .ok_or_else(|| "exe path not set".to_string())?
    };
    cloudflared::tunnel::list_tunnels(&exe)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn create_tunnel(
    app: AppHandle,
    profile: state::NewTunnelProfile,
    persisted: tauri::State<'_, Arc<Mutex<PersistedState>>>,
) -> Result<TunnelProfile, String> {
    cloudflared::config::validate_name(&profile.name).map_err(|e| e.to_string())?;
    cloudflared::config::validate_hostname(&profile.hostname).map_err(|e| e.to_string())?;
    cloudflared::config::validate_service_url(&profile.target_service_url)
        .map_err(|e| e.to_string())?;

    let cert = cloudflared::login::check_cert().map_err(|e| e.to_string())?;
    if !cert.logged_in {
        return Err("not logged in (cert.pem missing)".to_string());
    }

    let exe = {
        let p = persisted.lock().await;
        p.settings
            .cloudflared_exe_path
            .clone()
            .ok_or_else(|| "exe path not set".to_string())?
    };

    let (uuid, creds_path) = cloudflared::tunnel::create_tunnel(&exe, &profile.name)
        .await
        .map_err(|e| e.to_string())?;

    let id = Uuid::new_v4();
    let config_path = fs_paths::tunnels_dir(&app).join(id.to_string()).join("config.yml");
    cloudflared::config::write_config(
        &config_path,
        &profile.name,
        &creds_path,
        &profile.hostname,
        &profile.target_service_url,
    )
    .map_err(|e| e.to_string())?;

    let dns_result = cloudflared::tunnel::route_dns(&exe, &profile.name, &profile.hostname).await;
    let dns_routed = dns_result.is_ok();

    let tunnel = TunnelProfile {
        id,
        name: profile.name,
        hostname: profile.hostname,
        target_service_url: profile.target_service_url,
        config_yaml_path: config_path,
        credentials_json_path: Some(PathBuf::from(creds_path)),
        cloudflared_tunnel_uuid: Some(uuid),
        dns_routed,
        created_at: Utc::now(),
        last_started_at: None,
    };

    {
        let mut st = persisted.lock().await;
        st.tunnels.push(tunnel.clone());
        save_state(&app, &st).await.ok();
    }
    let _ = app.emit("global:state_changed", ());
    Ok(tunnel)
}

#[tauri::command]
pub async fn edit_tunnel(
    app: AppHandle,
    id: Uuid,
    patch: state::TunnelPatch,
    persisted: tauri::State<'_, Arc<Mutex<PersistedState>>>,
) -> Result<TunnelProfile, String> {
    let exe = {
        let p = persisted.lock().await;
        p.settings
            .cloudflared_exe_path
            .clone()
            .ok_or_else(|| "exe path not set".to_string())?
    };

    let mut st = persisted.lock().await;
    let tunnel = st
        .tunnels
        .iter_mut()
        .find(|t| t.id == id)
        .ok_or_else(|| "tunnel not found".to_string())?;

    if let Some(hostname) = patch.hostname {
        if hostname != tunnel.hostname {
            cloudflared::config::validate_hostname(&hostname).map_err(|e| e.to_string())?;
            cloudflared::tunnel::route_dns(&exe, &tunnel.name, &hostname)
                .await
                .map_err(|e| e.to_string())?;
            tunnel.hostname = hostname;
            tunnel.dns_routed = true;
        }
    }
    if let Some(url) = patch.target_service_url {
        cloudflared::config::validate_service_url(&url).map_err(|e| e.to_string())?;
        tunnel.target_service_url = url;
    }
    let creds = tunnel
        .credentials_json_path
        .clone()
        .ok_or_else(|| "credentials_json_path missing".to_string())?
        .to_string_lossy()
        .to_string();
    cloudflared::config::write_config(
        &tunnel.config_yaml_path,
        &tunnel.name,
        &creds,
        &tunnel.hostname,
        &tunnel.target_service_url,
    )
    .map_err(|e| e.to_string())?;

    let updated = tunnel.clone();
    save_state(&app, &st).await.ok();
    drop(st);
    let _ = app.emit("global:state_changed", ());
    Ok(updated)
}

#[tauri::command]
pub async fn delete_tunnel(
    app: AppHandle,
    id: Uuid,
    persisted: tauri::State<'_, Arc<Mutex<PersistedState>>>,
    session: tauri::State<'_, Arc<Mutex<state::Session>>>,
) -> Result<(), String> {
    let exe = {
        let p = persisted.lock().await;
        p.settings
            .cloudflared_exe_path
            .clone()
            .ok_or_else(|| "exe path not set".to_string())?
    };

    let tunnel_name = {
        let st = persisted.lock().await;
        st.tunnels
            .iter()
            .find(|t| t.id == id)
            .map(|t| t.name.clone())
            .ok_or_else(|| "tunnel not found".to_string())?
    };

    {
        let s = session.lock().await;
        if s.running.contains_key(&id) {
            return Err("tunnel is running — stop it first".to_string());
        }
    }

    cloudflared::tunnel::delete_tunnel(&exe, &tunnel_name)
        .await
        .map_err(|e| e.to_string())?;

    {
        let mut st = persisted.lock().await;
        if let Some(idx) = st.tunnels.iter().position(|t| t.id == id) {
            let t = st.tunnels[idx].clone();
            let _ = std::fs::remove_dir_all(
                t.config_yaml_path
                    .parent()
                    .unwrap_or_else(|| std::path::Path::new(".")),
            );
            st.tunnels.remove(idx);
        }
        save_state(&app, &st).await.ok();
    }
    pid_file::remove_pid(&app, &id.to_string()).ok();
    let _ = app.emit("global:state_changed", ());
    Ok(())
}

#[tauri::command]
pub async fn route_dns(
    id: Uuid,
    persisted: tauri::State<'_, Arc<Mutex<PersistedState>>>,
) -> Result<(), String> {
    let exe = {
        let p = persisted.lock().await;
        p.settings
            .cloudflared_exe_path
            .clone()
            .ok_or_else(|| "exe path not set".to_string())?
    };
    let st = persisted.lock().await;
    let tunnel = st
        .tunnels
        .iter()
        .find(|t| t.id == id)
        .ok_or_else(|| "tunnel not found".to_string())?;
    cloudflared::tunnel::route_dns(&exe, &tunnel.name, &tunnel.hostname)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn start_tunnel(
    app: AppHandle,
    id: Uuid,
    persisted: tauri::State<'_, Arc<Mutex<PersistedState>>>,
    session: tauri::State<'_, Arc<Mutex<state::Session>>>,
) -> Result<(), String> {
    start_tunnel_inner(app, id, Arc::clone(&persisted), Arc::clone(&session)).await
}

pub async fn start_tunnel_inner(
    app: AppHandle,
    id: Uuid,
    persisted: Arc<Mutex<PersistedState>>,
    session: Arc<Mutex<state::Session>>,
) -> Result<(), String> {
    let exe = {
        let p = persisted.lock().await;
        p.settings
            .cloudflared_exe_path
            .clone()
            .ok_or_else(|| "exe path not set".to_string())?
    };

    let (config_path, name) = {
        let st = persisted.lock().await;
        let t = st
            .tunnels
            .iter()
            .find(|t| t.id == id)
            .ok_or_else(|| "tunnel not found".to_string())?;
        (t.config_yaml_path.clone(), t.name.clone())
    };

    {
        let mut s = session.lock().await;
        if s.running.contains_key(&id) {
            return Err("already running".to_string());
        }
    }

    let mut spawned = cloudflared::run::spawn_tunrun(&exe, &config_path.to_string_lossy())
        .map_err(|e| e.to_string())?;
    let pid = spawned.pid;
    cloudflared::run::assign_to_job(&spawned.child).map_err(|e| e.to_string())?;
    cloudflared::run::spawn_log_readers(app.clone(), id, &mut spawned.child, Arc::clone(&session));

    {
        let mut s = session.lock().await;
        s.running.insert(id, state::RunningTunnel::new(spawned.child, pid));
    }

    pid_file::upsert_pid(
        &app,
        pid_file::PidEntry {
            tunnel_id: id.to_string(),
            pid,
            tunnel_name: name,
            started_at: Utc::now().to_rfc3339(),
        },
    )
    .ok();

    let now = Utc::now();
    {
        let mut st = persisted.lock().await;
        if let Some(t) = st.tunnels.iter_mut().find(|t| t.id == id) {
            t.last_started_at = Some(now);
        }
        save_state(&app, &st).await.ok();
    }
    let _ = app.emit("global:state_changed", ());
    Ok(())
}

#[tauri::command]
pub async fn stop_tunnel(
    app: AppHandle,
    id: Uuid,
    session: tauri::State<'_, Arc<Mutex<state::Session>>>,
) -> Result<(), String> {
    stop_tunnel_inner(app, id, Arc::clone(&session)).await
}

pub async fn stop_tunnel_inner(
    app: AppHandle,
    id: Uuid,
    session: Arc<Mutex<state::Session>>,
) -> Result<(), String> {
    // Quick lock: set Stopping, kill, remove entry, drop lock
    let rt_opt = {
        let mut s = session.lock().await;
        let Some(rt) = s.running.get_mut(&id) else {
            return Ok(());
        };
        rt.status = TunnelStatus::Stopping;
        let _ = rt.child.kill();
        let _ = app.emit(&format!("tunnel:{id}:status"), &rt.status.clone());
        s.running.remove(&id)
    };

    // Slow cleanup without holding the lock
    if let Some(mut rt) = rt_opt {
        if let Some(handle) = rt.metrics_handle.take() {
            handle.abort();
        }
        let wait_result = tokio::time::timeout(
            std::time::Duration::from_secs(3),
            rt.child.wait(),
        )
        .await;
        if wait_result.is_err() {
            let _ = pid_file::kill_pid(rt.pid);
            let _ = rt.child.wait().await;
        }
    }

    pid_file::remove_pid(&app, &id.to_string()).ok();
    let _ = app.emit("global:state_changed", ());
    Ok(())
}

#[tauri::command]
pub async fn restart_tunnel(
    app: AppHandle,
    id: Uuid,
    persisted: tauri::State<'_, Arc<Mutex<PersistedState>>>,
    session: tauri::State<'_, Arc<Mutex<state::Session>>>,
) -> Result<(), String> {
    let session_arc = Arc::clone(&session);
    let persisted_arc = Arc::clone(&persisted);
    stop_tunnel_inner(app.clone(), id, session_arc).await.ok();
    start_tunnel_inner(app, id, persisted_arc, Arc::clone(&session)).await
}

#[tauri::command]
pub async fn get_tunnel_status(
    id: Uuid,
    session: tauri::State<'_, Arc<Mutex<state::Session>>>,
) -> Result<TunnelStatus, String> {
    let s = session.lock().await;
    Ok(s.running
        .get(&id)
        .map(|rt| rt.status.clone())
        .unwrap_or(TunnelStatus::Stopped))
}

#[tauri::command]
pub async fn get_tunnel_logs(
    id: Uuid,
    session: tauri::State<'_, Arc<Mutex<state::Session>>>,
) -> Result<Vec<state::LogLine>, String> {
    let s = session.lock().await;
    Ok(s.running
        .get(&id)
        .map(|rt| rt.log_buffer.iter().cloned().collect())
        .unwrap_or_default())
}

#[tauri::command]
pub async fn get_tunnel_metrics(
    id: Uuid,
    session: tauri::State<'_, Arc<Mutex<state::Session>>>,
) -> Result<Option<state::MetricsSnapshot>, String> {
    let s = session.lock().await;
    let port = s.running.get(&id).and_then(|rt| rt.metrics_port);
    drop(s);
    if let Some(p) = port {
        Ok(Some(
            cloudflared::metrics::poll_once(p)
                .await
                .map_err(|e| e.to_string())?,
        ))
    } else {
        Ok(None)
    }
}

#[tauri::command]
pub async fn open_cloudflared_dir() -> Result<(), String> {
    let path = fs_paths::cloudflared_home().ok_or_else(|| "USERPROFILE not set".to_string())?;
    open_path(&path).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn open_config_file(
    id: Uuid,
    persisted: tauri::State<'_, Arc<Mutex<PersistedState>>>,
) -> Result<(), String> {
    let st = persisted.lock().await;
    let t = st
        .tunnels
        .iter()
        .find(|t| t.id == id)
        .ok_or_else(|| "tunnel not found".to_string())?;
    open_path(&t.config_yaml_path).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn cleanup_orphans(app: AppHandle) -> Result<Vec<pid_file::PidEntry>, String> {
    let orphans = pid_file::find_orphans(&app);
    let mut killed = Vec::new();
    for o in orphans {
        if pid_file::kill_pid(o.pid) {
            killed.push(o);
        }
    }
    pid_file::write_pids(&app, &[]).ok();
    Ok(killed)
}

#[tauri::command]
pub async fn quit_app(
    app: AppHandle,
    persisted: tauri::State<'_, Arc<Mutex<PersistedState>>>,
    session: tauri::State<'_, Arc<Mutex<state::Session>>>,
) -> Result<(), String> {
    let session_arc = Arc::clone(&session);
    let ids: Vec<Uuid> = {
        let s = session_arc.lock().await;
        s.running.keys().cloned().collect()
    };
    for id in ids {
        let _ = stop_tunnel_inner(app.clone(), id, Arc::clone(&session_arc)).await;
    }
    let _ = persisted;
    // Force exit after a short grace — `app.exit(0)` should suffice but
    // sometimes the runtime hangs on lingering tokio tasks, so flush pids and
    // call process::exit as a hard fallback.
    let _ = pid_file::write_pids(&app, &[]);
    app.exit(0);
    std::thread::spawn(|| {
        std::thread::sleep(std::time::Duration::from_secs(2));
        std::process::exit(0);
    });
    Ok(())
}

#[allow(dead_code)]
pub async fn save_state(app: &AppHandle, state: &PersistedState) -> AppResult<()> {
    let path = fs_paths::state_file(app);
    let data = serde_json::to_string_pretty(state)?;
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent)?;
        }
    }
    std::fs::write(&path, data)?;
    Ok(())
}

fn open_path(path: &std::path::Path) -> AppResult<()> {
    #[cfg(windows)]
    {
        std::process::Command::new("explorer")
            .arg(path)
            .spawn()
            .map_err(|e| crate::error::AppError::Other(format!("explorer: {e}")))?;
    }
    #[cfg(not(windows))]
    {
        let _ = path;
    }
    Ok(())
}
