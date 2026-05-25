# Voiceeee — Learnings

Decizii tehnice și de ce.

## 2026-04-08 — Stack inițial

### De ce Tauri 2 (nu Electron)
Tauri compilează un binary nativ ~10 MB, Electron duce 100+ MB. Pentru această aplicație în plus, avem nevoie de:
- Audio capture system-wide (cpal — pure Rust)
- Global keyboard hook (tauri-plugin-global-shortcut, peste GH `global-hotkey`)
- Simulare keystrokes (enigo)
Toate astea funcționează nativ în Rust și ar fi fost posibile în Electron doar prin native modules (chinuitor pe Windows). Tauri = backend Rust direct → simplu.

### De ce whisper.cpp prin whisper-rs (nu OpenAI Whisper Python)
- **Local-only**: zero Python runtime, zero pip, zero venv
- **Single binary**: la final user instalează un MSI, gata
- **CUDA optional**: feature flag în Cargo.toml, fără probleme cu PyTorch CUDA versions
- **Performanță**: whisper.cpp e mai rapid decât Python whisper pe CPU, comparabil cu faster-whisper pe GPU
- whisper-rs e singura librărie de binding Rust matură (v0.16, mentenanță activă)

### De ce large-v3-turbo default (nu large-v3 sau medium)
- **Turbo = 5.4x mai rapid decât large-v3** prin reducerea decoder layers de la 32 la 4
- Calitate ~similară cu large-v2 (mai bună decât medium pe RO)
- Encap în 1.55 GB → încape lejer în 4 GB VRAM al RTX 3050 Laptop
- large-v3 (2.93 GB) e descărcat ca opțiune "max quality" în UI dacă user vrea

### De ce React + TS + Tailwind (nu Svelte/vanilla)
Sergiu învață activ aceste tehnologii (vine din PHP/WordPress). Settings UI = teren bun de practică pentru:
- Componente React funcționale
- Hooks (useState, useEffect, custom hook pentru Tauri events)
- TypeScript strict (typing pentru IPC commands)
- Tailwind utility classes
Costul ușor de complexitate față de vanilla e justificat de valoarea educațională.

### De ce JSON nu SQLite pentru history
- < 1000 entries probabil → JSON e suficient
- Inspectabil manual cu orice text editor (debugging facil)
- Zero deps native suplimentare
- serde_json e standard în ecosistemul Rust
SQLite ar fi mai bun la >10k entries sau dacă voiam căutare full-text.

### De ce backtick (`` ` ``) ca default hotkey
Cerință explicită Sergiu. Are tradeoff: în terminale/dead key Windows poate da bătăi de cap. Mitigation: UI permite remap în 2 clickuri.

## 2026-04-08 — Faza 1 build issues

### CMake Visual Studio generator vs Ninja
**Problem:** whisper.cpp's default CMake build picks generator `"Visual Studio 17 2022"`. With **Visual Studio Build Tools 2022** (not full Visual Studio), CMake fails: `could not find any instance of Visual Studio`.

**Soluție:** Instalat Ninja (`winget install Ninja-build.Ninja`) și setat `CMAKE_GENERATOR=Ninja` în env. Ninja e și mai rapid decât MSBuild, deci win-win.

### vcvars64.bat needs vswhere.exe
**Problem:** Calling `vcvars64.bat` din cmd printează `'vswhere.exe' is not recognized` și environment-ul MSVC nu e configurat. vcvars64.bat → vcvarsall.bat → vsdevcmd.bat → calls `vswhere` fără path absolut.

**Soluție:** Adăugat `C:\Program Files (x86)\Microsoft Visual Studio\Installer` în PATH la începutul `build-rust.cmd` ca să găsească vswhere.

### Tool-uri proaspăt instalate vs PATH bash existent
**Problem:** Bash session care rulează în Claude Code Tool și-a snapshot-uit `PATH` la pornire. Cargo, CMake, LLVM, CUDA, Ninja toate instalate după pornire → nu apar în `where cargo` etc.

