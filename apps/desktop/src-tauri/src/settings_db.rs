use std::fs;
use std::path::{Path, PathBuf};

use rusqlite::{params, Connection};

use crate::state::UserSettings;

pub(crate) fn open_db(db_path: &PathBuf) -> Result<Connection, String> {
    Connection::open(db_path).map_err(|e| format!("Ouverture DB impossible: {e}"))
}

pub(crate) fn get_settings_from_db(
    db_path: &PathBuf,
    default_model_path: &Path,
    default_whisper_cli_path: &Path,
) -> Result<UserSettings, String> {
    let conn = open_db(db_path)?;
    let fallback_model_path = default_model_path.to_string_lossy().to_string();
    let fallback_whisper_cli_path = default_whisper_cli_path.to_string_lossy().to_string();

    let mut stmt = conn
        .prepare("SELECT language, translation_target, shortcut, model_path, whisper_cli_path, compute_mode, keep_model_loaded, widget_enabled, widget_autohide, voice_commands_enabled, onboarding_completed, widget_opacity, widget_pop_sound_volume, widget_pop_sound FROM settings WHERE id = 1")
        .map_err(|e| format!("Lecture settings impossible: {e}"))?;

    let mut rows = stmt
        .query([])
        .map_err(|e| format!("Lecture settings impossible: {e}"))?;

    if let Some(row) = rows
        .next()
        .map_err(|e| format!("Lecture settings impossible: {e}"))?
    {
        let model_path_from_db: String = row
            .get::<_, String>(3)
            .map_err(|e| format!("Lecture model_path impossible: {e}"))?;
        let whisper_cli_from_db: String = row
            .get::<_, String>(4)
            .map_err(|e| format!("Lecture whisper_cli_path impossible: {e}"))?;
        let compute_mode_from_db: String = row
            .get::<_, String>(5)
            .map_err(|e| format!("Lecture compute_mode impossible: {e}"))?;
        let keep_model_loaded: i64 = row
            .get::<_, i64>(6)
            .map_err(|e| format!("Lecture keep_model_loaded impossible: {e}"))?;
        let widget_enabled: i64 = row
            .get::<_, i64>(7)
            .map_err(|e| format!("Lecture widget_enabled impossible: {e}"))?;
        let widget_autohide: i64 = row
            .get::<_, i64>(8)
            .map_err(|e| format!("Lecture widget_autohide impossible: {e}"))?;
        let voice_commands_enabled: i64 = row
            .get::<_, i64>(9)
            .map_err(|e| format!("Lecture voice_commands_enabled impossible: {e}"))?;
        let onboarding_completed: i64 = row
            .get::<_, i64>(10)
            .map_err(|e| format!("Lecture onboarding_completed impossible: {e}"))?;
        let widget_opacity: f64 = row
            .get::<_, f64>(11)
            .map_err(|e| format!("Lecture widget_opacity impossible: {e}"))?;
        let widget_pop_sound_volume: f64 = row
            .get::<_, f64>(12)
            .map_err(|e| format!("Lecture widget_pop_sound_volume impossible: {e}"))?;
        let widget_pop_sound: String = row
            .get::<_, String>(13)
            .map_err(|e| format!("Lecture widget_pop_sound impossible: {e}"))?;

        let selected_model_path = resolve_active_model_path(&model_path_from_db, &fallback_model_path);
        let selected_cli_path = if whisper_cli_from_db.trim().is_empty() {
            fallback_whisper_cli_path.clone()
        } else {
            whisper_cli_from_db.clone()
        };

        let settings = UserSettings {
            language: row
                .get::<_, String>(0)
                .map_err(|e| format!("Lecture language impossible: {e}"))?,
            translation_target: super::normalize_translation_target(
                &row.get::<_, String>(1)
                    .map_err(|e| format!("Lecture translation_target impossible: {e}"))?,
            ),
            shortcut: row
                .get::<_, String>(2)
                .map_err(|e| format!("Lecture shortcut impossible: {e}"))?,
            model_path: selected_model_path.clone(),
            whisper_cli_path: selected_cli_path.clone(),
            compute_mode: super::normalize_compute_mode(&compute_mode_from_db),
            keep_model_loaded: keep_model_loaded != 0,
            widget_enabled: widget_enabled != 0,
            widget_autohide: widget_autohide != 0,
            widget_opacity: super::clamp_widget_opacity(widget_opacity as f32),
            widget_pop_sound_volume: super::normalize_widget_pop_sound_volume_from_db(
                widget_pop_sound_volume as f32,
            ),
            widget_pop_sound: super::normalize_widget_pop_sound(&widget_pop_sound),
            voice_commands_enabled: voice_commands_enabled != 0,
            onboarding_completed: onboarding_completed != 0,
        };

        if settings.model_path != model_path_from_db || settings.whisper_cli_path != whisper_cli_from_db {
            save_settings_impl(&conn, &settings)?;
        }
        return Ok(settings);
    }

    let defaults = UserSettings::with_defaults(fallback_model_path, fallback_whisper_cli_path);
    save_settings_impl(&conn, &defaults)?;
    Ok(defaults)
}

