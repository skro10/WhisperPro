use std::sync::atomic::Ordering;
use std::thread;
use std::time::{Duration, Instant};

use arboard::Clipboard;
use enigo::{Direction, Enigo, Key, Keyboard, Settings as EnigoSettings};
use tauri::{AppHandle, Emitter, Manager};
use tracing::error;

use crate::state::*;
use crate::{apply_overlay_visibility, show_overlay_near_cursor};

const CLIPBOARD_SETTLE_BEFORE_PASTE_MS: u64 = 180;
const CLIPBOARD_RESTORE_BASE_DELAY_MS: u64 = 2500;
const CLIPBOARD_RESTORE_STEP_CHARS: usize = 500;
const CLIPBOARD_RESTORE_STEP_DELAY_MS: u64 = 350;
const CLIPBOARD_RESTORE_MAX_DELAY_MS: u64 = 10_000;

pub(crate) fn emit_dictation_status(app: &AppHandle, state: &str, message: &str) {
    let app_state = app.state::<AppState>();
    let widget_enabled = app_state.widget_enabled.load(Ordering::SeqCst);
    let token = app_state.overlay_hide_token.fetch_add(1, Ordering::SeqCst) + 1;
    if widget_enabled && matches!(state, "listening" | "transcribing") {
        let _ = show_overlay_near_cursor(app);
    }

    let payload = DictationStatusEvent {
        state: state.to_string(),
        message: message.to_string(),
    };
    *app.state::<AppState>().dictation_status.lock() = payload.clone();

    if let Err(e) = app.emit("dictation-status", payload.clone()) {
        error!(target: "overlay", reason = %e, "broadcast dictation-status failed");
    }

    if widget_enabled && state == "idle" {
        let _ = apply_overlay_visibility(app, false);
    }

    if widget_enabled && matches!(state, "done" | "error") {
        let app_handle = app.clone();
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(560));
            let latest_token = app_handle
                .state::<AppState>()
                .overlay_hide_token
                .load(Ordering::SeqCst);
            if latest_token == token {
                let _ = apply_overlay_visibility(&app_handle, false);
            }
        });
    }
}

pub(crate) fn should_skip_duplicate_injection(app_state: &AppState, text: &str) -> bool {
    let guard = app_state.last_successful_injection.lock();
    if let Some((last_text, last_at)) = guard.as_ref() {
        if last_text == text && last_at.elapsed() < Duration::from_millis(1200) {
            return true;
        }
    }
    false
}

pub(crate) fn mark_successful_injection(app_state: &AppState, text: &str) {
    *app_state.last_successful_injection.lock() = Some((text.to_string(), Instant::now()));
}

pub(crate) struct InjectionReport {
    pub(crate) mode: &'static str,
    pub(crate) attempts: u8,
    pub(crate) text_len: usize,
}

pub(crate) fn inject_text_with_retry(text: &str) -> Result<InjectionReport, String> {
    let text = text.trim();
    let text_len = text.chars().count();
    if text.is_empty() {
        return Ok(InjectionReport {
            mode: "clipboard-paste",
            attempts: 0,
            text_len: 0,
        });
    }

    let mut clipboard = Clipboard::new().map_err(|e| format!("Clipboard indisponible: {e}"))?;
    let previous_clipboard = clipboard.get_text().ok();

    clipboard
        .set_text(text.to_string())
        .map_err(|e| format!("Ecriture clipboard impossible: {e}"))?;

    thread::sleep(Duration::from_millis(CLIPBOARD_SETTLE_BEFORE_PASTE_MS));

    let mut last_error: Option<String> = None;
    let mut attempts: u8 = 0;
    for _ in 0..2 {
        attempts += 1;
        match send_ctrl_v() {
            Ok(()) => {
                let restore_delay_ms = compute_clipboard_restore_delay_ms(text_len);
                restore_clipboard_later(previous_clipboard, text.to_string(), restore_delay_ms);
                return Ok(InjectionReport {
                    mode: "clipboard-paste",
                    attempts,
                    text_len,
                });
            }
            Err(e) => {
                last_error = Some(e);
                thread::sleep(Duration::from_millis(80));
            }
        }
    }

    let restore_delay_ms = compute_clipboard_restore_delay_ms(text_len);
    restore_clipboard_later(previous_clipboard, text.to_string(), restore_delay_ms);

    let reason = last_error.unwrap_or_else(|| "Erreur d'injection inconnue".to_string());
    error!(target: "inject", attempts = attempts, reason = %reason, "text injection failed");
    Err(reason)
}

pub(crate) fn compute_clipboard_restore_delay_ms(text_len: usize) -> u64 {
    let steps = (text_len / CLIPBOARD_RESTORE_STEP_CHARS) as u64;
    let delay = CLIPBOARD_RESTORE_BASE_DELAY_MS
        .saturating_add(steps.saturating_mul(CLIPBOARD_RESTORE_STEP_DELAY_MS));
    delay.min(CLIPBOARD_RESTORE_MAX_DELAY_MS)
}

pub(crate) fn restore_clipboard_later(
    previous_clipboard: Option<String>,
    injected_text: String,
    restore_delay_ms: u64,
) {
    let Some(previous) = previous_clipboard else {
        return;
    };

    thread::spawn(move || {
        thread::sleep(Duration::from_millis(restore_delay_ms));
        if let Ok(mut clipboard) = Clipboard::new() {
            // Restore only if clipboard still contains our injected payload.
            // This avoids overriding user's own copy action and reduces paste races in slow editors.
            let current = clipboard.get_text().ok();
            if current.as_deref() == Some(injected_text.as_str()) {
                let _ = clipboard.set_text(previous);
            }
        }
    });
}

pub(crate) fn send_ctrl_v() -> Result<(), String> {
    let mut enigo = Enigo::new(&EnigoSettings::default())
        .map_err(|e| format!("Injection clavier indisponible: {e}"))?;

    enigo
        .key(Key::Control, Direction::Press)
        .map_err(|e| format!("Injection Ctrl down impossible: {e}"))?;
    enigo
        .key(Key::Unicode('v'), Direction::Click)
        .map_err(|e| format!("Injection V impossible: {e}"))?;
    enigo
        .key(Key::Control, Direction::Release)
        .map_err(|e| format!("Injection Ctrl up impossible: {e}"))
}

#[cfg(test)]
mod tests {
    use super::compute_clipboard_restore_delay_ms;

    #[test]
    fn restore_delay_scales_with_text_size_and_is_capped() {
        assert_eq!(compute_clipboard_restore_delay_ms(0), 2500);
        assert_eq!(compute_clipboard_restore_delay_ms(499), 2500);
        assert_eq!(compute_clipboard_restore_delay_ms(500), 2850);
        assert_eq!(compute_clipboard_restore_delay_ms(2000), 3900);
        assert_eq!(compute_clipboard_restore_delay_ms(100_000), 10_000);
    }
}
