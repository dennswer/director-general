// Synthetic pipeline test: generate a sine wave, push it through downmix +
// resample + WAV write, then read it back and confirm we still have a sine wave
// at the same frequency. Doesn't depend on the microphone.

use anyhow::{anyhow, Result};
use hound::WavReader;
use std::path::PathBuf;
use voiceeee_lib::audio::{downmix_to_mono, resample_linear, write_wav_16k_mono, TARGET_SAMPLE_RATE};

const SOURCE_RATE: u32 = 48_000;
const FREQ_HZ: f32 = 440.0;
const DURATION_SEC: f32 = 2.0;
const CHANNELS: u16 = 2;

fn main() -> Result<()> {
    println!("=== Synthetic pipeline test ===");

    // Generate a stereo sine wave at 48 kHz (mimics typical mic config)
    let n_frames = (SOURCE_RATE as f32 * DURATION_SEC) as usize;
    let mut interleaved = Vec::with_capacity(n_frames * CHANNELS as usize);
    for i in 0..n_frames {
        let t = i as f32 / SOURCE_RATE as f32;
        let sample = (2.0 * std::f32::consts::PI * FREQ_HZ * t).sin() * 0.5;
        for _ in 0..CHANNELS {
            interleaved.push(sample);
        }
    }
    println!(
        "Generated {} interleaved samples at {} Hz, {} ch ({:.2}s {:.0} Hz sine)",
        interleaved.len(),
        SOURCE_RATE,
        CHANNELS,
        DURATION_SEC,
        FREQ_HZ
    );

    // Downmix
    let mono = downmix_to_mono(&interleaved, CHANNELS);
    assert_eq!(mono.len(), n_frames, "downmix length mismatch");
    let mono_peak = mono.iter().fold(0.0f32, |acc, &v| acc.max(v.abs()));
    println!("After downmix: {} samples, peak {:.4}", mono.len(), mono_peak);
    if mono_peak < 0.4 || mono_peak > 0.6 {
        return Err(anyhow!("downmix peak out of expected range (~0.5)"));
    }

    // Resample 48k -> 16k
    let resampled = resample_linear(&mono, SOURCE_RATE, TARGET_SAMPLE_RATE);
    let expected_len = (n_frames as f64 * TARGET_SAMPLE_RATE as f64 / SOURCE_RATE as f64).round() as usize;
    println!(
        "After resample: {} samples (expected ~{})",
        resampled.len(),
        expected_len
    );
    if (resampled.len() as i64 - expected_len as i64).abs() > 4 {
        return Err(anyhow!("resampled length way off expected"));
    }

    // Write WAV
    let path = PathBuf::from("synth_test.wav");
    write_wav_16k_mono(&resampled, &path)?;
    println!("Wrote {:?}", path);

    // Read back, verify spec + frequency
    let mut reader = WavReader::open(&path)?;
    let spec = reader.spec();
    if spec.sample_rate != TARGET_SAMPLE_RATE {
        return Err(anyhow!("WAV rate {} != 16000", spec.sample_rate));
    }
    if spec.channels != 1 {
        return Err(anyhow!("WAV channels {} != 1", spec.channels));
    }
    if spec.bits_per_sample != 16 {
        return Err(anyhow!("WAV bits {} != 16", spec.bits_per_sample));
    }

    let read_samples: Vec<f32> = reader
        .samples::<i16>()
        .map(|s| s.map(|v| v as f32 / i16::MAX as f32))
        .collect::<Result<_, _>>()?;
    let peak = read_samples.iter().fold(0.0f32, |acc, &v| acc.max(v.abs()));
    let rms = (read_samples.iter().map(|s| (*s as f64).powi(2)).sum::<f64>() / read_samples.len() as f64).sqrt() as f32;
    println!("Read back: {} samples, peak {:.4}, rms {:.4}", read_samples.len(), peak, rms);

    if peak < 0.4 || peak > 0.6 {
        return Err(anyhow!("read-back peak out of expected range"));
    }

    // Crude frequency check via zero-crossing count.
    // For a 440 Hz sine over ~2s we expect ~880 zero crossings ± a few.
    let mut zero_crossings = 0u32;
    for w in read_samples.windows(2) {
        if w[0].is_sign_negative() != w[1].is_sign_negative() {
            zero_crossings += 1;
        }
    }
    let estimated_freq = zero_crossings as f32 / 2.0 / DURATION_SEC;
    println!(
        "Zero crossings: {} → estimated freq {:.1} Hz (expected ~{:.1})",
        zero_crossings, estimated_freq, FREQ_HZ
    );

    if (estimated_freq - FREQ_HZ).abs() > 5.0 {
        return Err(anyhow!(
            "frequency drift too large: {} vs {}",
            estimated_freq,
            FREQ_HZ
        ));
    }

    println!("=== PASS — pipeline preserves signal ===");
    std::fs::remove_file(&path).ok();
    Ok(())
}
