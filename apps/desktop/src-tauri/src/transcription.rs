use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use tracing::info;

use crate::state::{AppState, ComputeCapabilityReport, TranscriptSegment, TranscriptionResult, WhisperServerRuntime};
use crate::{apply_no_window, normalize_compute_mode};

pub(crate) struct WavAnalysis {
    pub(crate) sample_count: u64,
    pub(crate) rms: f64,
    pub(crate) peak_norm: f64,
    pub(crate) activity_ratio: f64,
}

pub(crate) fn transcribe_with_strategy(
    app_state: &AppState,
    whisper_cli_path: &Path,
    model_path: &Path,
    wav_path: &Path,
    language: &str,
    compute_mode: &str,
    keep_model_loaded: bool,
    translate_to_english: bool,
) -> Result<TranscriptionResult, String> {
    if keep_model_loaded {
        match run_transcription_via_server(
            app_state,
            whisper_cli_path,
            model_path,
            wav_path,
            language,
            compute_mode,
            translate_to_english,
        ) {
            Ok(result) => return Ok(result),
            Err(e) => {
                info!(target: "asr", reason = %e, "server mode failed, fallback to cli mode");
            }
        }
    }

    run_transcription_cli(
        whisper_cli_path,
        model_path,
        wav_path,
        language,
        compute_mode,
        translate_to_english,
    )
}

pub(crate) fn run_transcription_cli(
    whisper_cli_path: &Path,
    model_path: &Path,
    wav_path: &Path,
    language: &str,
    compute_mode: &str,
    translate_to_english: bool,
) -> Result<TranscriptionResult, String> {
    let out_dir = std::env::temp_dir().join("whisperpro_transcripts");
    fs::create_dir_all(&out_dir).map_err(|e| format!("Creation dossier transcripts impossible: {e}"))?;

    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| format!("Horodatage impossible: {e}"))?
        .as_millis();
    let out_prefix = out_dir.join(format!("transcript_{stamp}"));

    let normalized_lang = normalize_language(language);
    let normalized_compute = normalize_compute_mode(compute_mode);
    let capability = detect_compute_capability_for_cli(whisper_cli_path);
    if normalized_compute == "gpu" && !capability.gpu_available {
        return Err(format!(
            "Mode GPU indisponible avec ce whisper-cli.\n{}\nAction: utilise Auto/CPU ou installe une build whisper-cli avec backend GPU.",
            capability.details
        ));
    }
    let output = if normalized_compute == "cpu" {
        run_whisper_once(
            whisper_cli_path,
            model_path,
            wav_path,
            &normalized_lang,
            &out_prefix,
            translate_to_english,
            "cpu",
        )?
    } else if normalized_compute == "gpu" {
        run_whisper_once(
            whisper_cli_path,
            model_path,
            wav_path,
            &normalized_lang,
            &out_prefix,
            translate_to_english,
            "gpu",
        )
        .map_err(|err| {
            format!(
                "{err}\nAction: Passe le mode de calcul sur Auto ou CPU si le GPU n'est pas disponible."
            )
        })?
    } else {
        if !capability.gpu_available {
            run_whisper_once(
                whisper_cli_path,
                model_path,
                wav_path,
                &normalized_lang,
                &out_prefix,
                translate_to_english,
                "cpu",
            )?
        } else {
            match run_whisper_once(
                whisper_cli_path,
                model_path,
                wav_path,
                &normalized_lang,
                &out_prefix,
                translate_to_english,
                "gpu",
            ) {
            Ok(out) => out,
            Err(gpu_err) => {
                info!(target: "asr", error = %gpu_err, "gpu transcription failed, retrying on cpu");
                run_whisper_once(
                    whisper_cli_path,
                    model_path,
                    wav_path,
                    &normalized_lang,
                    &out_prefix,
                    translate_to_english,
                    "cpu",
                )
                .map_err(|cpu_err| {
                    format!(
                        "Echec en mode Auto.\nGPU: {gpu_err}\nCPU: {cpu_err}\nAction: Verifie le modele, la langue et le WAV dans Settings."
                    )
                })?
            }
        }
        }
    };

    let txt_path = out_prefix.with_extension("txt");
    let text = if txt_path.exists() {
        fs::read_to_string(&txt_path).map_err(|e| format!("Lecture transcript txt impossible: {e}"))?
    } else {
        String::from_utf8_lossy(&output.stdout).to_string()
    };

    let cleaned = sanitize_transcript_text(text.trim());
    let segments = if cleaned.is_empty() {
        vec![]
    } else {
        vec![TranscriptSegment {
            start_ms: 0,
            end_ms: 0,
            text: cleaned.clone(),
        }]
    };

    Ok(TranscriptionResult {
        text: cleaned,
        segments,
        model_path: model_path.to_string_lossy().to_string(),
        wav_path: wav_path.to_string_lossy().to_string(),
    })
}