pub(crate) fn save_settings_impl(conn: &Connection, settings: &UserSettings) -> Result<(), String> {
    conn.execute(
        "INSERT INTO settings (id, language, translation_target, shortcut, model_path, whisper_cli_path, compute_mode, keep_model_loaded, widget_enabled, widget_autohide, voice_commands_enabled, onboarding_completed, widget_opacity, widget_pop_sound_volume, widget_pop_sound) VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
         ON CONFLICT(id) DO UPDATE SET language = excluded.language, translation_target = excluded.translation_target, shortcut = excluded.shortcut, model_path = excluded.model_path, whisper_cli_path = excluded.whisper_cli_path, compute_mode = excluded.compute_mode, keep_model_loaded = excluded.keep_model_loaded, widget_enabled = excluded.widget_enabled, widget_autohide = excluded.widget_autohide, voice_commands_enabled = excluded.voice_commands_enabled, onboarding_completed = excluded.onboarding_completed, widget_opacity = excluded.widget_opacity, widget_pop_sound_volume = excluded.widget_pop_sound_volume, widget_pop_sound = excluded.widget_pop_sound",
        params![
            settings.language,
            super::normalize_translation_target(&settings.translation_target),
            settings.shortcut,
            settings.model_path,
            settings.whisper_cli_path,
            super::normalize_compute_mode(&settings.compute_mode),
            if settings.keep_model_loaded { 1 } else { 0 },
            if settings.widget_enabled { 1 } else { 0 },
            if settings.widget_autohide { 1 } else { 0 },
            if settings.voice_commands_enabled { 1 } else { 0 },
            if settings.onboarding_completed { 1 } else { 0 },
            super::clamp_widget_opacity(settings.widget_opacity),
            super::clamp_widget_pop_sound_volume(settings.widget_pop_sound_volume),
            super::normalize_widget_pop_sound(&settings.widget_pop_sound)
        ],
    )
    .map_err(|e| format!("Sauvegarde settings impossible: {e}"))?;

    Ok(())
}

pub(crate) fn init_db(
    db_path: &PathBuf,
    default_model_path: &Path,
    default_whisper_cli_path: &Path,
) -> Result<(), String> {
    let conn = Connection::open(db_path).map_err(|e| format!("Init DB impossible: {e}"))?;

    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS settings (
            id INTEGER PRIMARY KEY CHECK(id = 1),
            language TEXT NOT NULL,
            shortcut TEXT NOT NULL
        );",
    )
    .map_err(|e| format!("Migration settings impossible: {e}"))?;

    let _ = conn.execute("ALTER TABLE settings ADD COLUMN model_path TEXT NOT NULL DEFAULT ''", []);
    let _ = conn.execute("ALTER TABLE settings ADD COLUMN whisper_cli_path TEXT NOT NULL DEFAULT ''", []);
    let _ = conn.execute("ALTER TABLE settings ADD COLUMN translation_target TEXT NOT NULL DEFAULT 'none'", []);
    let _ = conn.execute("ALTER TABLE settings ADD COLUMN compute_mode TEXT NOT NULL DEFAULT 'auto'", []);
    let _ = conn.execute("ALTER TABLE settings ADD COLUMN keep_model_loaded INTEGER NOT NULL DEFAULT 0", []);
    let _ = conn.execute("ALTER TABLE settings ADD COLUMN widget_enabled INTEGER NOT NULL DEFAULT 1", []);
    let _ = conn.execute("ALTER TABLE settings ADD COLUMN widget_autohide INTEGER NOT NULL DEFAULT 1", []);
    let _ = conn.execute("ALTER TABLE settings ADD COLUMN voice_commands_enabled INTEGER NOT NULL DEFAULT 1", []);
    let _ = conn.execute("ALTER TABLE settings ADD COLUMN onboarding_completed INTEGER NOT NULL DEFAULT 0", []);
    let _ = conn.execute("ALTER TABLE settings ADD COLUMN widget_opacity REAL NOT NULL DEFAULT 0.9", []);
    let _ = conn.execute("ALTER TABLE settings ADD COLUMN widget_pop_sound_volume REAL NOT NULL DEFAULT 0.65", []);
    let _ = conn.execute("ALTER TABLE settings ADD COLUMN widget_pop_sound TEXT NOT NULL DEFAULT 'sound1.mp3'", []);

    let existing_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM settings WHERE id = 1", [], |row| row.get(0))
        .map_err(|e| format!("Verification settings impossible: {e}"))?;
    if existing_count == 0 {
        let defaults = UserSettings::with_defaults(
            default_model_path.to_string_lossy().to_string(),
            default_whisper_cli_path.to_string_lossy().to_string(),
        );
        save_settings_impl(&conn, &defaults)?;
    }
    Ok(())
}

pub(crate) fn resolve_active_model_path(model_path_from_db: &str, fallback_model_path: &str) -> String {
    let configured = model_path_from_db.trim();
    if !configured.is_empty() && Path::new(configured).exists() {
        return configured.to_string();
    }

    if Path::new(fallback_model_path).exists() {
        return fallback_model_path.to_string();
    }

    if let Some(model_dir) = Path::new(fallback_model_path).parent() {
        for entry in super::MODEL_CATALOG {
            let candidate = model_dir.join(entry.filename);
            if candidate.exists() {
                return candidate.to_string_lossy().to_string();
            }
        }

        if let Ok(read_dir) = fs::read_dir(model_dir) {
            for entry in read_dir.flatten() {
                let path = entry.path();
                let is_bin = path
                    .extension()
                    .map(|ext| ext.to_string_lossy().to_lowercase() == "bin")
                    .unwrap_or(false);
                if is_bin {
                    return path.to_string_lossy().to_string();
                }
            }
        }
    }

    if !configured.is_empty() {
        configured.to_string()
    } else {
        fallback_model_path.to_string()
    }
}
