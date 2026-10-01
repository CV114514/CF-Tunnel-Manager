use crate::state::{Session, TunnelStatus};
use crate::commands;
use std::sync::Arc;
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager,
};

pub fn build_tray(app: &tauri::AppHandle) -> tauri::Result<()> {
    let show_item = MenuItem::with_id(app, "show", "显示窗口", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show_item, &quit_item])?;

    let _tray = TrayIconBuilder::with_id("main")
        .icon(app.default_window_icon().unwrap().clone())
        .tooltip("CF Tunnel Manager — 未运行")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            "quit" => {
                let app2 = app.clone();
                tauri::async_runtime::spawn(async move {
                    let session = app2
                        .state::<Arc<tokio::sync::Mutex<Session>>>()
                        .inner()
                        .clone();
                    let ids: Vec<uuid::Uuid> = {
                        let s = session.lock().await;
                        s.running.keys().cloned().collect()
                    };
                    for id in ids {
                        let _ = commands::stop_tunnel_inner(app2.clone(), id, Arc::clone(&session)).await;
                    }
                    let _ = crate::pid_file::write_pids(&app2, &[]);
                    app2.exit(0);
                    std::thread::spawn(|| {
                        std::thread::sleep(std::time::Duration::from_secs(2));
                        std::process::exit(0);
                    });
                });
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                if let Some(window) = app.get_webview_window("main") {
                    if window.is_visible().unwrap_or(false) {
                        let _ = window.hide();
                    } else {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
            }
        })
        .build(app)?;

    let app_for_status = app.clone();
    tauri::async_runtime::spawn(async move {
        status_watcher(app_for_status).await;
    });

    Ok(())
}

async fn status_watcher(app: tauri::AppHandle) {
    let mut last_color: Option<&'static str> = None;
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        let session = app.state::<Arc<tokio::sync::Mutex<Session>>>();
        let (color, summary) = {
            let s = session.lock().await;
            aggregate(&s)
        };
        if Some(color) != last_color {
            last_color = Some(color);
            let _ = update_tray_icon(&app, color);
            let _ = update_tray_tooltip(&app, &summary);
        }
    }
}

fn aggregate(session: &Session) -> (&'static str, String) {
    let mut connected = 0u32;
    let mut starting = false;
    let mut stopping = false;
    let mut has_error = false;
    let mut total_running = 0u32;
    for rt in session.running.values() {
        match &rt.status {
            TunnelStatus::Connected { count, .. } => {
                connected += 1;
                total_running += count;
            }
            TunnelStatus::Starting => starting = true,
            TunnelStatus::Stopping => stopping = true,
            TunnelStatus::Error(_) => has_error = true,
            TunnelStatus::Stopped => {}
        }
    }
    let color = if connected > 0 {
        "green"
    } else if starting || stopping {
        "yellow"
    } else if has_error {
        "red"
    } else {
        "grey"
    };
    let summary = match (connected, starting, stopping, has_error) {
        (0, false, false, false) => "未运行".to_string(),
        (n, false, false, false) => format!("{n} 个隧道已连接（共 {total_running} 条 edge 连接）"),
        (_, _, _, _) if starting => "隧道启动中…".to_string(),
        (_, _, _, _) if stopping => "隧道停止中…".to_string(),
        _ => "有错误".to_string(),
    };
    (color, summary)
}

fn update_tray_icon(app: &tauri::AppHandle, color: &str) -> tauri::Result<()> {
    let bytes = match color {
        "green" => generate_dot_png(0xa6, 0xe3, 0xa1),
        "yellow" => generate_dot_png(0xf9, 0xe2, 0xaf),
        "red" => generate_dot_png(0xf3, 0x8b, 0xa8),
        _ => generate_dot_png(0x6c, 0x70, 0x86),
    };
    let img = tauri::image::Image::new(&bytes, 32, 32);
    if let Some(tray) = app.tray_by_id("main") {
        tray.set_icon(Some(img))?;
    }
    Ok(())
}

fn update_tray_tooltip(app: &tauri::AppHandle, summary: &str) -> tauri::Result<()> {
    if let Some(tray) = app.tray_by_id("main") {
        let title = format!("CF Tunnel Manager — {summary}");
        tray.set_tooltip(Some(&title))?;
    }
    Ok(())
}

fn generate_dot_png(r: u8, g: u8, b: u8) -> Vec<u8> {
    let mut buf = Vec::with_capacity(32 * 32 * 4);
    for y in 0..32 {
        for x in 0..32 {
            let dx = x as i32 - 16;
            let dy = y as i32 - 16;
            let in_outer = dx * dx + dy * dy <= 14 * 14;
            let in_inner = dx * dx + dy * dy <= 9 * 9;
            if in_inner {
                buf.extend_from_slice(&[r, g, b, 255]);
            } else if in_outer {
                buf.extend_from_slice(&[(r / 2), (g / 2), (b / 2), 255]);
            } else {
                buf.extend_from_slice(&[0, 0, 0, 0]);
            }
        }
    }
    buf
}