pub(crate) fn run_whisper_once(
    whisper_cli_path: &Path,
    model_path: &Path,
    wav_path: &Path,
    normalized_lang: &str,
    out_prefix: &Path,
    translate_to_english: bool,
    compute_mode: &str,
) -> Result<std::process::Output, String> {
    let (supports_ngl, supports_ng) = detect_whisper_gpu_flags(whisper_cli_path);
    let mut cmd = Command::new(whisper_cli_path);
    apply_no_window(&mut cmd);
    cmd.arg("-m")
        .arg(model_path)
        .arg("-f")
        .arg(wav_path)
        .arg("-l")
        .arg(normalized_lang)
        .arg("-of")
        .arg(out_prefix)
        .arg("-otxt")
        .arg("-np")
        .arg("-nt");

    // whisper.cpp has two common CLI variants:
    // - newer: -ngl <N> (GPU layers)
    // - older: -ng / --no-gpu
    // We adapt dynamically to keep compatibility with both.
    if compute_mode == "cpu" {
        if supports_ng {
            cmd.arg("-ng");
        } else if supports_ngl {
            cmd.arg("-ngl").arg("0");
        }
    } else if compute_mode == "gpu" && supports_ngl {
        cmd.arg("-ngl").arg("999");
    }

    if translate_to_english {
        cmd.arg("-tr");
    }

    let output = cmd.output().map_err(|e| {
        format!(
            "Impossible de lancer whisper-cli.\nProbleme: {e}\nAction: Verifie le chemin whisper-cli dans Settings et relance l'application."
        )
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(format!(
            "Transcription echouee (whisper-cli code {:?}).\nstdout: {}\nstderr: {}",
            output.status.code(),
            stdout.trim(),
            stderr.trim()
        ));
    }

    Ok(output)
}

pub(crate) fn run_transcription_via_server(
    app_state: &AppState,
    whisper_cli_path: &Path,
    model_path: &Path,
    wav_path: &Path,
    language: &str,
    compute_mode: &str,
    translate_to_english: bool,
) -> Result<TranscriptionResult, String> {
    let server_exe = whisper_cli_path
        .parent()
        .map(|p| p.join("whisper-server.exe"))
        .ok_or_else(|| "Dossier bin whisper introuvable.".to_string())?;
    if !server_exe.exists() {
        return Err("whisper-server.exe introuvable.".to_string());
    }

    let normalized_lang = normalize_language(language);
    let normalized_compute = normalize_compute_mode(compute_mode);
    let capability = detect_compute_capability_for_cli(whisper_cli_path);

    let port = ensure_whisper_server_running(
        app_state,
        &server_exe,
        model_path,
        &normalized_lang,
        &normalized_compute,
        translate_to_english,
        &capability,
    )?;

    let url = format!("http://127.0.0.1:{port}/inference");
    let file_part = reqwest::blocking::multipart::Part::file(wav_path)
        .map_err(|e| format!("Lecture WAV pour serveur impossible: {e}"))?;
    let form = reqwest::blocking::multipart::Form::new()
        .text("response_format", "json")
        .part("file", file_part);
    let response = reqwest::blocking::Client::new()
        .post(url)
        .multipart(form)
        .send()
        .map_err(|e| format!("Inference serveur whisper impossible: {e}"))?;

    if !response.status().is_success() {
        return Err(format!(
            "Serveur whisper en erreur (HTTP {}).",
            response.status()
        ));
    }

    let body = response
        .text()
        .map_err(|e| format!("Lecture reponse serveur whisper impossible: {e}"))?;
    let payload: serde_json::Value =
        serde_json::from_str(&body).map_err(|e| format!("JSON serveur whisper invalide: {e}"))?;

    let text = if let Some(t) = payload.get("text").and_then(|v| v.as_str()) {
        t.to_string()
    } else if let Some(segments) = payload.get("segments").and_then(|v| v.as_array()) {
        segments
            .iter()
            .filter_map(|s| s.get("text").and_then(|v| v.as_str()))
            .collect::<Vec<_>>()
            .join(" ")
            .trim()
            .to_string()
    } else {
        String::new()
    };

    let cleaned = sanitize_transcript_text(text.trim());
    let segments = if cleaned.is_empty() {
        vec![]
    } else {
        vec![TranscriptSegment {
            start_ms: 0,
            end_ms: 0,
            text: cleaned.clone(),
        }]
    };

    Ok(TranscriptionResult {
        text: cleaned,
        segments,
        model_path: model_path.to_string_lossy().to_string(),
        wav_path: wav_path.to_string_lossy().to_string(),
    })
}

pub(crate) fn ensure_whisper_server_running(
    app_state: &AppState,
    server_exe: &Path,
    model_path: &Path,
    language: &str,
    compute_mode: &str,
    translate_to_english: bool,
    capability: &ComputeCapabilityReport,
) -> Result<u16, String> {
    let mut guard = app_state.whisper_server.lock();

    if let Some(runtime) = guard.as_mut() {
        let same_config = runtime.model_path == model_path.to_string_lossy()
            && runtime.language == language
            && runtime.compute_mode == compute_mode
            && runtime.translate_to_english == translate_to_english;
        if same_config {
            if runtime.child.try_wait().ok().flatten().is_none() {
                return Ok(runtime.port);
            }
        }
        let _ = runtime.child.kill();
        let _ = runtime.child.wait();
        *guard = None;
    }

    let port: u16 = 8178;
    let mut cmd = Command::new(server_exe);
    apply_no_window(&mut cmd);
    cmd.arg("-m")
        .arg(model_path)
        .arg("-l")
        .arg(language)
        .arg("--host")
        .arg("127.0.0.1")
        .arg("--port")
        .arg(port.to_string())
        .arg("-nt");

    if translate_to_english {
        cmd.arg("-tr");
    }
    if compute_mode == "cpu" {
        cmd.arg("-ng");
    }
    if compute_mode == "gpu" && !capability.gpu_available {
        return Err("Mode GPU indisponible pour le serveur whisper.".to_string());
    }

    let child = cmd
        .spawn()
        .map_err(|e| format!("Demarrage whisper-server impossible: {e}"))?;
    *guard = Some(WhisperServerRuntime {
        child,
        model_path: model_path.to_string_lossy().to_string(),
        language: language.to_string(),
        compute_mode: compute_mode.to_string(),
        translate_to_english,
        port,
    });

    wait_for_server_ready(port)
}

pub(crate) fn wait_for_server_ready(port: u16) -> Result<u16, String> {
    let url = format!("http://127.0.0.1:{port}/");
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_millis(250))
        .build()
        .map_err(|e| format!("Client readiness impossible: {e}"))?;

    let started = std::time::Instant::now();
    while started.elapsed() < Duration::from_secs(10) {
        if let Ok(resp) = client.get(&url).send() {
            if resp.status().is_success() || resp.status().as_u16() == 404 {
                return Ok(port);
            }
        }
        thread::sleep(Duration::from_millis(150));
    }

    Err("whisper-server n'a pas demarre a temps.".to_string())
}

