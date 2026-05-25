// Standalone test for the audio module — bypasses Tauri so we can verify
// recording works in isolation. Records 5 seconds and writes a WAV file.
//
// Run with: cargo run --example record_test
// (use build-rust.cmd on Windows so the env is set up)

use std::path::PathBuf;
use std::time::Duration;

use voiceeee_lib::audio::{save_recording_as_wav, Recorder};

fn main() -> anyhow::Result<()> {
    println!("Spawning recorder...");
    let recorder = Recorder::spawn();

    println!("Starting recording for 5 seconds. Speak something...");
    recorder.start()?;
    std::thread::sleep(Duration::from_secs(5));

    println!("Stopping...");
    let result = recorder.stop()?;
    println!(
        "Captured {} samples at {} Hz, {} channels",
        result.samples.len(),
        result.source_rate,
        result.source_channels
    );

    let path = PathBuf::from("test_recording.wav");
    save_recording_as_wav(result, &path)?;
    println!("Saved to {:?}", path.canonicalize().unwrap_or(path));
    Ok(())
}
