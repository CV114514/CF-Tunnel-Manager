use crate::error::{AppError, AppResult};
use crate::state::{LogLine, TunnelStatus};
use chrono::Utc;
use regex::Regex;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::Mutex;
pub struct SpawnedTunnel {
    pub child: Child,
    pub pid: u32,
}

#[cfg(windows)]
pub fn assign_to_job(child: &tokio::process::Child) -> AppResult<()> {
    use std::os::windows::io::AsRawHandle;
    let handle = child
        .raw_handle()
        .ok_or_else(|| AppError::Other("no child handle".to_string()))?;
    let job = win32job::Job::create()
        .map_err(|e| AppError::Other(format!("create job: {e}")))?;
    let mut info = win32job::ExtendedLimitInfo::new();
    info.limit_kill_on_job_close();
    job.set_extended_limit_info(&mut info)
        .map_err(|e| AppError::Other(format!("set limit info: {e}")))?;
    job.assign_process(handle as isize)
        .map_err(|e| AppError::Other(format!("assign process: {e}")))?;
    std::mem::forget(job);
    Ok(())
}

#[cfg(not(windows))]
pub fn assign_to_job(_child: &tokio::process::Child) -> AppResult<()> {
    Ok(())
}

pub fn spawn_tunrun(exe: &PathBuf, config_path: &str) -> AppResult<SpawnedTunnel> {
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
    let pid = child
        .id()
        .ok_or_else(|| AppError::CommandFailed("no pid".to_string()))?;
    Ok(SpawnedTunnel { child, pid })
}

pub fn spawn_log_readers(
    app: AppHandle,
    tunnel_id: uuid::Uuid,
    child: &mut Child,
    state: Arc<Mutex<crate::state::Session>>,
) {
    let metrics_re = Regex::new(r"(?i)metrics.*?(?:127\.0\.0\.1|localhost):(\d+)").unwrap();
    let conn_re = Regex::new(r"Registered tunnel connection").unwrap();
    let loc_re = Regex::new(r"location=([A-Z]+)").unwrap();
    let err_re = Regex::new(r"(?i)\b(error|fatal|panic)\b").unwrap();

    if let Some(stdout) = child.stdout.take() {
        let app2 = app.clone();
        let id2 = tunnel_id;
        let state2 = state.clone();
        let metrics_re2 = metrics_re.clone();
        let conn_re2 = conn_re.clone();
        let loc_re2 = loc_re.clone();
        let err_re2 = err_re.clone();
        tokio::spawn(async move {
            let mut lines = BufReader::new(stdout).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                process_line(
                    &app2,
                    id2,
                    &state2,
                    &line,
                    "stdout",
                    &metrics_re2,
                    &conn_re2,
                    &loc_re2,
                    &err_re2,
                )
                .await;
            }
            mark_terminated(&app2, id2, &state2).await;
        });
    }
    if let Some(stderr) = child.stderr.take() {
        let app2 = app.clone();
        let id2 = tunnel_id;
        let state2 = state.clone();
        let metrics_re2 = metrics_re;
        let conn_re2 = conn_re;
        let loc_re2 = loc_re;
        let err_re2 = err_re;
        tokio::spawn(async move {
            let mut lines = BufReader::new(stderr).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                process_line(
                    &app2,
                    id2,
                    &state2,
                    &line,
                    "stderr",
                    &metrics_re2,
                    &conn_re2,
                    &loc_re2,
                    &err_re2,
                )
                .await;
            }
            mark_terminated(&app2, id2, &state2).await;
        });
    }

    let app_for_metrics = app.clone();
    let id_for_metrics = tunnel_id;
    let state_for_metrics = state.clone();
    tokio::spawn(async move {
        spawn_metrics_poller(app_for_metrics, id_for_metrics, state_for_metrics).await;
    });
}

async fn spawn_metrics_poller(
    app: AppHandle,
    tunnel_id: uuid::Uuid,
    state: Arc<Mutex<crate::state::Session>>,
) {
    let interval_secs = {
        let persisted = app.state::<Arc<Mutex<crate::state::PersistedState>>>();
        let p = persisted.lock().await;
        p.settings.metrics_poll_interval_secs.max(1)
    };
    let mut ticks = 0u32;
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(interval_secs)).await;
        let port = {
            let s = state.lock().await;
            s.running.get(&tunnel_id).and_then(|rt| rt.metrics_port)
        };
        let Some(port) = port else {
            ticks += 1;
            if ticks > 12 {
                return;
            }
            continue;
        };
        match crate::cloudflared::metrics::poll_once(port).await {
            Ok(snap) => {
                let _ = app.emit(&format!("tunnel:{tunnel_id}:metrics"), &snap);
                let mut s = state.lock().await;
                if let Some(rt) = s.running.get_mut(&tunnel_id) {
                    let _ = app.emit(
                        &format!("tunnel:{tunnel_id}:status"),
                        &rt.status.clone(),
                    );
                }
            }
            Err(_) => {}
        }
    }
}

