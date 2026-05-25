// director-general — local voice typing for Windows with Whisper

pub mod active_window;
pub mod audio;
#[cfg(windows)]
pub mod caps_hook;
pub mod hotkey;
pub mod paste;
pub mod state;
pub mod storage;
pub mod transcribe;

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{Emitter, Listener, Manager, State, WindowEvent};
use tauri_plugin_autostart::MacosLauncher;

use crate::hotkey::{on_press, register_hotkey, unregister_all};
use crate::state::{AppState, Mode};
use crate::storage::{
    ensure_app_data_dir, load_history, load_settings, save_history, save_settings, HistoryEntry,
    Settings,
};

#[tauri::command]
fn ping() -> &'static str {
    "pong from director-general"
}

#[tauri::command]
fn get_mode(state: State<AppState>) -> Mode {
    *state.mode.lock().unwrap()
}

#[tauri::command]
fn get_settings(state: State<AppState>) -> Settings {
    state.settings.lock().unwrap().clone()
}

#[tauri::command]
fn list_input_devices() -> Vec<String> {
    crate::audio::list_input_devices()
}

#[tauri::command]
fn trigger_hotkey(app: tauri::AppHandle) {
    // Used by the overlay window's "stop" button.
    on_press(&app);
}

#[tauri::command]
fn update_settings(
    app: tauri::AppHandle,
    state: State<AppState>,
    new_settings: Settings,
) -> Result<(), String> {
    let (old_hotkey, old_model_path) = {
        let cur = state.settings.lock().unwrap();
        (cur.hotkey.clone(), cur.model_path.clone())
    };

    *state.settings.lock().unwrap() = new_settings.clone();
    save_settings(&app, &new_settings).map_err(|e| format!("save_settings: {e:#}"))?;

    if new_settings.hotkey != old_hotkey {
        apply_hotkey(&app, &new_settings.hotkey)
            .map_err(|e| format!("apply_hotkey: {e:#}"))?;
    }

    if new_settings.model_path != old_model_path {
        state
            .transcriber
            .set_model_path(new_settings.model_path.clone());
    }

    Ok(())
}

#[tauri::command]
fn get_history(state: State<AppState>) -> Vec<HistoryEntry> {
    state.history.lock().unwrap().clone()
}

#[tauri::command]
fn delete_history_entry(
    app: tauri::AppHandle,
    state: State<AppState>,
    id: String,
) -> Result<(), String> {
    let mut hist = state.history.lock().unwrap();
    hist.retain(|e| e.id != id);
    save_history(&app, &hist).map_err(|e| format!("save_history: {e:#}"))
}

#[tauri::command]
fn clear_history(app: tauri::AppHandle, state: State<AppState>) -> Result<(), String> {
    state.history.lock().unwrap().clear();
    save_history(&app, &[]).map_err(|e| format!("save_history: {e:#}"))
}

#[tauri::command]
fn copy_history_entry(
    app: tauri::AppHandle,
    state: State<AppState>,
    id: String,
) -> Result<(), String> {
    let hist = state.history.lock().unwrap();
    let entry = hist
        .iter()
        .find(|e| e.id == id)
        .ok_or_else(|| "history entry not found".to_string())?;
    use tauri_plugin_clipboard_manager::ClipboardExt;
    app.clipboard()
        .write_text(entry.text.clone())
        .map_err(|e| format!("clipboard write: {e:?}"))
}

#[tauri::command]
fn transcribe_file(
    state: State<AppState>,
    path: String,
    language: Option<String>,
) -> Result<String, String> {
    state
        .transcriber
        .transcribe(&PathBuf::from(&path), language.as_deref(), None)
        .map_err(|e| format!("{e:#}"))
}

/// Switch between low-level keyboard hook (e.g. for Caps Lock so we can
/// suppress the toggle) and the normal global-shortcut path.
fn apply_hotkey(app: &tauri::AppHandle, spec: &str) -> anyhow::Result<()> {
    #[cfg(windows)]
    {
        if let Some(vk) = crate::caps_hook::vk_for_intercept(spec) {
            unregister_all(app);
            crate::caps_hook::set_target_vk(Some(vk));
            println!("[hotkey] using low-level hook for {spec}");
            return Ok(());
        }
        crate::caps_hook::set_target_vk(None);
    }
    register_hotkey(app, spec)
}

