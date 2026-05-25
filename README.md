# Director General

Local voice typing for Windows. Press a key, talk, get text in any input field
— transcription runs on your machine via [whisper.cpp](https://github.com/ggerganov/whisper.cpp).

Built with Tauri 2 + React + Rust + whisper-rs (CUDA or CPU).

## Features

- **Press-to-talk with any key as hotkey.** Caps Lock works as the default and
  is intercepted via a low-level Windows keyboard hook so it does *not* toggle
  the Caps Lock state — it only starts/stops recording.
- **Always-on-top recording overlay** with an animated equalizer + stop button.
- **Auto-paste** the transcription into whatever window you had focused.
- **Active-window tracking** — each history entry remembers *where* you
  dictated.
- **Microphone picker** in settings.
- **Common-words list** fed to Whisper as `initial_prompt` so terms like
  "API", "WordPress", "Bossnet" are transcribed consistently.
- **Sys tray** — close the window and the app keeps running; left-click the
  tray icon to reopen.
- **Two Whisper models** out of the box: `large-v3-turbo` (1.5 GB, fast) and
  `large-v3` (3 GB, max quality).
- **History** with human-readable timestamps and one-click copy.

## Quick start (dev)

```sh
npm install
# Drop a model into ./models/ (the app expects ggml-large-v3-turbo.bin or
# ggml-large-v3.bin)
npm run tauri dev -- --features cuda     # GPU build (RTX 3050+)
npm run tauri dev                         # CPU build (any Windows machine)
```

## Building a release

GitHub Actions builds an unsigned MSI/NSIS bundle for every push to `main`.
See [`.github/workflows/build.yml`](.github/workflows/build.yml). The CI
artefact is a CPU-only build — recompile locally with `--features cuda` if
you have an NVIDIA GPU and want acceleration.

## Hotkey notes

`global_hotkey` exposes keys by their KeyboardEvent `code`:

- `CapsLock` — default; uses the low-level hook so the toggle is suppressed.
- `Backquote`, `F9`, `Ctrl+Space`, `Ctrl+Shift+KeyA`, etc.

Pick a key in Settings; the picker captures whatever you press.

## License

MIT — see [LICENSE](LICENSE).
