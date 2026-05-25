// Persistence: settings.json + history.json in `%APPDATA%\ro.bossnet.directorgeneral\`

use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

fn default_max_history() -> usize {
    200
}

fn default_common_words() -> String {
    String::new()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    /// Hotkey accelerator string (parses with `global_hotkey::HotKey::from_str`).
    /// Default: "CapsLock" (intercepted via low-level keyboard hook so it doesn't toggle caps state).
    pub hotkey: String,

    /// Path to the GGML whisper model file.
    pub model_path: PathBuf,

    /// Whisper language hint. None = auto-detect, Some("ro") = Romanian, etc.
    pub language: Option<String>,

    /// If true, after transcription we automatically simulate Ctrl+V into the focused window.
    /// If false, we just put the text on the clipboard.
    pub auto_paste: bool,

    /// Maximum entries to keep in history.json. Older ones get pruned.
    #[serde(default = "default_max_history")]
    pub max_history: usize,

    /// Preferred input device name (cpal). None = default.
    #[serde(default)]
    pub input_device: Option<String>,

    /// Comma- or newline-separated list of words/terms the user wants Whisper
    /// to recognise correctly (technical jargon, names). Fed as `initial_prompt`.
    #[serde(default = "default_common_words")]
    pub common_words: String,

    /// Show the small floating equalizer overlay while recording.
    #[serde(default = "default_show_overlay")]
    pub show_overlay: bool,
}

fn default_show_overlay() -> bool {
    true
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            hotkey: "CapsLock".to_string(),
            model_path: default_model_path(),
            language: None,
            auto_paste: true,
            max_history: 200,
            input_device: None,
            common_words: String::new(),
            show_overlay: true,
        }
    }
}

pub fn default_model_path() -> PathBuf {
    let candidates = ["ggml-large-v3-turbo.bin", "ggml-large-v3.bin"];

    // Search bases relative to current dir AND relative to the exe's location.
    let mut bases: Vec<PathBuf> = vec![
        PathBuf::from("../models"),
        PathBuf::from("models"),
        PathBuf::from("../../models"),
        PathBuf::from("../../../models"),
    ];

    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            // src-tauri/target/debug/director-general.exe → models is at ../../../models
            for rel in &[
                "../../../models",
                "../../models",
                "../models",
                "models",
            ] {
                bases.push(dir.join(rel));
            }
        }
    }

    for base in &bases {
        for name in &candidates {
            let p = base.join(name);
            if p.exists() {
                return p.canonicalize().unwrap_or(p);
            }
        }
    }
    PathBuf::from("../models/ggml-large-v3-turbo.bin")
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub text: String,
    pub audio_path: PathBuf,
    pub model: String,
    pub duration_ms: u64,
    /// Title of the foreground window captured at the moment recording started.
    /// `None` on non-Windows or when the title couldn't be read.
    #[serde(default)]
    pub window_title: Option<String>,
}

pub fn settings_path(app: &AppHandle) -> Result<PathBuf> {
    Ok(app
        .path()
        .app_data_dir()
        .context("app_data_dir")?
        .join("settings.json"))
}

pub fn history_path(app: &AppHandle) -> Result<PathBuf> {
    Ok(app
        .path()
        .app_data_dir()
        .context("app_data_dir")?
        .join("history.json"))
}

pub fn ensure_app_data_dir(app: &AppHandle) -> Result<PathBuf> {
    let dir = app.path().app_data_dir().context("app_data_dir")?;
    fs::create_dir_all(&dir).with_context(|| format!("create {dir:?}"))?;
    Ok(dir)
}

pub fn load_settings(app: &AppHandle) -> Settings {
    let path = match settings_path(app) {
        Ok(p) => p,
        Err(_) => return Settings::default(),
    };
    if !path.exists() {
        return Settings::default();
    }
    match fs::read_to_string(&path) {
        Ok(s) => serde_json::from_str(&s).unwrap_or_default(),
        Err(_) => Settings::default(),
    }
}

pub fn save_settings(app: &AppHandle, s: &Settings) -> Result<()> {
    let _ = ensure_app_data_dir(app)?;
    let path = settings_path(app)?;
    let json = serde_json::to_string_pretty(s)?;
    fs::write(&path, json).with_context(|| format!("write {path:?}"))?;
    Ok(())
}

pub fn load_history(app: &AppHandle) -> Vec<HistoryEntry> {
    let path = match history_path(app) {
        Ok(p) => p,
        Err(_) => return Vec::new(),
    };
    if !path.exists() {
        return Vec::new();
    }
    match fs::read_to_string(&path) {
        Ok(s) => serde_json::from_str(&s).unwrap_or_default(),
        Err(_) => Vec::new(),
    }
}

pub fn save_history(app: &AppHandle, entries: &[HistoryEntry]) -> Result<()> {
    let _ = ensure_app_data_dir(app)?;
    let path = history_path(app)?;
    let json = serde_json::to_string_pretty(entries)?;
    fs::write(&path, json).with_context(|| format!("write {path:?}"))?;
    Ok(())
}
