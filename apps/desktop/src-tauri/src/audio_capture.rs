use std::path::PathBuf;
use std::sync::mpsc::Receiver;
use std::thread;
use std::time::Duration;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use parking_lot::Mutex;

use crate::state::InputDeviceInfo;

fn build_input_device_id(index: usize, name: &str) -> String {
    format!("{index}::{name}")
}

pub(crate) fn list_input_devices_impl() -> Result<Vec<InputDeviceInfo>, String> {
    let host = cpal::default_host();
    let default_name = host
        .default_input_device()
        .and_then(|device| device.name().ok());

    let devices = host
        .input_devices()
        .map_err(|e| format!("Impossible de lire les micros: {e}"))?;

    let mut output: Vec<InputDeviceInfo> = Vec::new();
    let mut default_marked = false;
    for (index, device) in devices.enumerate() {
        let name = device
            .name()
            .unwrap_or_else(|_| format!("Microphone {index}"));
        let is_default = !default_marked && default_name.as_deref() == Some(name.as_str());
        if is_default {
            default_marked = true;
        }
        output.push(InputDeviceInfo {
            id: build_input_device_id(index, &name),
            name,
            is_default,
        });
    }
    Ok(output)
}

fn resolve_input_device(
    host: &cpal::Host,
    preferred_device_id: Option<&str>,
) -> Result<cpal::Device, String> {
    let preferred = preferred_device_id
        .map(str::trim)
        .filter(|value| !value.is_empty());

    if let Some(preferred_id) = preferred {
        let devices = host
            .input_devices()
            .map_err(|e| format!("Impossible de lire les micros: {e}"))?;
        for (index, device) in devices.enumerate() {
            let name = match device.name() {
                Ok(value) => value,
                Err(_) => format!("Microphone {index}"),
            };
            let current_id = build_input_device_id(index, &name);
            if current_id == preferred_id {
                return Ok(device);
            }
        }
    }

    host.default_input_device()
        .ok_or_else(|| "Aucun micro detecte".to_string())
}

pub(crate) fn run_capture_loop(
    output_path: PathBuf,
    stop_rx: Receiver<()>,
    preferred_device_id: Option<String>,
) -> Result<String, String> {
    let host = cpal::default_host();
    let device = resolve_input_device(&host, preferred_device_id.as_deref())?;

    let config = device
        .default_input_config()
        .map_err(|e| format!("Configuration micro invalide: {e}"))?;

    let spec = hound::WavSpec {
        channels: config.channels(),
        sample_rate: config.sample_rate().0,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let writer = hound::WavWriter::create(&output_path, spec)
        .map_err(|e| format!("Impossible de creer le WAV: {e}"))?;
    let writer = std::sync::Arc::new(Mutex::new(Some(writer)));

    let writer_for_stream = std::sync::Arc::clone(&writer);
    let err_fn = |err| eprintln!("Erreur flux audio: {err}");

    let stream_config = config.clone().into();
    let stream = match config.sample_format() {
        cpal::SampleFormat::F32 => {
            build_stream_f32(&device, &stream_config, writer_for_stream, err_fn)
        }
        cpal::SampleFormat::I16 => {
            build_stream_i16(&device, &stream_config, writer_for_stream, err_fn)
        }
        cpal::SampleFormat::U16 => {
            build_stream_u16(&device, &stream_config, writer_for_stream, err_fn)
        }
        other => return Err(format!("Format audio non supporte: {other:?}")),
    }
    .map_err(|e| format!("Impossible de demarrer le flux audio: {e}"))?;

    stream
        .play()
        .map_err(|e| format!("Impossible de lire le flux audio: {e}"))?;

    loop {
        if stop_rx.try_recv().is_ok() {
            break;
        }
        thread::sleep(Duration::from_millis(30));
    }

    drop(stream);

    if let Some(writer) = writer.lock().take() {
        writer
            .finalize()
            .map_err(|e| format!("Impossible de finaliser le WAV: {e}"))?;
    }

    Ok(output_path.to_string_lossy().to_string())
}


pub(crate) fn build_stream_f32(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    writer: std::sync::Arc<Mutex<Option<hound::WavWriter<std::io::BufWriter<std::fs::File>>>>>,
    err_fn: impl FnMut(cpal::StreamError) + Send + 'static,
) -> Result<cpal::Stream, cpal::BuildStreamError> {
    device.build_input_stream(
        config,
        move |data: &[f32], _| {
            if let Some(writer) = writer.lock().as_mut() {
                for &sample in data {
                    let value = (sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
                    let _ = writer.write_sample(value);
                }
            }
        },
        err_fn,
        None,
    )
}

pub(crate) fn build_stream_i16(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    writer: std::sync::Arc<Mutex<Option<hound::WavWriter<std::io::BufWriter<std::fs::File>>>>>,
    err_fn: impl FnMut(cpal::StreamError) + Send + 'static,
) -> Result<cpal::Stream, cpal::BuildStreamError> {
    device.build_input_stream(
        config,
        move |data: &[i16], _| {
            if let Some(writer) = writer.lock().as_mut() {
                for &sample in data {
                    let _ = writer.write_sample(sample);
                }
            }
        },
        err_fn,
        None,
    )
}

pub(crate) fn build_stream_u16(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    writer: std::sync::Arc<Mutex<Option<hound::WavWriter<std::io::BufWriter<std::fs::File>>>>>,
    err_fn: impl FnMut(cpal::StreamError) + Send + 'static,
) -> Result<cpal::Stream, cpal::BuildStreamError> {
    device.build_input_stream(
        config,
        move |data: &[u16], _| {
            if let Some(writer) = writer.lock().as_mut() {
                for &sample in data {
                    let normalized = sample as i32 - 32768;
                    let value = normalized as i16;
                    let _ = writer.write_sample(value);
                }
            }
        },
        err_fn,
        None,
    )
}

