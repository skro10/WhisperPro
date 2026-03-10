use std::collections::HashSet;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use tauri::{AppHandle, Emitter};
use tracing::error;

use crate::state::{AppState, ModelCatalogEntry, ModelDownloadProgressEvent, ModelInfo, UserSettings};
use crate::settings_db::{get_settings_from_db, open_db, save_settings_impl};

pub(crate) const MODEL_CATALOG: [ModelCatalogEntry; 5] = [
    ModelCatalogEntry {
        id: "tiny",
        label: "Tiny",
        filename: "ggml-tiny.bin",
        download_url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.bin",
    },
    ModelCatalogEntry {
        id: "base",
        label: "Base",
        filename: "ggml-base.bin",
        download_url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.bin",
    },
    ModelCatalogEntry {
        id: "small",
        label: "Small",
        filename: "ggml-small.bin",
        download_url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.bin",
    },
    ModelCatalogEntry {
        id: "medium",
        label: "Medium",
        filename: "ggml-medium.bin",
        download_url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-medium.bin",
    },
    ModelCatalogEntry {
        id: "large-v3",
        label: "Large v3",
        filename: "ggml-large-v3.bin",
        download_url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3.bin",
    },
];

pub(crate) fn model_catalog_entry(model_id: &str) -> Option<ModelCatalogEntry> {
    MODEL_CATALOG.iter().find(|m| m.id == model_id).copied()
}

pub(crate) fn models_dir(app_state: &AppState) -> Result<PathBuf, String> {
    app_state
        .model_default_path
        .parent()
        .map(|p| p.to_path_buf())
        .ok_or_else(|| "Dossier modeles introuvable".to_string())
}

pub(crate) fn list_models_impl(app_state: &AppState, settings: &UserSettings) -> Result<Vec<ModelInfo>, String> {
    let model_dir = models_dir(app_state)?;
    fs::create_dir_all(&model_dir).map_err(|e| format!("Creation dossier modeles impossible: {e}"))?;

    let active_path = PathBuf::from(settings.model_path.clone());
    let active_name = active_path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default()
        .to_lowercase();

    let models = MODEL_CATALOG
        .iter()
        .map(|entry| {
            let path = model_dir.join(entry.filename);
            let installed = path.exists();
            let size_bytes = if installed {
                fs::metadata(&path).ok().map(|m| m.len())
            } else {
                None
            };
            let active = active_name == entry.filename.to_lowercase();

            ModelInfo {
                id: entry.id.to_string(),
                label: entry.label.to_string(),
                filename: entry.filename.to_string(),
                installed,
                active,
                size_bytes,
            }
        })
        .collect();

    Ok(models)
}

pub(crate) fn set_active_model_impl(app_state: &AppState, model_id: &str) -> Result<String, String> {
    let entry = model_catalog_entry(model_id)
        .ok_or_else(|| format!("Modele inconnu: {model_id}"))?;
    let model_dir = models_dir(app_state)?;
    let target = model_dir.join(entry.filename);
    if !target.exists() {
        return Err(format!(
            "Modele non installe: {}. Telecharge-le d'abord.",
            entry.label
        ));
    }

    let mut settings = get_settings_from_db(
        &app_state.db_path,
        &app_state.model_default_path,
        &app_state.whisper_cli_default_path,
    )?;
    settings.model_path = target.to_string_lossy().to_string();
    let conn = open_db(&app_state.db_path)?;
    save_settings_impl(&conn, &settings)?;
    Ok(format!("Modele actif: {}", entry.label))
}

pub(crate) fn emit_model_download_progress(app: &AppHandle, payload: ModelDownloadProgressEvent) {
    if let Err(e) = app.emit("model-download-progress", payload) {
        error!(target: "models", reason = %e, "broadcast model-download-progress failed");
    }
}

pub(crate) fn models_dir_from_default_path(model_default_path: &Path) -> Result<PathBuf, String> {
    model_default_path
        .parent()
        .map(|p| p.to_path_buf())
        .ok_or_else(|| "Dossier modeles introuvable".to_string())
}

