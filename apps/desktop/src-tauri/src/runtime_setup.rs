use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use tauri::{AppHandle, Manager};
use tracing::info;

use crate::models::{dedupe_paths, first_existing_path};
use crate::settings_db::{get_settings_from_db, open_db, save_settings_impl};
use crate::state::{AppState, GpuVendor};
use crate::{apply_no_window, detect_compute_capability_for_cli};

pub(crate) fn ensure_runtime_dependencies(app: &AppHandle, app_state: &AppState) -> Result<(), String> {
    let whisper_bin_dir = app_state
        .whisper_cli_default_path
        .parent()
        .ok_or_else(|| "Dossier bin whisper introuvable.".to_string())?
        .to_path_buf();
    fs::create_dir_all(&whisper_bin_dir)
        .map_err(|e| format!("Creation dossier dependances impossible: {e}"))?;

    if !app_state.whisper_cli_default_path.exists() {
        let _ = copy_runtime_from_resources(app, app_state);
    }

    let vendor = detect_gpu_vendor();
    let current_capability = if app_state.whisper_cli_default_path.exists() {
        Some(detect_compute_capability_for_cli(
            &app_state.whisper_cli_default_path,
        ))
    } else {
        None
    };

    let needs_runtime = !app_state.whisper_cli_default_path.exists();
    let needs_nvidia_upgrade = matches!(vendor, GpuVendor::Nvidia)
        && current_capability
            .as_ref()
            .map(|c| !c.gpu_available)
            .unwrap_or(true);

    if needs_runtime || needs_nvidia_upgrade {
        let reason = if needs_nvidia_upgrade {
            "nvidia-gpu-detected"
        } else {
            "missing-runtime"
        };
        let install_note = install_runtime_from_official_release(vendor, &whisper_bin_dir)?;
        info!(target: "bootstrap", reason = reason, note = %install_note, "runtime dependency installed from official release");
    }

    let mut settings = get_settings_from_db(
        &app_state.db_path,
        &app_state.model_default_path,
        &app_state.whisper_cli_default_path,
    )?;
    if settings.whisper_cli_path.trim().is_empty() || !Path::new(&settings.whisper_cli_path).exists() {
        settings.whisper_cli_path = app_state
            .whisper_cli_default_path
            .to_string_lossy()
            .to_string();
        let conn = open_db(&app_state.db_path)?;
        save_settings_impl(&conn, &settings)?;
    }

    Ok(())
}

pub(crate) fn copy_runtime_from_resources(app: &AppHandle, app_state: &AppState) -> Result<(), String> {
    let mut candidates: Vec<PathBuf> = Vec::new();

    if let Ok(resource_dir) = app.path().resource_dir() {
        candidates.push(resource_dir.join("bin").join("whisper-cli.exe"));
        candidates.push(resource_dir.join("whisper-cli.exe"));
    }

    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            candidates.push(exe_dir.join("whisper-cli.exe"));
            candidates.push(exe_dir.join("resources").join("bin").join("whisper-cli.exe"));
        }
    }

    if let Ok(cwd) = std::env::current_dir() {
        candidates.push(cwd.join("whisper-cli.exe"));
        candidates.push(cwd.join("src-tauri").join("resources").join("bin").join("whisper-cli.exe"));
        candidates.push(
            cwd.join("apps")
                .join("desktop")
                .join("src-tauri")
                .join("resources")
                .join("bin")
                .join("whisper-cli.exe"),
        );
    }

    let source = first_existing_path(&dedupe_paths(candidates)).ok_or_else(|| {
        "Dependance absente: whisper-cli.exe non trouve dans les ressources de l'application."
            .to_string()
    })?;

    fs::copy(&source, &app_state.whisper_cli_default_path).map_err(|e| {
        format!(
            "Copie de whisper-cli impossible depuis {}: {e}",
            source.to_string_lossy()
        )
    })?;
    Ok(())
}

pub(crate) fn detect_gpu_vendor() -> GpuVendor {
    let mut nvidia = Command::new("nvidia-smi");
    apply_no_window(&mut nvidia);
    if let Ok(output) = nvidia.output() {
        if output.status.success() {
            return GpuVendor::Nvidia;
        }
    }

    let mut combined = String::new();
    let mut wmic = Command::new("cmd");
    wmic.args(["/C", "wmic path win32_VideoController get name"]);
    apply_no_window(&mut wmic);
    if let Ok(output) = wmic.output() {
        combined.push_str(&String::from_utf8_lossy(&output.stdout).to_lowercase());
        combined.push_str(&String::from_utf8_lossy(&output.stderr).to_lowercase());
    }
    if combined.is_empty() {
        let mut ps = Command::new("powershell");
        ps.args([
            "-NoProfile",
            "-Command",
            "(Get-CimInstance Win32_VideoController | Select-Object -ExpandProperty Name) -join \"`n\"",
        ]);
        apply_no_window(&mut ps);
        if let Ok(output) = ps.output() {
            combined.push_str(&String::from_utf8_lossy(&output.stdout).to_lowercase());
            combined.push_str(&String::from_utf8_lossy(&output.stderr).to_lowercase());
        }
    }

    if combined.contains("nvidia") {
        GpuVendor::Nvidia
    } else if combined.contains("amd") || combined.contains("radeon") {
        GpuVendor::Amd
    } else if combined.contains("intel") {
        GpuVendor::Intel
    } else {
        GpuVendor::Unknown
    }
}

