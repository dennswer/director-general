// Standalone test for the whisper transcription module.
//
// Run with:  cargo run --example transcribe_test --release -- <path-to-wav> [language]
// (release mode is much faster for inference; debug works but is slower)

use std::env;
use std::path::PathBuf;
use std::time::Instant;

use voiceeee_lib::transcribe::Transcriber;

fn main() -> anyhow::Result<()> {
    let mut args = env::args().skip(1);
    let wav_path = args
        .next()
        .ok_or_else(|| anyhow::anyhow!("usage: transcribe_test <wav> [lang]"))?;
    let language = args.next();

    let model = locate_model()?;
    println!("Loading model: {model:?}");
    let t0 = Instant::now();
    let transcriber = Transcriber::new(&model)?;
    println!("Model loaded in {:.2}s", t0.elapsed().as_secs_f32());

    println!("Transcribing {wav_path}...");
    let t1 = Instant::now();
    let text = transcriber.transcribe(&PathBuf::from(&wav_path), language.as_deref())?;
    let dt = t1.elapsed().as_secs_f32();

    println!("--- Transcription ({:.2}s) ---", dt);
    println!("{text}");
    println!("--- end ---");

    Ok(())
}

fn locate_model() -> anyhow::Result<PathBuf> {
    let candidates = ["ggml-large-v3-turbo.bin", "ggml-large-v3.bin"];
    let bases = ["../models", "models", "../../models"];
    for base in &bases {
        for name in &candidates {
            let p = PathBuf::from(base).join(name);
            if p.exists() {
                return Ok(p);
            }
        }
    }
    Err(anyhow::anyhow!("no whisper model found in ../models or models/"))
}
