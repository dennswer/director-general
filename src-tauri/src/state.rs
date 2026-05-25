// Shared application state.

use std::sync::Mutex;

use crate::audio::Recorder;
use crate::storage::{HistoryEntry, Settings};
use crate::transcribe::LazyTranscriber;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    Idle,
    Recording,
    Transcribing,
}

pub struct AppState {
    pub mode: Mutex<Mode>,
    pub recorder: Recorder,
    pub transcriber: LazyTranscriber,
    pub settings: Mutex<Settings>,
    pub history: Mutex<Vec<HistoryEntry>>,
    /// Foreground window title captured at the moment recording starts;
    /// drained when the history entry is written.
    pub pending_window_title: Mutex<Option<String>>,
}

impl AppState {
    pub fn new(settings: Settings, history: Vec<HistoryEntry>) -> Self {
        let transcriber = LazyTranscriber::new(settings.model_path.clone());
        Self {
            mode: Mutex::new(Mode::Idle),
            recorder: Recorder::spawn(),
            transcriber,
            settings: Mutex::new(settings),
            history: Mutex::new(history),
            pending_window_title: Mutex::new(None),
        }
    }
}
