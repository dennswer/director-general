// Whisper transcription wrapper.
//
// `WhisperContext` is heavy to load (2-5 s with CUDA on a 1.5 GB model), so we
// build it once and keep it inside the app state. The actual inference creates
// a fresh `WhisperState` per call.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use anyhow::{Context, Result};
use hound::{SampleFormat, WavReader};
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

use crate::audio::{downmix_to_mono, resample_linear};

const TARGET_SAMPLE_RATE: u32 = 16_000;

pub struct Transcriber {
    ctx: Arc<WhisperContext>,
}

impl Transcriber {
    pub fn new(model_path: &Path) -> Result<Self> {
        // On a CUDA-enabled build we try GPU first and silently fall back to
        // CPU if init fails — this covers machines without an NVIDIA driver,
        // missing cuBLAS DLLs, or a GPU that's too small for the model.
        // On a CPU-only build the `use_gpu(true)` flag is ignored by whisper-rs.
        let ctx = match try_load(model_path, true) {
            Ok(c) => c,
            Err(e_gpu) => {
                eprintln!(
                    "[transcribe] GPU init failed, falling back to CPU: {e_gpu:#}"
                );
                try_load(model_path, false).with_context(|| {
                    format!("failed to load whisper model from {model_path:?} (CPU fallback after GPU error: {e_gpu:#})")
                })?
            }
        };
        Ok(Self {
            ctx: Arc::new(ctx),
        })
    }

    pub fn transcribe(
        &self,
        wav: &Path,
        language: Option<&str>,
        initial_prompt: Option<&str>,
    ) -> Result<String> {
        let samples = read_wav_to_f32_mono_16k(wav)?;
        if samples.is_empty() {
            return Ok(String::new());
        }

        let mut state = self
            .ctx
            .create_state()
            .context("failed to create whisper state")?;

        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        params.set_n_threads(num_cpus::get() as i32);
        params.set_language(language); // None → auto-detect
        params.set_translate(false);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_special(false);
        params.set_print_timestamps(false);

        if let Some(prompt) = initial_prompt {
            let trimmed = prompt.trim();
            if !trimmed.is_empty() {
                params.set_initial_prompt(trimmed);
            }
        }

        state
            .full(params, &samples)
            .context("whisper full() failed")?;

        let n = state.full_n_segments();
        let mut text = String::new();
        for i in 0..n {
            if let Some(seg) = state.get_segment(i) {
                if let Ok(s) = seg.to_str_lossy() {
                    text.push_str(&s);
                }
            }
        }
        Ok(text.trim().to_string())
    }
}

/// Lazily loaded transcriber. Avoids the 2-5 s model load at app startup.
pub struct LazyTranscriber {
    inner: Mutex<Option<Transcriber>>,
    model_path: Mutex<PathBuf>,
}

impl LazyTranscriber {
    pub fn new(model_path: PathBuf) -> Self {
        Self {
            inner: Mutex::new(None),
            model_path: Mutex::new(model_path),
        }
    }

    pub fn set_model_path(&self, path: PathBuf) {
        *self.model_path.lock().unwrap() = path;
        // Drop the cached transcriber so the next call reloads.
        *self.inner.lock().unwrap() = None;
    }

    pub fn current_model_path(&self) -> PathBuf {
        self.model_path.lock().unwrap().clone()
    }

    pub fn transcribe(
        &self,
        wav: &Path,
        language: Option<&str>,
        initial_prompt: Option<&str>,
    ) -> Result<String> {
        let mut guard = self.inner.lock().unwrap();
        if guard.is_none() {
            let path = self.model_path.lock().unwrap().clone();
            *guard = Some(Transcriber::new(&path)?);
        }
        guard.as_ref().unwrap().transcribe(wav, language, initial_prompt)
    }
}

fn try_load(model_path: &Path, gpu: bool) -> Result<WhisperContext> {
    let mut params = WhisperContextParameters::default();
    params.use_gpu(gpu);
    WhisperContext::new_with_params(model_path, params)
        .with_context(|| format!("WhisperContext init (gpu={gpu})"))
}

/// Read a WAV file from disk and return f32 samples that are mono and 16 kHz —
/// the format Whisper requires.
pub fn read_wav_to_f32_mono_16k(path: &Path) -> Result<Vec<f32>> {
    let mut reader =
        WavReader::open(path).with_context(|| format!("failed to open WAV {path:?}"))?;
    let spec = reader.spec();

    let mut samples: Vec<f32> = match spec.sample_format {
        SampleFormat::Int => match spec.bits_per_sample {
            16 => reader
                .samples::<i16>()
                .map(|s| s.map(|v| v as f32 / i16::MAX as f32))
                .collect::<Result<_, _>>()?,
            32 => reader
                .samples::<i32>()
                .map(|s| s.map(|v| v as f32 / i32::MAX as f32))
                .collect::<Result<_, _>>()?,
            other => anyhow::bail!("unsupported int bits_per_sample: {other}"),
        },
        SampleFormat::Float => reader.samples::<f32>().collect::<Result<_, _>>()?,
    };

    if spec.channels > 1 {
        samples = downmix_to_mono(&samples, spec.channels);
    }

    if spec.sample_rate != TARGET_SAMPLE_RATE {
        samples = resample_linear(&samples, spec.sample_rate, TARGET_SAMPLE_RATE);
    }

    Ok(samples)
}