pub(crate) fn install_runtime_from_official_release(vendor: GpuVendor, install_dir: &Path) -> Result<String, String> {
    let assets = fetch_whisper_release_assets()?;
    let preferred_asset = match vendor {
        GpuVendor::Nvidia => assets
            .cublas_12_4_x64
            .as_deref()
            .or(assets.cublas_11_8_x64.as_deref())
            .or(assets.cpu_x64.as_deref()),
        GpuVendor::Amd | GpuVendor::Intel | GpuVendor::Unknown => assets.cpu_x64.as_deref(),
    }
    .ok_or_else(|| "Aucun runtime Windows x64 compatible trouve dans la release whisper.cpp.".to_string())?;

    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| format!("Horodatage impossible: {e}"))?
        .as_millis();
    let zip_path = install_dir.join(format!("runtime-{stamp}.zip"));
    let mut response = reqwest::blocking::Client::new()
        .get(preferred_asset)
        .header("User-Agent", "WhisperPro/1.0")
        .send()
        .map_err(|e| format!("Telechargement runtime whisper impossible: {e}"))?;
    if !response.status().is_success() {
        return Err(format!(
            "Telechargement runtime whisper echoue (HTTP {}).",
            response.status()
        ));
    }
    let mut zip_file = fs::File::create(&zip_path)
        .map_err(|e| format!("Creation archive runtime impossible: {e}"))?;
    std::io::copy(&mut response, &mut zip_file)
        .map_err(|e| format!("Ecriture archive runtime impossible: {e}"))?;

    let file = fs::File::open(&zip_path).map_err(|e| format!("Lecture archive runtime impossible: {e}"))?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| format!("Archive runtime invalide: {e}"))?;

    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| format!("Lecture entree archive impossible: {e}"))?;
        if entry.is_dir() {
            continue;
        }
        let entry_name = entry.name().replace('\\', "/");
        let lower = entry_name.to_lowercase();
        if !(lower.ends_with(".exe") || lower.ends_with(".dll")) {
            continue;
        }

        let Some(file_name) = Path::new(&entry_name).file_name() else {
            continue;
        };
        let target = install_dir.join(file_name);
        let mut out = fs::File::create(&target)
            .map_err(|e| format!("Creation fichier runtime impossible ({}): {e}", target.to_string_lossy()))?;
        std::io::copy(&mut entry, &mut out)
            .map_err(|e| format!("Ecriture fichier runtime impossible ({}): {e}", target.to_string_lossy()))?;
    }

    let _ = fs::remove_file(&zip_path);

    let cli = install_dir.join("whisper-cli.exe");
    if !cli.exists() {
        return Err("Installation runtime incomplete: whisper-cli.exe introuvable apres extraction.".to_string());
    }

    Ok(format!("Runtime installe depuis {}", assets.tag))
}

pub(crate) struct WhisperReleaseAssets {
    tag: String,
    cpu_x64: Option<String>,
    cublas_11_8_x64: Option<String>,
    cublas_12_4_x64: Option<String>,
}

pub(crate) fn fetch_whisper_release_assets() -> Result<WhisperReleaseAssets, String> {
    let response = reqwest::blocking::Client::new()
        .get("https://api.github.com/repos/ggml-org/whisper.cpp/releases/latest")
        .header("User-Agent", "WhisperPro/1.0")
        .send()
        .map_err(|e| format!("Lecture release whisper.cpp impossible: {e}"))?;
    if !response.status().is_success() {
        return Err(format!(
            "Lecture release whisper.cpp echouee (HTTP {}).",
            response.status()
        ));
    }

    let body = response
        .text()
        .map_err(|e| format!("Lecture reponse release whisper.cpp impossible: {e}"))?;
    let payload: serde_json::Value = serde_json::from_str(&body)
        .map_err(|e| format!("Reponse release whisper.cpp invalide: {e}"))?;
    let tag = payload
        .get("tag_name")
        .and_then(|v| v.as_str())
        .unwrap_or("latest")
        .to_string();

    let mut cpu_x64: Option<String> = None;
    let mut cublas_11_8_x64: Option<String> = None;
    let mut cublas_12_4_x64: Option<String> = None;

    if let Some(assets) = payload.get("assets").and_then(|v| v.as_array()) {
        for asset in assets {
            let name = asset
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_lowercase();
            let url = asset
                .get("browser_download_url")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            if url.is_empty() {
                continue;
            }

            if name == "whisper-bin-x64.zip" {
                cpu_x64 = Some(url.clone());
            } else if name == "whisper-cublas-11.8.0-bin-x64.zip" {
                cublas_11_8_x64 = Some(url.clone());
            } else if name == "whisper-cublas-12.4.0-bin-x64.zip" {
                cublas_12_4_x64 = Some(url.clone());
            }
        }
    }

    Ok(WhisperReleaseAssets {
        tag,
        cpu_x64,
        cublas_11_8_x64,
        cublas_12_4_x64,
    })
}