pub(crate) fn detect_whisper_gpu_flags(whisper_cli_path: &Path) -> (bool, bool) {
    let mut cmd = Command::new(whisper_cli_path);
    apply_no_window(&mut cmd);
    let output = cmd.arg("--help").output();
    let Ok(output) = output else {
        return (false, false);
    };

    let mut text = String::new();
    text.push_str(&String::from_utf8_lossy(&output.stdout).to_lowercase());
    text.push('\n');
    text.push_str(&String::from_utf8_lossy(&output.stderr).to_lowercase());

    let supports_ngl = text.contains("-ngl") || text.contains("--gpu-layers");
    let supports_ng = text.contains(" -ng,") || text.contains("--no-gpu");
    (supports_ngl, supports_ng)
}

pub(crate) fn detect_compute_capability_for_cli(whisper_cli_path: &Path) -> ComputeCapabilityReport {
    let (supports_ngl, supports_no_gpu_flag) = detect_whisper_gpu_flags(whisper_cli_path);
    let gpu_backend_dlls = [
        "ggml-cuda.dll",
        "ggml-vulkan.dll",
        "ggml-hipblas.dll",
        "ggml-opencl.dll",
        "ggml-sycl.dll",
        "ggml-kompute.dll",
    ];

    let mut found_gpu_runtime: Vec<String> = Vec::new();
    if let Some(parent) = whisper_cli_path.parent() {
        for dll in gpu_backend_dlls {
            if parent.join(dll).exists() {
                found_gpu_runtime.push(dll.to_string());
            }
        }
    }

    let gpu_available = !found_gpu_runtime.is_empty() || supports_ngl;
    let details = if !found_gpu_runtime.is_empty() {
        format!("Backends detectes: {}", found_gpu_runtime.join(", "))
    } else if supports_ngl {
        "Option -ngl detectee dans whisper-cli (GPU potentiellement disponible).".to_string()
    } else {
        "Aucun backend GPU detecte (ni DLL GPU whisper.cpp, ni option -ngl).".to_string()
    };

    ComputeCapabilityReport {
        gpu_available,
        supports_ngl,
        supports_no_gpu_flag,
        whisper_cli_path: whisper_cli_path.to_string_lossy().to_string(),
        details,
    }
}

