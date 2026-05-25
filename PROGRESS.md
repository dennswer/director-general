# Voiceeee — Progress

App desktop pentru transcriere locală cu Whisper (Tauri 2 + React + Rust + whisper.cpp CUDA).
Plan complet: `C:\Users\Dennswer\.claude\plans\pure-jingling-falcon.md`

## Faze

- [x] **Faza 0 — Prerequisites** ✅
  - [x] Verificat hardware: NVIDIA RTX 3050 Laptop, 4GB VRAM, CUDA 12.3
  - [x] Verificat tooling: Node 24.11.1, npm 11.6.2, MSVC BuildTools 2022, WebView2 OK
  - [x] Descărcat `models/ggml-large-v3-turbo.bin` (1.55 GB)
  - [x] Descărcat `models/ggml-large-v3.bin` (2.93 GB)
  - [x] Creat `setup-prereqs.ps1`
  - [x] Instalat Rust 1.94.1 (via winget Rustlang.Rustup)
  - [x] Instalat CMake 4.3.1 (via winget Kitware.CMake)
  - [x] Instalat LLVM 22.1.2 (via winget LLVM.LLVM) + adăugat manual în User PATH
  - [x] Setat LIBCLANG_PATH = C:\Program Files\LLVM\bin
  - [x] Instalat CUDA Toolkit 12.6.85 (via winget Nvidia.CUDA)
  - [x] Toate verificările verzi în setup-prereqs.ps1

- [x] **Faza 1 — Scaffold proiect** ✅
  - [x] `npm create tauri-app@latest` cu template react-ts → `voiceeee`
  - [x] Configurat Tailwind v4 cu `@tailwindcss/vite` plugin
  - [x] Adăugat deps Cargo: whisper-rs 0.16 (cuda), cpal 0.15, hound 3.5, rubato 0.16, enigo 0.5, tauri-plugin-{global-shortcut,clipboard-manager,fs}
  - [x] Capabilities: global-shortcut, clipboard, fs cu app-data scope
  - [x] Lib.rs cu plugin registration (global-shortcut, clipboard, fs)
  - [x] App.tsx cu sidebar (Status / Settings / History) și tema dark
  - [x] `cargo check` PASS în 10m 36s — whisper-rs + CUDA OK
  - [x] `npm run build` PASS — TS + Vite + Tailwind v4 OK
  - [x] Bonus: instalat Ninja, scris `build-rust.cmd` care încarcă vcvars64 + setează env corect

  **Învățat:**
  - CMake-ul whisper.cpp default vrea generator "Visual Studio 17 2022" care eșuează cu BuildTools 2022. Soluție: instalat Ninja și setat `CMAKE_GENERATOR=Ninja`.
  - vcvars64.bat are nevoie de vswhere.exe în PATH (e în `C:\Program Files (x86)\Microsoft Visual Studio\Installer`)
  - Tools instalate într-o sesiune nu apar în PATH-ul bash-ului care era deja deschis. Build wrapper-ul hardcodează căile.

- [x] **Faza 2 — Audio recording** ✅
  - [x] `audio.rs` cu Recorder pe worker thread (cpal Stream e !Send pe Windows)
  - [x] Comunicare via channels (Cmd::Start/Stop, Reply::Started/Stopped)
  - [x] Multi-format input (F32/I16/U16) → normalizat la f32
  - [x] `downmix_to_mono()` cu averaging pe canale
  - [x] `resample_linear()` (interpolare liniară — suficient pentru voce; rubato opțional)
  - [x] `write_wav_16k_mono()` cu hound, 16-bit PCM
  - [x] `state.rs` cu AppState ce deține Recorder
  - [x] Tauri commands `start_recording` / `stop_recording`
  - [x] Example `record_test.rs` — captură 5s de la mic default
  - [x] Example `verify_wav.rs` — verifică spec + RMS/peak
  - [x] Example `synth_pipeline_test.rs` — pipeline test deterministic cu sinusoidă 440 Hz
  - [x] Pipeline PASS: 440 Hz în → 439.8 Hz out, peak/RMS corecte
  - [x] WAV format verificat: PCM 16-bit, 1 canal, 16000 Hz

- [x] **Faza 3 — Whisper transcribere** ✅
  - [x] `transcribe.rs` cu `Transcriber` (model load) și `LazyTranscriber` (lazy init la prima cerere)
  - [x] `read_wav_to_f32_mono_16k()` care handle-uiește WAV cu orice rate/channels (auto-resample + downmix)
  - [x] FullParams: greedy, language=auto, n_threads=cpu_count, no print
  - [x] Tauri command `transcribe_file(path, language)`
  - [x] AppState extins cu LazyTranscriber
  - [x] Lib.rs locate_default_model() cu căi candidate (../models, models, ../../models)
  - [x] Example `transcribe_test.rs` standalone
  - [x] **TEST PASS:** SAPI generated speech_en.wav → "Hello, this is a test of voice transcription using Whisper. The quick brown fox jumps over the lazy dog." 100% acuratețe
  - [x] **Performanță:** 1.71s pentru ~8s audio pe RTX 3050 → real-time factor 0.21x

- [x] **Faza 4 — State machine + hotkey** ✅
  - [x] `state.rs` cu Mode enum (Idle/Recording/Transcribing)
  - [x] `hotkey.rs` cu tauri-plugin-global-shortcut, register_hotkey + on_press handler
  - [x] State machine în handler: Idle→Recording→Transcribing→Idle
  - [x] Procesare pe thread worker ca să nu blocheze hotkey thread
  - [x] Events: mode-changed, recording-started, recording-stopped, transcription-complete, transcription-error

- [x] **Faza 5 — Output flow** ✅
  - [x] `paste.rs` cu `send_ctrl_v()` via enigo (Press/Click/Release)
  - [x] Clipboard write via tauri-plugin-clipboard-manager
  - [x] Delay 80ms înainte de paste (fereastra Tauri să-și piardă focus-ul)
  - [x] `storage.rs` cu Settings + HistoryEntry, load/save JSON
  - [x] `default_model_path()` cu căutare relativă la cwd ȘI executable
  - [x] Pruning automat istoric la `max_history` entries
  - [x] Commands: get_settings, update_settings, get_history, delete_history_entry, clear_history, copy_history_entry, transcribe_file
  - [x] Re-register hotkey automat când se schimbă în settings

- [x] **Faza 6 — UI React** ✅
  - [x] `lib/ipc.ts` — wrappers tipate pentru toate command-urile + event listeners + keyboardEventToShortcut
  - [x] `StatusIndicator.tsx` — orb cu 3 stări (Idle/Recording pulsant/Transcribing spinner), live event listeners
  - [x] `HotkeyPicker.tsx` — captură keydown, conversie la formato global_hotkey
  - [x] `Settings.tsx` — hotkey picker, language dropdown, model radio, auto-paste toggle, auto-save
  - [x] `HistoryList.tsx` — listă cu timestamps, expand on click, Copy/Delete per entry, Clear all
  - [x] `App.tsx` — sidebar cu nav, indicator mod live, layout dark Tailwind v4
  - [x] **Verificat E2E:** Tauri dev rulează, hotkey înregistrat, model găsit
  - [x] **Verificat vizual:** screenshots Playwright pe toate 3 tab-uri cu mock invoke (UI render perfect)

- [ ] **Faza 7 — Polish**
  - [ ] Tray icon
  - [ ] Autostart Windows
  - [ ] Cleanup WAV >7 zile
  - [ ] Build MSI release

## STOP-uri planificate

Conform regulilor din `~/.claude/CLAUDE.md`, mă opresc între faze pentru confirmare.
