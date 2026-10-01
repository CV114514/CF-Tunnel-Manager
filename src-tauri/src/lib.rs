mod cloudflared;
mod commands;
mod error;
mod fs_paths;
mod pid_file;
mod state;
mod tray;

use std::sync::Arc;
use state::{PersistedState, Session};
use tauri::{Emitter, Manager};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with_target(true)
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let handle = app.handle();

            let persisted = load_persisted(handle);
            let session = Arc::new(tokio::sync::Mutex::new(Session::default()));
            let persisted_arc = Arc::new(tokio::sync::Mutex::new(persisted));

            app.manage(persisted_arc.clone());
            app.manage(session.clone());

            let app2 = handle.clone();
            tauri::async_runtime::spawn(async move {
                let orphans = pid_file::find_orphans(&app2);
                if !orphans.is_empty() {
                    tracing::warn!("found {} orphan cloudflared processes", orphans.len());
                    let _ = app2.emit("orphans:detected", &orphans);
                }
            });

            tray::build_tray(handle)?;

            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }

            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let _ = window.hide();
                api.prevent_close();
                let _ = window.app_handle().emit("window:hidden", ());
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_state,
            commands::set_cloudflared_exe,
            commands::test_cloudflared_version,
            commands::check_login,
            commands::start_login,
            commands::list_tunnels_remote,
            commands::create_tunnel,
            commands::edit_tunnel,
            commands::delete_tunnel,
            commands::route_dns,
            commands::start_tunnel,
            commands::stop_tunnel,
            commands::restart_tunnel,
            commands::get_tunnel_status,
            commands::get_tunnel_logs,
            commands::get_tunnel_metrics,
            commands::open_cloudflared_dir,
            commands::open_config_file,
            commands::cleanup_orphans,
            commands::quit_app,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn load_persisted(app: &tauri::AppHandle) -> PersistedState {
    let path = fs_paths::state_file(app);
    if !path.exists() {
        return PersistedState::default();
    }
    let data = match std::fs::read_to_string(&path) {
        Ok(d) => d,
        Err(_) => return PersistedState::default(),
    };
    serde_json::from_str(&data).unwrap_or_default()
}
