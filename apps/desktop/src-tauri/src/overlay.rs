use std::thread;
use std::time::Duration;

use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

use crate::state::AppState;

pub(crate) fn apply_overlay_visibility(app: &AppHandle, enabled: bool) -> Result<(), String> {
    if enabled {
        if let Some(existing) = app.get_webview_window("overlay") {
            let _ = existing.show();
            return Ok(());
        }
        let builder = WebviewWindowBuilder::new(
            app,
            "overlay",
            WebviewUrl::App("index.html?overlay=1".into()),
        )
        .title("WhisperPro Widget")
        .inner_size(240.0, 88.0)
        .resizable(false)
        .maximizable(false)
        .minimizable(false)
        .closable(false)
        .decorations(false)
        .transparent(true)
        .shadow(false)
        .always_on_top(true)
        .focused(false)
        .skip_taskbar(true)
        .visible(true);
        let _window = builder
            .build()
            .map_err(|e| format!("Creation fenetre widget impossible: {e}"))?;
    } else if let Some(window) = app.get_webview_window("overlay") {
        let _ = window.hide();
    }
    Ok(())
}

pub(crate) fn show_overlay_near_cursor(app: &AppHandle) -> Result<(), String> {
    apply_overlay_visibility(app, true)?;
    let Some(window) = app.get_webview_window("overlay") else {
        return Ok(());
    };

    if let Some((cursor_x, cursor_y)) = current_cursor_position() {
        let target_x = (cursor_x - 110.0).max(0.0);
        let target_y = (cursor_y - 120.0).max(0.0);
        let state = app.state::<AppState>();
        let previous = *state.overlay_last_position.lock();

        if let Some((start_x, start_y)) = previous {
            let dx = target_x - start_x;
            let dy = target_y - start_y;
            let distance = (dx * dx + dy * dy).sqrt();
            if distance > 8.0 {
                let steps = ((distance / 42.0).ceil() as i32).clamp(3, 8);
                for i in 1..=steps {
                    let t = i as f64 / steps as f64;
                    let eased = 1.0 - (1.0 - t).powi(3);
                    let x = start_x + dx * eased;
                    let y = start_y + dy * eased;
                    let _ =
                        window.set_position(tauri::Position::Logical(tauri::LogicalPosition { x, y }));
                    thread::sleep(Duration::from_millis(10));
                }
            } else {
                let _ = window.set_position(tauri::Position::Logical(tauri::LogicalPosition {
                    x: target_x,
                    y: target_y,
                }));
            }
        } else {
            let _ = window.set_position(tauri::Position::Logical(tauri::LogicalPosition {
                x: target_x,
                y: target_y,
            }));
        }
        *state.overlay_last_position.lock() = Some((target_x, target_y));
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn current_cursor_position() -> Option<(f64, f64)> {
    #[repr(C)]
    struct Point {
        x: i32,
        y: i32,
    }
    unsafe extern "system" {
        fn GetCursorPos(lp_point: *mut Point) -> i32;
    }

    let mut pt = Point { x: 0, y: 0 };
    let ok = unsafe { GetCursorPos(&mut pt as *mut Point) };
    if ok != 0 {
        Some((pt.x as f64, pt.y as f64))
    } else {
        None
    }
}

#[cfg(not(target_os = "windows"))]
fn current_cursor_position() -> Option<(f64, f64)> {
    None
}