pub(crate) fn set_active_model_with_paths(
    db_path: &Path,
    model_default_path: &Path,
    whisper_cli_default_path: &Path,
    model_id: &str,
) -> Result<String, String> {
    let entry = model_catalog_entry(model_id).ok_or_else(|| format!("Modele inconnu: {model_id}"))?;
    let model_dir = models_dir_from_default_path(model_default_path)?;
    let target = model_dir.join(entry.filename);
    if !target.exists() {
        return Err(format!(
            "Modele non installe: {}. Telecharge-le d'abord.",
            entry.label
        ));
    }

    let mut settings = get_settings_from_db(
        &db_path.to_path_buf(),
        &model_default_path.to_path_buf(),
        &whisper_cli_default_path.to_path_buf(),
    )?;
    settings.model_path = target.to_string_lossy().to_string();
    let conn = open_db(&db_path.to_path_buf())?;
    save_settings_impl(&conn, &settings)?;
    Ok(format!("Modele actif: {}", entry.label))
}

pub(crate) fn download_model_with_paths(
    app: &AppHandle,
    db_path: PathBuf,
    model_default_path: PathBuf,
    whisper_cli_default_path: PathBuf,
    model_id: String,
    download_cancel: Arc<AtomicBool>,
) -> Result<String, String> {
    let entry = model_catalog_entry(&model_id)
        .ok_or_else(|| format!("Modele inconnu: {}", model_id))?;
    emit_model_download_progress(
        app,
        ModelDownloadProgressEvent {
            model_id: model_id.clone(),
            status: "starting".to_string(),
            progress_pct: Some(0),
            downloaded_bytes: 0,
            total_bytes: None,
            message: format!("Preparation du telechargement: {}", entry.label),
        },
    );
    let model_dir = models_dir_from_default_path(&model_default_path)?;
    fs::create_dir_all(&model_dir).map_err(|e| format!("Creation dossier modeles impossible: {e}"))?;

    let target = model_dir.join(entry.filename);

    if !target.exists() {
        let mut response = reqwest::blocking::get(entry.download_url)
            .map_err(|e| format!("Telechargement impossible: {e}"))?;
        if !response.status().is_success() {
            return Err(format!(
                "Telechargement echoue: code HTTP {}",
                response.status()
            ));
        }
        let total_bytes = response.content_length();

        let tmp = target.with_extension("download");
        let mut file = fs::File::create(&tmp).map_err(|e| format!("Creation fichier modele impossible: {e}"))?;

        let mut downloaded_bytes: u64 = 0;
        let mut last_pct: u8 = 0;
        let mut buffer = [0_u8; 64 * 1024];

        loop {
            if download_cancel.load(Ordering::SeqCst) {
                let _ = fs::remove_file(&tmp);
                return Err("Telechargement annule".to_string());
            }
            let read = response
                .read(&mut buffer)
                .map_err(|e| format!("Lecture flux telechargement impossible: {e}"))?;
            if read == 0 {
                break;
            }

            file.write_all(&buffer[..read])
                .map_err(|e| format!("Ecriture modele impossible: {e}"))?;
            downloaded_bytes += read as u64;

            let next_pct = total_bytes
                .map(|total| ((downloaded_bytes.saturating_mul(100)) / total.max(1)) as u8)
                .unwrap_or(0);
            if next_pct != last_pct {
                last_pct = next_pct;
                emit_model_download_progress(
                    app,
                    ModelDownloadProgressEvent {
                        model_id: model_id.clone(),
                        status: "downloading".to_string(),
                        progress_pct: Some(next_pct.min(99)),
                        downloaded_bytes,
                        total_bytes,
                        message: format!("Telechargement {}%", next_pct.min(99)),
                    },
                );
            }
        }

        fs::rename(&tmp, &target)
            .map_err(|e| format!("Finalisation modele impossible: {e}"))?;
        emit_model_download_progress(
            app,
            ModelDownloadProgressEvent {
                model_id: model_id.clone(),
                status: "downloading".to_string(),
                progress_pct: Some(100),
                downloaded_bytes: total_bytes.unwrap_or(0),
                total_bytes,
                message: "Telechargement termine".to_string(),
            },
        );
    }

    let settings = get_settings_from_db(
        &db_path,
        &model_default_path,
        &whisper_cli_default_path,
    )?;
    let installed_count = MODEL_CATALOG
        .iter()
        .filter(|m| model_dir.join(m.filename).exists())
        .count();
    let active_exists = {
        let configured = settings.model_path.trim();
        !configured.is_empty() && Path::new(configured).exists()
    };
    let should_auto_activate = installed_count <= 1 || !active_exists;

    if should_auto_activate {
        set_active_model_with_paths(&db_path, &model_default_path, &whisper_cli_default_path, &model_id)?;
    }

    emit_model_download_progress(
        app,
        ModelDownloadProgressEvent {
            model_id: model_id.clone(),
            status: "done".to_string(),
            progress_pct: Some(100),
            downloaded_bytes: 0,
            total_bytes: None,
            message: format!("Modele pret: {}", entry.label),
        },
    );
    if should_auto_activate {
        Ok(format!("Modele pret et actif: {}", entry.label))
    } else {
        Ok(format!("Modele telecharge: {} (modele actif conserve)", entry.label))
    }
}