pub(crate) fn err_model_missing(model_path: &Path) -> String {
    format!(
        "Modele introuvable.\nChemin: {}\nAction: Place un modele Whisper.cpp (.bin) a ce chemin ou mets a jour \"Chemin modele Whisper\" dans Settings.",
        model_path.to_string_lossy()
    )
}

pub(crate) fn err_cli_missing(cli_path: &Path) -> String {
    format!(
        "whisper-cli introuvable.\nChemin: {}\nAction: Place whisper-cli.exe a ce chemin ou mets a jour \"Chemin whisper-cli.exe\" dans Settings.",
        cli_path.to_string_lossy()
    )
}

pub(crate) fn err_wav_missing(wav_path: &Path) -> String {
    format!(
        "Fichier WAV introuvable.\nChemin: {}\nAction: Refais un enregistrement micro puis relance la transcription.",
        wav_path.to_string_lossy()
    )
}

pub(crate) fn is_probably_silent_wav(wav_path: &Path) -> Result<bool, String> {
    let analysis = analyze_wav(wav_path)?;
    if analysis.sample_count == 0 {
        return Ok(true);
    }

    let silent =
        analysis.rms < 0.0032 && analysis.peak_norm < 0.03 && analysis.activity_ratio < 0.0015;
    Ok(silent)
}