/// Delete WAV files in the tmp dir older than `max_age_days`. Best-effort, errors ignored.
fn cleanup_old_recordings(app_data: &Path, max_age_days: u64) {
    let tmp = app_data.join("tmp");
    if !tmp.exists() {
        return;
    }
    let cutoff = match SystemTime::now().checked_sub(Duration::from_secs(max_age_days * 24 * 3600))
    {
        Some(t) => t,
        None => return,
    };
    let entries = match std::fs::read_dir(&tmp) {
        Ok(e) => e,
        Err(_) => return,
    };
    let mut deleted = 0;
    for entry in entries.flatten() {
        let path = entry.path();
        let is_wav = path
            .extension()
            .and_then(|s| s.to_str())
            .map(|s| s.eq_ignore_ascii_case("wav"))
            .unwrap_or(false);
        if !is_wav {
            continue;
        }
        if let Ok(meta) = entry.metadata() {
            if let Ok(modified) = meta.modified() {
                if modified < cutoff && std::fs::remove_file(&path).is_ok() {
                    deleted += 1;
                }
            }
        }
    }
    if deleted > 0 {
        println!("[director-general] cleaned up {deleted} old recording(s)");
    }
}

fn build_tray_menu(app: &tauri::AppHandle) -> tauri::Result<Menu<tauri::Wry>> {
    let show = MenuItem::with_id(app, "show", "Show Director General", true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "open_settings", "Settings", true, None::<&str>)?;
    let history = MenuItem::with_id(app, "open_history", "History", true, None::<&str>)?;
    let sep = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    Menu::with_items(app, &[&show, &settings, &history, &sep, &quit])
}

fn focus_main_window(app: &tauri::AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

/// Position the overlay window centred horizontally near the bottom of the
/// primary monitor. Called every time we're about to show it because the
/// monitor layout can change.
fn position_overlay(app: &tauri::AppHandle) {
    if let Some(overlay) = app.get_webview_window("overlay") {
        if let Ok(Some(monitor)) = overlay.primary_monitor() {
            let size = monitor.size();
            let scale = monitor.scale_factor();
            let win_w_logical = 180.0_f64;
            let win_h_logical = 56.0_f64;
            let win_w_phys = (win_w_logical * scale) as i32;
            let win_h_phys = (win_h_logical * scale) as i32;
            let x = (size.width as i32 / 2) - (win_w_phys / 2);
            let y = size.height as i32 - win_h_phys - (60.0 * scale) as i32;
            let _ = overlay.set_position(tauri::PhysicalPosition::new(x, y));
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            Some(vec![]),
        ))
        .setup(|app| {
            let handle = app.handle().clone();

            let app_data = ensure_app_data_dir(&handle)?;
            cleanup_old_recordings(&app_data, 7);

            let settings = load_settings(&handle);
            let history = load_history(&handle);
            println!(
                "[director-general] startup: model={:?} hotkey={:?} input_device={:?} history_entries={}",
                settings.model_path,
                settings.hotkey,
                settings.input_device,
                history.len()
            );
            let hotkey_spec = settings.hotkey.clone();
            app.manage(AppState::new(settings, history));

            // Install the low-level keyboard hook on Windows so we can intercept
            // keys like Caps Lock without letting the OS toggle their state.
            #[cfg(windows)]
            {
                let (tx, rx) = std::sync::mpsc::channel::<()>();
                crate::caps_hook::install(tx);
                let handle_hook = handle.clone();
                std::thread::Builder::new()
                    .name("dg-caps-pump".into())
                    .spawn(move || {
                        while rx.recv().is_ok() {
                            on_press(&handle_hook);
                        }
                    })
                    .ok();
            }

            // Activate the right hotkey path for the loaded spec.
            if let Err(e) = apply_hotkey(&handle, &hotkey_spec) {
                eprintln!("[director-general] apply_hotkey error: {e:#}");
            }

            // System tray icon with menu.
            let menu = build_tray_menu(app.handle())?;
            let _tray = TrayIconBuilder::with_id("main-tray")
                .tooltip("Director General — local voice typing")
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "show" => focus_main_window(app),
                    "open_settings" => {
                        focus_main_window(app);
                        let _ = app.emit("nav", "settings");
                    }
                    "open_history" => {
                        focus_main_window(app);
                        let _ = app.emit("nav", "history");
                    }
                    "quit" => {
                        app.exit(0);
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        focus_main_window(tray.app_handle());
                    }
                })
                .build(app)?;

            // Show/hide overlay window on mode changes.
            let handle_ov = handle.clone();
            handle.listen("mode-changed", move |event| {
                let payload = event.payload().trim_matches('"').to_string();
                if let Some(overlay) = handle_ov.get_webview_window("overlay") {
                    match payload.as_str() {
                        "recording" => {
                            let show = handle_ov
                                .try_state::<AppState>()
                                .map(|s| s.settings.lock().unwrap().show_overlay)
                                .unwrap_or(true);
                            if show {
                                position_overlay(&handle_ov);
                                let _ = overlay.show();
                            }
                        }
                        _ => {
                            let _ = overlay.hide();
                        }
                    }
                }
            });

            Ok(())
        })
        // Hide the main window to tray on close instead of exiting.
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == "main" {
                    let _ = window.hide();
                    api.prevent_close();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            ping,
            get_mode,
            get_settings,
            update_settings,
            get_history,
            delete_history_entry,
            clear_history,
            copy_history_entry,
            transcribe_file,
            list_input_devices,
            trigger_hotkey,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