**Soluție:** `build-rust.cmd` adaugă explicit căile absolute la PATH înainte de cargo. Poate fi fix-uit definitiv reluând bash-ul.

## 2026-05-25 — Faza 8 — Director General rebrand & feature pack

### De ce low-level keyboard hook pentru Caps Lock (nu RegisterHotKey)
`tauri-plugin-global-shortcut` folosește `RegisterHotKey` din Win32. Acel API doar **notifică** că s-a apăsat o tastă — nu o **consumă**. Dacă setezi Caps Lock ca hotkey prin RegisterHotKey, primești evenimentul DAR Windows tot face toggle pe Caps Lock — exact ce nu vrem.

**Soluție:** `SetWindowsHookExW(WH_KEYBOARD_LL, ...)` instalează un hook de nivel jos care primește **toate** apăsările de tastă înainte ca OS să le proceseze. Returnând `LRESULT(1)` din `keyboard_proc`, tasta dispare — nu mai ajunge la OS, deci niciun toggle.

**Tradeoff:**
- Hook-ul trebuie să ruleze pe un thread cu message pump (`GetMessageW` loop). Nu se poate face în main thread.
- Cross-thread communication via `mpsc::Sender<()>` într-un `OnceLock<HookState>`.
- Funcționează DOAR pe Windows (cfg-gated).
- Cere admin? Nu, hook-urile LL nu cer privilegii.

### De ce overlay window separat (nu webview popup)
Vrem un widget tip equalizer mereu vizibil în timpul recording-ului. Opțiuni:
1. **Popup în main window** — dispare când minimizezi main → useless
2. **Notification toast** — Windows OS toast e ne-customizabil, dispare după ~5s
3. **A doua fereastră Tauri** — full control: alwaysOnTop, transparent, skipTaskbar, focus:false (nu fură focus)

Opțiunea 3 câștigă. Cost: Vite multi-page (2 entry points, 2 HTML files), capabilities listează ambele windows, sincronizare via `app.listen("mode-changed")`.

### De ce equalizer fake (sin/cos), nu Web Audio API real
Audio capture-ul e în Rust (cpal), pe alt thread. Overlay-ul nu are acces direct la mic. Două opțiuni:
1. Rust emite eveniment cu RMS la fiecare 50ms → overlay listen + animație reală
2. CSS animație cu valori random/sinusoidal → "feels alive"

MVP merge cu opțiunea 2 (zero adăugat în Rust hot path). Dacă utilizatorul vrea feedback real, e o linie de cod în audio.rs să emit RMS.

### De ce whisper-rs cuda → optional feature
GitHub Actions Windows runners NU au CUDA Toolkit (e ~3 GB de descărcat, ~20 min de instalat). Build local cu CUDA = 1-3s real-time factor (RTX 3050), CPU build = 5-15x mai lent dar funcționează pe orice mașină.

**Soluție:** `cuda = ["whisper-rs/cuda"]` în `[features]`. CI build = CPU (default). Local: `npm run tauri build -- --features cuda`. Best of both.

### De ce active window title capturat ÎNAINTE de orice altceva
`GetForegroundWindow()` returnează handle-ul ferestrei OS focused. Dacă hotkey-ul e apăsat în Word, foreground e Word. DAR dacă Tauri arată overlay-ul/orice → foreground devine Tauri.

**Soluție:** capturăm window title în primul lucru din `on_press` (mode == Idle), îl punem în `state.pending_window_title`. La final, când scriem HistoryEntry, îl drenăm din state. Funcționează indiferent ce se întâmplă în between.

### De ce `windows = "0.58"` (nu 0.62)
Tauri 2.10 are deja windows-core 0.58 în tree. Folosind aceeași versiune evităm duplicare în Cargo.lock (~30 MB extra crate-uri pentru nimic).