pub(crate) fn maybe_create_normalized_wav_copy(wav_path: &Path) -> Result<Option<PathBuf>, String> {
    let analysis = analyze_wav(wav_path)?;
    if analysis.sample_count == 0 {
        return Ok(None);
    }

    // Only boost input when voice is likely too quiet.
    if analysis.peak_norm >= 0.45 && analysis.rms >= 0.045 {
        return Ok(None);
    }
    if analysis.activity_ratio < 0.001 {
        return Ok(None);
    }

    let mut reader =
        hound::WavReader::open(wav_path).map_err(|e| format!("Lecture WAV impossible: {e}"))?;
    let spec = reader.spec();
    if spec.sample_format != hound::SampleFormat::Int || spec.bits_per_sample != 16 {
        return Ok(None);
    }

    let gain_from_peak = if analysis.peak_norm > 0.0 {
        0.86 / analysis.peak_norm
    } else {
        1.0
    };
    let gain_from_rms = if analysis.rms > 0.0 {
        0.11 / analysis.rms
    } else {
        1.0
    };
    let gain = gain_from_peak.min(gain_from_rms).clamp(1.0, 8.0);
    if gain <= 1.25 {
        return Ok(None);
    }

    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| format!("Horodatage impossible: {e}"))?
        .as_millis();
    let normalized_path = std::env::temp_dir().join(format!("whisperpro_recording_norm_{stamp}.wav"));
    let mut writer = hound::WavWriter::create(&normalized_path, spec)
        .map_err(|e| format!("Ecriture WAV normalisee impossible: {e}"))?;

    for s in reader.samples::<i16>() {
        let sample = s.map_err(|e| format!("Lecture sample WAV impossible: {e}"))? as f32;
        let boosted = (sample * gain as f32).round().clamp(i16::MIN as f32, i16::MAX as f32);
        writer
            .write_sample(boosted as i16)
            .map_err(|e| format!("Ecriture sample WAV normalisee impossible: {e}"))?;
    }

    writer
        .finalize()
        .map_err(|e| format!("Finalisation WAV normalisee impossible: {e}"))?;

    Ok(Some(normalized_path))
}

pub(crate) fn analyze_wav(wav_path: &Path) -> Result<WavAnalysis, String> {
    let mut reader =
        hound::WavReader::open(wav_path).map_err(|e| format!("Lecture WAV impossible: {e}"))?;
    let spec = reader.spec();
    if spec.sample_format != hound::SampleFormat::Int || spec.bits_per_sample == 0 {
        return Ok(WavAnalysis {
            sample_count: 0,
            rms: 0.0,
            peak_norm: 0.0,
            activity_ratio: 0.0,
        });
    }

    let max_amp = ((1_i64 << (spec.bits_per_sample - 1)) - 1) as f64;
    if max_amp <= 0.0 {
        return Ok(WavAnalysis {
            sample_count: 0,
            rms: 0.0,
            peak_norm: 0.0,
            activity_ratio: 0.0,
        });
    }

    let mut sample_count: u64 = 0;
    let mut energy_sum: f64 = 0.0;
    let mut peak_norm: f64 = 0.0;
    let mut activity_count: u64 = 0;

    for s in reader.samples::<i16>() {
        let sample = s.map_err(|e| format!("Lecture sample WAV impossible: {e}"))? as f64;
        let norm = (sample.abs() / max_amp).clamp(0.0, 1.0);
        sample_count += 1;
        energy_sum += norm * norm;
        if norm > peak_norm {
            peak_norm = norm;
        }
        if norm > 0.018 {
            activity_count += 1;
        }
    }

    if sample_count == 0 {
        return Ok(WavAnalysis {
            sample_count: 0,
            rms: 0.0,
            peak_norm: 0.0,
            activity_ratio: 0.0,
        });
    }

    Ok(WavAnalysis {
        sample_count,
        rms: (energy_sum / sample_count as f64).sqrt(),
        peak_norm,
        activity_ratio: activity_count as f64 / sample_count as f64,
    })
}

