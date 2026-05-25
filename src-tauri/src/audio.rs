// Audio capture module.
//
// cpal `Stream` is `!Send` on Windows (WASAPI uses thread-affine COM objects),
// so we can't store it in a `tauri::State` shared across threads. Instead,
// we run a dedicated worker thread that owns the stream and is driven via channels.

use std::path::Path;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;

use anyhow::{anyhow, Context, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, StreamConfig};
use hound::{SampleFormat as HoundSampleFormat, WavSpec, WavWriter};

pub const TARGET_SAMPLE_RATE: u32 = 16_000;

enum Cmd {
    Start { device: Option<String> },
    Stop,
}

enum Reply {
    Started,
    Stopped {
        samples: Vec<f32>,
        source_rate: u32,
        source_channels: u16,
    },
    Error(String),
}

/// Owns a worker thread that holds a cpal `Stream`. Communicates via channels.
pub struct Recorder {
    cmd_tx: Sender<Cmd>,
    rep_rx: Mutex<Receiver<Reply>>,
}

impl Recorder {
    pub fn spawn() -> Self {
        let (cmd_tx, cmd_rx) = channel();
        let (rep_tx, rep_rx) = channel();
        thread::Builder::new()
            .name("director-general-audio".into())
            .spawn(move || worker_loop(cmd_rx, rep_tx))
            .expect("failed to spawn audio worker thread");
        Self {
            cmd_tx,
            rep_rx: Mutex::new(rep_rx),
        }
    }

    pub fn start(&self, device: Option<String>) -> Result<()> {
        self.cmd_tx
            .send(Cmd::Start { device })
            .map_err(|_| anyhow!("audio worker died"))?;
        let rx = self.rep_rx.lock().unwrap();
        match rx.recv().map_err(|_| anyhow!("audio worker died"))? {
            Reply::Started => Ok(()),
            Reply::Error(e) => Err(anyhow!(e)),
            _ => Err(anyhow!("unexpected reply during start")),
        }
    }

    pub fn stop(&self) -> Result<RecordingResult> {
        self.cmd_tx
            .send(Cmd::Stop)
            .map_err(|_| anyhow!("audio worker died"))?;
        let rx = self.rep_rx.lock().unwrap();
        match rx.recv().map_err(|_| anyhow!("audio worker died"))? {
            Reply::Stopped {
                samples,
                source_rate,
                source_channels,
            } => Ok(RecordingResult {
                samples,
                source_rate,
                source_channels,
            }),
            Reply::Error(e) => Err(anyhow!(e)),
            _ => Err(anyhow!("unexpected reply during stop")),
        }
    }
}

pub struct RecordingResult {
    pub samples: Vec<f32>,
    pub source_rate: u32,
    pub source_channels: u16,
}

/// Enumerate input devices on the default host. Returns display names.
pub fn list_input_devices() -> Vec<String> {
    let host = cpal::default_host();
    let mut names = Vec::new();
    if let Ok(devs) = host.input_devices() {
        for d in devs {
            if let Ok(n) = d.name() {
                names.push(n);
            }
        }
    }
    names.sort();
    names.dedup();
    names
}

fn worker_loop(cmd_rx: Receiver<Cmd>, rep_tx: Sender<Reply>) {
    let host = cpal::default_host();

    while let Ok(cmd) = cmd_rx.recv() {
        match cmd {
            Cmd::Start { device } => {
                if let Err(e) = handle_start_session(&host, device.as_deref(), &cmd_rx, &rep_tx) {
                    let _ = rep_tx.send(Reply::Error(format!("{e:#}")));
                }
            }
            Cmd::Stop => {
                // Stop without active session — ignore.
            }
        }
    }
}

fn pick_device(host: &cpal::Host, preferred: Option<&str>) -> Result<cpal::Device> {
    if let Some(name) = preferred {
        if let Ok(devs) = host.input_devices() {
            for d in devs {
                if let Ok(n) = d.name() {
                    if n == name {
                        return Ok(d);
                    }
                }
            }
        }
        eprintln!(
            "[audio] preferred device '{name}' not found, falling back to default input"
        );
    }
    host.default_input_device()
        .ok_or_else(|| anyhow!("no default input device"))
}

