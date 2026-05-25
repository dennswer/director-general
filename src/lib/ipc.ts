import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export type Mode = "idle" | "recording" | "transcribing";

export interface Settings {
  hotkey: string;
  model_path: string;
  language: string | null;
  auto_paste: boolean;
  max_history: number;
  input_device: string | null;
  common_words: string;
  show_overlay: boolean;
}

export interface HistoryEntry {
  id: string;
  timestamp: string; // ISO 8601 from chrono::DateTime<Utc>
  text: string;
  audio_path: string;
  model: string;
  duration_ms: number;
  window_title: string | null;
}

export const ipc = {
  ping: () => invoke<string>("ping"),
  getMode: () => invoke<Mode>("get_mode"),
  getSettings: () => invoke<Settings>("get_settings"),
  updateSettings: (s: Settings) =>
    invoke<void>("update_settings", { newSettings: s }),
  getHistory: () => invoke<HistoryEntry[]>("get_history"),
  deleteHistoryEntry: (id: string) =>
    invoke<void>("delete_history_entry", { id }),
  clearHistory: () => invoke<void>("clear_history"),
  copyHistoryEntry: (id: string) =>
    invoke<void>("copy_history_entry", { id }),
  transcribeFile: (path: string, language: string | null) =>
    invoke<string>("transcribe_file", { path, language }),
  listInputDevices: () => invoke<string[]>("list_input_devices"),
  triggerHotkey: () => invoke<void>("trigger_hotkey"),
};

export function onModeChanged(cb: (m: Mode) => void): Promise<UnlistenFn> {
  return listen<string>("mode-changed", (e) => cb(e.payload as Mode));
}

export function onTranscriptionComplete(
  cb: (text: string) => void,
): Promise<UnlistenFn> {
  return listen<string>("transcription-complete", (e) => cb(e.payload));
}

export function onTranscriptionError(
  cb: (msg: string) => void,
): Promise<UnlistenFn> {
  return listen<string>("transcription-error", (e) => cb(e.payload));
}

/// Convert a KeyboardEvent into a hotkey accelerator string compatible with
/// the `global_hotkey` Rust crate (used by tauri-plugin-global-shortcut).
/// Examples: "Backquote", "F9", "Ctrl+Space", "Ctrl+Shift+KeyA", "CapsLock"
export function keyboardEventToShortcut(e: KeyboardEvent): string | null {
  // Don't capture modifier-only presses
  if (
    e.code === "ControlLeft" ||
    e.code === "ControlRight" ||
    e.code === "ShiftLeft" ||
    e.code === "ShiftRight" ||
    e.code === "AltLeft" ||
    e.code === "AltRight" ||
    e.code === "MetaLeft" ||
    e.code === "MetaRight"
  ) {
    return null;
  }

  const parts: string[] = [];
  if (e.ctrlKey) parts.push("Ctrl");
  if (e.shiftKey) parts.push("Shift");
  if (e.altKey) parts.push("Alt");
  if (e.metaKey) parts.push("Super");
  parts.push(e.code);
  return parts.join("+");
}

/// Pretty-print a hotkey for display
export function prettyHotkey(s: string): string {
  return s
    .replace(/^Backquote$/, "`")
    .replace(/^Key([A-Z])$/, "$1")
    .replace(/^Digit(\d)$/, "$1")
    .replace(/^CapsLock$/, "Caps Lock")
    .replace(/Backquote/g, "`")
    .replace(/Key([A-Z])/g, "$1")
    .replace(/Digit(\d)/g, "$1");
}

/// Human-readable relative timestamp (e.g. "2 min ago", "yesterday 14:32").
export function humanTimestamp(iso: string): string {
  const t = new Date(iso);
  const now = new Date();
  const diffMs = now.getTime() - t.getTime();
  const diffMin = Math.round(diffMs / 60000);
  const diffSec = Math.round(diffMs / 1000);
  if (diffSec < 60) return "just now";
  if (diffMin < 60) return `${diffMin} min ago`;
  const sameDay =
    t.getFullYear() === now.getFullYear() &&
    t.getMonth() === now.getMonth() &&
    t.getDate() === now.getDate();
  const hhmm = t.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
  if (sameDay) return `today ${hhmm}`;
  const yesterday = new Date(now);
  yesterday.setDate(yesterday.getDate() - 1);
  const isYesterday =
    t.getFullYear() === yesterday.getFullYear() &&
    t.getMonth() === yesterday.getMonth() &&
    t.getDate() === yesterday.getDate();
  if (isYesterday) return `yesterday ${hhmm}`;
  return t.toLocaleString([], {
    day: "2-digit",
    month: "short",
    hour: "2-digit",
    minute: "2-digit",
  });
}