pub(crate) fn normalize_language(language: &str) -> String {
    let trimmed = language.trim();
    if trimmed.is_empty() {
        return "fr".to_string();
    }

    let lower = trimmed.to_lowercase();
    if let Some((prefix, _)) = lower.split_once('-') {
        prefix.to_string()
    } else {
        lower
    }
}

fn sanitize_transcript_text(input: &str) -> String {
    let cleaned = input.trim();
    if cleaned.is_empty() {
        return String::new();
    }
    if is_known_hallucination(cleaned) {
        return String::new();
    }
    cleaned.to_string()
}

fn is_known_hallucination(input: &str) -> bool {
    let normalized = input
        .to_lowercase()
        .replace('-', " ")
        .replace('\'', " ")
        .replace('’', " ")
        .replace(':', " ")
        .replace(';', " ")
        .replace(',', " ")
        .replace('.', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");

    normalized == "sous titrage societe radio canada"
        || normalized == "sous titrage société radio canada"
}

#[cfg(test)]
mod tests {
    use super::{analyze_wav, maybe_create_normalized_wav_copy, sanitize_transcript_text};
    use std::f32::consts::PI;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static TEST_COUNTER: AtomicUsize = AtomicUsize::new(0);

    fn make_temp_dir(name: &str) -> PathBuf {
        let mut dir = std::env::temp_dir();
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time before epoch")
            .as_nanos();
        let seq = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
        dir.push(format!("whisperpro-transcription-tests-{name}-{stamp}-{seq}"));
        fs::create_dir_all(&dir).expect("create temp test dir");
        dir
    }

    fn write_sine_wav(path: &PathBuf, amplitude: f32) {
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: 16_000,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::create(path, spec).expect("create wav");
        let frequency = 440.0_f32;
        let duration_samples = 16_000;
        for i in 0..duration_samples {
            let t = i as f32 / 16_000.0;
            let sample = (2.0 * PI * frequency * t).sin() * amplitude;
            let pcm = (sample * i16::MAX as f32)
                .round()
                .clamp(i16::MIN as f32, i16::MAX as f32) as i16;
            writer.write_sample(pcm).expect("write sample");
        }
        writer.finalize().expect("finalize wav");
    }

    #[test]
    fn strips_known_radio_canada_hallucination() {
        assert_eq!(sanitize_transcript_text("Sous-titrage société radio-canada"), "");
        assert_eq!(sanitize_transcript_text("   sous titrage societe radio canada   "), "");
    }

    #[test]
    fn keeps_regular_transcript_content() {
        assert_eq!(sanitize_transcript_text("Bonjour tout le monde"), "Bonjour tout le monde");
    }

    #[test]
    fn low_level_audio_gets_normalized_copy() {
        let temp = make_temp_dir("normalize-low");
        let input = temp.join("low.wav");
        write_sine_wav(&input, 0.035);

        let before = analyze_wav(&input).expect("analyze before");
        let normalized = maybe_create_normalized_wav_copy(&input)
            .expect("normalize result")
            .expect("normalized copy path");
        let after = analyze_wav(&normalized).expect("analyze after");

        assert!(after.peak_norm > before.peak_norm);
        assert!(after.rms > before.rms);

        let _ = fs::remove_file(normalized);
        let _ = fs::remove_dir_all(temp);
    }

    #[test]
    fn healthy_audio_skips_normalization() {
        let temp = make_temp_dir("normalize-skip");
        let input = temp.join("healthy.wav");
        write_sine_wav(&input, 0.70);

        let normalized = maybe_create_normalized_wav_copy(&input).expect("normalize result");
        assert!(normalized.is_none());

        let _ = fs::remove_dir_all(temp);
    }
}