async fn process_line(
    app: &AppHandle,
    tunnel_id: uuid::Uuid,
    state: &Arc<Mutex<crate::state::Session>>,
    line: &str,
    source: &str,
    metrics_re: &Regex,
    conn_re: &Regex,
    loc_re: &Regex,
    err_re: &Regex,
) {
    let truncated: String = if line.len() > 8192 {
        line.chars().take(8192).collect()
    } else {
        line.to_string()
    };
    let level = if err_re.is_match(line) {
        "error"
    } else if line.to_lowercase().contains("warn") {
        "warn"
    } else {
        "info"
    };
    let log_line = LogLine {
        ts: Utc::now(),
        level: level.to_string(),
        line: truncated.clone(),
        source: source.to_string(),
    };

    let mut s = state.lock().await;
    if let Some(rt) = s.running.get_mut(&tunnel_id) {
        if rt.log_buffer.len() >= crate::state::LOG_BUFFER_CAP {
            rt.log_buffer.pop_front();
        }
        rt.log_buffer.push_back(log_line.clone());

        if rt.metrics_port.is_none() {
            if let Some(cap) = metrics_re.captures(&truncated) {
                if let Some(p) = cap.get(1).and_then(|m| m.as_str().parse::<u16>().ok()) {
                    rt.metrics_port = Some(p);
                }
            }
        }

        if conn_re.is_match(&truncated) {
            let loc = loc_re
                .captures(&truncated)
                .and_then(|c| c.get(1).map(|m| m.as_str().to_string()));
            let new_status = match &rt.status {
                TunnelStatus::Connected { count, edge_locations } => {
                    let mut locs = edge_locations.clone();
                    if let Some(l) = loc {
                        if !locs.contains(&l) {
                            locs.push(l);
                        }
                    }
                    TunnelStatus::Connected {
                        count: count + 1,
                        edge_locations: locs,
                    }
                }
                _ => TunnelStatus::Connected {
                    count: 1,
                    edge_locations: loc.into_iter().collect(),
                },
            };
            rt.status = new_status;
        } else if level == "error" {
            if !matches!(rt.status, TunnelStatus::Connected { .. }) {
                rt.status = TunnelStatus::Error(truncated.clone());
            }
        }

        let _ = app.emit(
            &format!("tunnel:{tunnel_id}:log"),
            &log_line,
        );
        let _ = app.emit(
            &format!("tunnel:{tunnel_id}:status"),
            &rt.status.clone(),
        );
    }
}

async fn mark_terminated(
    app: &AppHandle,
    tunnel_id: uuid::Uuid,
    state: &Arc<Mutex<crate::state::Session>>,
) {
    let mut s = state.lock().await;
    let new_status = if let Some(rt) = s.running.get_mut(&tunnel_id) {
        let last_logs: String = rt
            .log_buffer
            .iter()
            .rev()
            .take(8)
            .rev()
            .map(|l| l.line.clone())
            .collect::<Vec<_>>()
            .join("\n");
        let new_status = match &rt.status {
            TunnelStatus::Stopping => TunnelStatus::Stopped,
            TunnelStatus::Error(_) => rt.status.clone(),
            _ => {
                let msg = if last_logs.trim().is_empty() {
                    "cloudflared 进程意外退出，未产生任何日志".to_string()
                } else {
                    last_logs
                };
                TunnelStatus::Error(msg)
            }
        };
        rt.status = new_status.clone();
        if let Some(handle) = rt.metrics_handle.take() {
            handle.abort();
        }
        new_status
    } else {
        return;
    };

    let should_remove = matches!(new_status, TunnelStatus::Stopped);
    if should_remove {
        if let Some(mut rt) = s.running.remove(&tunnel_id) {
            let _ = rt.child.wait().await;
        }
    }
    drop(s);

    let _ = app.emit(&format!("tunnel:{tunnel_id}:status"), &new_status);
    let _ = crate::pid_file::remove_pid(app, &tunnel_id.to_string());
    let _ = app.emit("global:state_changed", ());
}
