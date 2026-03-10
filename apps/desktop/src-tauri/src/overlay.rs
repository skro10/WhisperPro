use std::thread;
use std::time::Duration;

use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

use crate::state::AppState;

const OVERLAY_WIDTH_PX: f64 = 240.0;
const OVERLAY_HEIGHT_PX: f64 = 88.0;

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
        let raw_target_x = cursor_x - 110.0;
        let raw_target_y = cursor_y - 120.0;
        let (target_x, target_y) =
            clamp_overlay_position_to_monitor(raw_target_x, raw_target_y, cursor_x, cursor_y);
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
                    let _ = window.set_position(tauri::Position::Physical(tauri::PhysicalPosition {
                        x: x.round() as i32,
                        y: y.round() as i32,
                    }));
                    thread::sleep(Duration::from_millis(10));
                }
            } else {
                let _ = window.set_position(tauri::Position::Physical(tauri::PhysicalPosition {
                    x: target_x.round() as i32,
                    y: target_y.round() as i32,
                }));
            }
        } else {
            let _ = window.set_position(tauri::Position::Physical(tauri::PhysicalPosition {
                x: target_x.round() as i32,
                y: target_y.round() as i32,
            }));
        }
        *state.overlay_last_position.lock() = Some((target_x, target_y));
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn clamp_overlay_position_to_monitor(
    target_x: f64,
    target_y: f64,
    cursor_x: f64,
    cursor_y: f64,
) -> (f64, f64) {
    let Some((left, top, right, bottom)) =
        current_monitor_work_area(cursor_x.round() as i32, cursor_y.round() as i32)
    else {
        return (target_x, target_y);
    };

    let min_x = left as f64;
    let min_y = top as f64;
    let max_x = (right as f64 - OVERLAY_WIDTH_PX).max(min_x);
    let max_y = (bottom as f64 - OVERLAY_HEIGHT_PX).max(min_y);

    (target_x.clamp(min_x, max_x), target_y.clamp(min_y, max_y))
}

#[cfg(not(target_os = "windows"))]
fn clamp_overlay_position_to_monitor(
    target_x: f64,
    target_y: f64,
    _cursor_x: f64,
    _cursor_y: f64,
) -> (f64, f64) {
    (target_x, target_y)
}

#[cfg(target_os = "windows")]
#[repr(C)]
struct WinPoint {
    x: i32,
    y: i32,
}

#[cfg(target_os = "windows")]
#[repr(C)]
struct WinRect {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}

#[cfg(target_os = "windows")]
#[repr(C)]
struct WinMonitorInfo {
    cb_size: u32,
    rc_monitor: WinRect,
    rc_work: WinRect,
    dw_flags: u32,
}

#[cfg(target_os = "windows")]
type Hmonitor = *mut std::ffi::c_void;

#[cfg(target_os = "windows")]
const MONITOR_DEFAULTTONEAREST: u32 = 2;

#[cfg(target_os = "windows")]
unsafe extern "system" {
    fn GetCursorPos(lp_point: *mut WinPoint) -> i32;
    fn MonitorFromPoint(pt: WinPoint, dw_flags: u32) -> Hmonitor;
    fn GetMonitorInfoW(h_monitor: Hmonitor, lpmi: *mut WinMonitorInfo) -> i32;
}

#[cfg(target_os = "windows")]
fn current_cursor_position() -> Option<(f64, f64)> {
    let mut pt = WinPoint { x: 0, y: 0 };
    let ok = unsafe { GetCursorPos(&mut pt as *mut WinPoint) };
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

#[cfg(target_os = "windows")]
fn current_monitor_work_area(x: i32, y: i32) -> Option<(i32, i32, i32, i32)> {
    let monitor = unsafe { MonitorFromPoint(WinPoint { x, y }, MONITOR_DEFAULTTONEAREST) };
    if monitor.is_null() {
        return None;
    }

    let mut info = WinMonitorInfo {
        cb_size: std::mem::size_of::<WinMonitorInfo>() as u32,
        rc_monitor: WinRect {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        },
        rc_work: WinRect {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        },
        dw_flags: 0,
    };
    let ok = unsafe { GetMonitorInfoW(monitor, &mut info as *mut WinMonitorInfo) };
    if ok == 0 {
        return None;
    }

    Some((
        info.rc_work.left,
        info.rc_work.top,
        info.rc_work.right,
        info.rc_work.bottom,
    ))
}

#[cfg(not(target_os = "windows"))]
fn current_monitor_work_area(_x: i32, _y: i32) -> Option<(i32, i32, i32, i32)> {
    None
}