fn handle_start_session(
    host: &cpal::Host,
    preferred_device: Option<&str>,
    cmd_rx: &Receiver<Cmd>,
    rep_tx: &Sender<Reply>,
) -> Result<()> {
    let device = pick_device(host, preferred_device)?;

    let supported = device
        .default_input_config()
        .context("failed to query default input config")?;

    let sample_format = supported.sample_format();
    let config: StreamConfig = supported.into();
    let sample_rate = config.sample_rate.0;
    let channels = config.channels;

    let samples = Arc::new(Mutex::new(Vec::<f32>::with_capacity(
        sample_rate as usize * 60,
    )));
    let err_fn = |err| eprintln!("[director-general] audio stream error: {err}");

    let stream = match sample_format {
        SampleFormat::F32 => {
            let s = samples.clone();
            device.build_input_stream(
                &config,
                move |data: &[f32], _| {
                    if let Ok(mut buf) = s.lock() {
                        buf.extend_from_slice(data);
                    }
                },
                err_fn,
                None,
            )?
        }
        SampleFormat::I16 => {
            let s = samples.clone();
            device.build_input_stream(
                &config,
                move |data: &[i16], _| {
                    if let Ok(mut buf) = s.lock() {
                        buf.extend(data.iter().map(|&x| x as f32 / i16::MAX as f32));
                    }
                },
                err_fn,
                None,
            )?
        }
        SampleFormat::U16 => {
            let s = samples.clone();
            device.build_input_stream(
                &config,
                move |data: &[u16], _| {
                    if let Ok(mut buf) = s.lock() {
                        buf.extend(data.iter().map(|&x| (x as f32 - 32768.0) / 32768.0));
                    }
                },
                err_fn,
                None,
            )?
        }
        fmt => return Err(anyhow!("unsupported sample format: {fmt:?}")),
    };

    stream.play().context("failed to start cpal stream")?;
    rep_tx
        .send(Reply::Started)
        .map_err(|_| anyhow!("reply channel closed"))?;

    // Block until we get a Stop command.
    loop {
        match cmd_rx.recv() {
            Ok(Cmd::Stop) => break,
            Ok(Cmd::Start { .. }) => {
                // Already recording — ignore extra Start.
                let _ = rep_tx.send(Reply::Error("already recording".into()));
            }
            Err(_) => return Err(anyhow!("command channel closed")),
        }
    }

    drop(stream);

    let buf = samples.lock().unwrap().clone();
    rep_tx
        .send(Reply::Stopped {
            samples: buf,
            source_rate: sample_rate,
            source_channels: channels,
        })
        .map_err(|_| anyhow!("reply channel closed"))?;

    Ok(())
}

/// Convert multi-channel interleaved samples to mono by averaging channels.
pub fn downmix_to_mono(samples: &[f32], channels: u16) -> Vec<f32> {
    if channels <= 1 {
        return samples.to_vec();
    }
    let ch = channels as usize;
    let frames = samples.len() / ch;
    let mut out = Vec::with_capacity(frames);
    for frame in 0..frames {
        let start = frame * ch;
        let sum: f32 = samples[start..start + ch].iter().sum();
        out.push(sum / ch as f32);
    }
    out
}

/// Linear resampling. Good enough for voice transcription with Whisper —
/// any aliasing introduced is below Whisper's robustness threshold.
/// We avoid pulling in a heavyweight FFT resampler for this hot path.
pub fn resample_linear(input: &[f32], in_rate: u32, out_rate: u32) -> Vec<f32> {
    if in_rate == out_rate || input.is_empty() {
        return input.to_vec();
    }
    let ratio = out_rate as f64 / in_rate as f64;
    let out_len = ((input.len() as f64) * ratio).round() as usize;
    let mut output = Vec::with_capacity(out_len);
    let last_idx = input.len() - 1;
    for i in 0..out_len {
        let src_pos = i as f64 / ratio;
        let src_idx = src_pos.floor() as usize;
        if src_idx >= last_idx {
            output.push(input[last_idx]);
        } else {
            let frac = (src_pos - src_idx as f64) as f32;
            let a = input[src_idx];
            let b = input[src_idx + 1];
            output.push(a + (b - a) * frac);
        }
    }
    output
}

/// Write samples as 16-bit PCM WAV at 16 kHz mono. This is the format Whisper expects.
pub fn write_wav_16k_mono(samples: &[f32], path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    let spec = WavSpec {
        channels: 1,
        sample_rate: TARGET_SAMPLE_RATE,
        bits_per_sample: 16,
        sample_format: HoundSampleFormat::Int,
    };
    let mut writer = WavWriter::create(path, spec)
        .with_context(|| format!("failed to create WAV at {path:?}"))?;
    for &s in samples {
        let clipped = s.clamp(-1.0, 1.0);
        let v = (clipped * i16::MAX as f32) as i16;
        writer.write_sample(v)?;
    }
    writer.finalize().context("failed to finalize WAV")?;
    Ok(())
}

/// One-shot helper: take a recording result and write it as 16 kHz mono WAV.
pub fn save_recording_as_wav(rec: RecordingResult, path: &Path) -> Result<()> {
    let mono = downmix_to_mono(&rec.samples, rec.source_channels);
    let resampled = resample_linear(&mono, rec.source_rate, TARGET_SAMPLE_RATE);
    write_wav_16k_mono(&resampled, path)?;
    Ok(())
}
