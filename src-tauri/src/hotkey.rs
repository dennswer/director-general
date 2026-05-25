// Global hotkey registration + the press → record/stop/transcribe state machine.

use std::path::PathBuf;
use std::str::FromStr;
use std::time::Instant;

use anyhow::{anyhow, Context, Result};
use chrono::Utc;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_clipboard_manager::ClipboardExt;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

use crate::active_window::get_active_window_title;
use crate::audio::save_recording_as_wav;
use crate::paste::send_ctrl_v;
use crate::state::{AppState, Mode};
use crate::storage::{ensure_app_data_dir, save_history, HistoryEntry};

pub fn parse_shortcut(spec: &str) -> Result<Shortcut> {
    Shortcut::from_str(spec).map_err(|e| anyhow!("invalid hotkey '{spec}': {e:?}"))
}

/// Register a global shortcut that toggles the recording state machine.
/// Unregisters any previously registered shortcut first.
///
/// Some keys (e.g. Caps Lock) bypass this path entirely and are intercepted by
/// the low-level keyboard hook instead — see `caps_hook.rs`. The caller is
/// responsible for picking the right path.
pub fn register_hotkey(app: &AppHandle, spec: &str) -> Result<()> {
    let shortcut = parse_shortcut(spec)?;
    let gs = app.global_shortcut();
    let _ = gs.unregister_all();

    gs.on_shortcut(shortcut, move |app, _sc, event| {
        if event.state == ShortcutState::Pressed {
            on_press(app);
        }
    })
    .with_context(|| format!("registering shortcut {spec}"))?;

    println!("[hotkey] registered: {spec}");
    Ok(())
}

pub fn unregister_all(app: &AppHandle) {
    let _ = app.global_shortcut().unregister_all();
}

/// Entry point for both the global-shortcut callback and the low-level hook.
pub fn on_press(app: &AppHandle) {
    let state = app.state::<AppState>();
    let (start_recording, do_stop) = {
        let mut mode = state.mode.lock().unwrap();
        match *mode {
            Mode::Idle => {
                *mode = Mode::Recording;
                (true, false)
            }
            Mode::Recording => {
                *mode = Mode::Transcribing;
                (false, true)
            }
            Mode::Transcribing => return, // busy
        }
    };

    if start_recording {
        // Capture the foreground window title *before* anything we do can
        // steal focus.
        let title = get_active_window_title();
        *state.pending_window_title.lock().unwrap() = title;

        let device = state.settings.lock().unwrap().input_device.clone();
        if let Err(e) = state.recorder.start(device) {
            eprintln!("[hotkey] start_recording failed: {e:#}");
            *state.mode.lock().unwrap() = Mode::Idle;
            let _ = app.emit("error", format!("start_recording: {e:#}"));
            return;
        }
        let _ = app.emit("mode-changed", "recording");
        let _ = app.emit("recording-started", ());
        return;
    }

    if do_stop {
        let _ = app.emit("mode-changed", "transcribing");
        let _ = app.emit("recording-stopped", ());

        let app2 = app.clone();
        std::thread::spawn(move || {
            run_transcription_flow(&app2);
        });
    }
}

/// Stop recording → save WAV → transcribe → clipboard → paste → history → emit events.
/// Always resets mode to Idle at the end, even on error.
fn run_transcription_flow(app: &AppHandle) {
    let started = Instant::now();
    let result = transcription_flow_inner(app, started);

    let state = app.state::<AppState>();
    *state.mode.lock().unwrap() = Mode::Idle;
    let _ = app.emit("mode-changed", "idle");

    match result {
        Ok(text) => {
            let _ = app.emit("transcription-complete", text);
        }
        Err(e) => {
            eprintln!("[transcription_flow] error: {e:#}");
            let _ = app.emit("transcription-error", format!("{e:#}"));
        }
    }
}

fn transcription_flow_inner(app: &AppHandle, started: Instant) -> Result<String> {
    let state = app.state::<AppState>();

    // 1. Stop recording, get raw samples
    let recording = state.recorder.stop().context("recorder.stop")?;

    // 2. Save WAV to app_data/tmp/
    let app_data = ensure_app_data_dir(app)?;
    let tmp_dir = app_data.join("tmp");
    std::fs::create_dir_all(&tmp_dir)?;
    let ts = Utc::now().format("%Y%m%d-%H%M%S").to_string();
    let wav_path: PathBuf = tmp_dir.join(format!("rec_{ts}.wav"));
    save_recording_as_wav(recording, &wav_path).context("save_recording_as_wav")?;

    // 3. Transcribe
    let (language, common_words) = {
        let s = state.settings.lock().unwrap();
        (s.language.clone(), s.common_words.clone())
    };
    let prompt = if common_words.trim().is_empty() {
        None
    } else {
        Some(common_words)
    };
    let text = state
        .transcriber
        .transcribe(&wav_path, language.as_deref(), prompt.as_deref())
        .context("transcribe")?;

    // 4. Clipboard
    app.clipboard()
        .write_text(text.clone())
        .map_err(|e| anyhow!("clipboard write: {e:?}"))?;

    // 5. Optional auto-paste
    let auto_paste = state.settings.lock().unwrap().auto_paste;
    if auto_paste && !text.is_empty() {
        // Small delay so the Tauri window (if focused) has time to lose focus.
        std::thread::sleep(std::time::Duration::from_millis(80));
        if let Err(e) = send_ctrl_v() {
            eprintln!("[paste] send_ctrl_v failed: {e:#}");
        }
    }

    // 6. Append to history
    let model_name = state
        .settings
        .lock()
        .unwrap()
        .model_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string();

    let window_title = state.pending_window_title.lock().unwrap().take();

    let entry = HistoryEntry {
        id: uuid::Uuid::new_v4().to_string(),
        timestamp: Utc::now(),
        text: text.clone(),
        audio_path: wav_path,
        model: model_name,
        duration_ms: started.elapsed().as_millis() as u64,
        window_title,
    };

    {
        let mut hist = state.history.lock().unwrap();
        hist.push(entry);
        let max = state.settings.lock().unwrap().max_history;
        if hist.len() > max {
            let drain_count = hist.len() - max;
            hist.drain(0..drain_count);
        }
        save_history(app, &hist)?;
    }

    Ok(text)
}