pub(crate) fn delete_model_impl(app_state: &AppState, model_id: &str) -> Result<String, String> {
    let entry = model_catalog_entry(model_id)
        .ok_or_else(|| format!("Modele inconnu: {model_id}"))?;
    let model_dir = models_dir(app_state)?;
    let target = model_dir.join(entry.filename);
    if target.exists() {
        fs::remove_file(&target).map_err(|e| format!("Suppression modele impossible: {e}"))?;
    }

    let mut settings = get_settings_from_db(
        &app_state.db_path,
        &app_state.model_default_path,
        &app_state.whisper_cli_default_path,
    )?;
    let active_name = PathBuf::from(settings.model_path.clone())
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default()
        .to_lowercase();
    if active_name == entry.filename.to_lowercase() {
        if let Some(next) = MODEL_CATALOG.iter().find(|m| model_dir.join(m.filename).exists()) {
            settings.model_path = model_dir.join(next.filename).to_string_lossy().to_string();
        } else {
            settings.model_path = app_state.model_default_path.to_string_lossy().to_string();
        }
        let conn = open_db(&app_state.db_path)?;
        save_settings_impl(&conn, &settings)?;
    }

    Ok(format!("Modele supprime: {}", entry.label))
}

pub(crate) fn candidate_model_paths(app_state: &AppState, settings: &UserSettings) -> Vec<PathBuf> {
    let mut paths = Vec::new();

    if !settings.model_path.trim().is_empty() {
        paths.push(PathBuf::from(settings.model_path.clone()));
    }

    paths.push(app_state.model_default_path.clone());

    if let Some(model_dir) = app_state.model_default_path.parent() {
        for name in ["ggml-base.bin", "ggml-small.bin", "ggml-tiny.bin", "ggml-medium.bin"] {
            paths.push(model_dir.join(name));
        }
    }

    if let Ok(cwd) = std::env::current_dir() {
        for name in ["ggml-base.bin", "ggml-small.bin", "ggml-tiny.bin"] {
            paths.push(cwd.join(name));
        }
    }

    dedupe_paths(paths)
}

pub(crate) fn candidate_cli_paths(app_state: &AppState, settings: &UserSettings) -> Vec<PathBuf> {
    let mut paths = Vec::new();

    if !settings.whisper_cli_path.trim().is_empty() {
        paths.push(PathBuf::from(settings.whisper_cli_path.clone()));
    }

    paths.push(app_state.whisper_cli_default_path.clone());

    if let Ok(cwd) = std::env::current_dir() {
        paths.push(cwd.join("whisper-cli.exe"));
        paths.push(cwd.join("build").join("bin").join("Release").join("whisper-cli.exe"));
        paths.push(cwd.join("whisper.cpp").join("build").join("bin").join("Release").join("whisper-cli.exe"));
    }

    dedupe_paths(paths)
}

pub(crate) fn dedupe_paths(paths: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();

    for p in paths {
        let key = p.to_string_lossy().to_string().to_lowercase();
        if seen.insert(key) {
            out.push(p);
        }
    }

    out
}

pub(crate) fn first_existing_path(paths: &[PathBuf]) -> Option<PathBuf> {
    paths.iter().find(|p| p.exists()).cloned()
}

