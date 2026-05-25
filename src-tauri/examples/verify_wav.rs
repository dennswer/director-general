// Read a WAV file, print spec + RMS / peak amplitude.
// Usage: cargo run --example verify_wav -- <path>

use anyhow::Result;
use hound::WavReader;
use std::env;

fn main() -> Result<()> {
    let path = env::args()
        .nth(1)
        .unwrap_or_else(|| "test_recording.wav".into());
    let mut reader = WavReader::open(&path)?;
    let spec = reader.spec();
    println!("File:     {}", path);
    println!("Channels: {}", spec.channels);
    println!("Rate:     {} Hz", spec.sample_rate);
    println!("Bits:     {}", spec.bits_per_sample);
    println!("Format:   {:?}", spec.sample_format);

    let samples: Vec<i16> = reader.samples::<i16>().collect::<Result<_, _>>()?;
    let n = samples.len();
    let sum_sq: f64 = samples.iter().map(|&s| (s as f64).powi(2)).sum();
    let rms = (sum_sq / n as f64).sqrt();
    let peak = samples.iter().map(|&s| s.abs()).max().unwrap_or(0);
    let duration = n as f64 / spec.sample_rate as f64;

    println!("Samples:  {}", n);
    println!("Duration: {:.2}s", duration);
    println!("RMS:      {:.1} ({:.4} normalized)", rms, rms / 32767.0);
    println!("Peak:     {} ({:.4} normalized)", peak, peak as f64 / 32767.0);

    if rms < 100.0 {
        println!("WARNING: very low signal — possibly silence or muted mic");
    } else {
        println!("OK: signal present");
    }

    Ok(())
}
