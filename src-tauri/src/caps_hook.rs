// Low-level keyboard hook for Windows.
//
// `tauri-plugin-global-shortcut` uses `RegisterHotKey`, which only delivers a
// notification — it can't *consume* a key. For keys like Caps Lock the user
// expects the toggling behaviour to be *suppressed* when the key is used as
// the recording hotkey. We achieve that with a WH_KEYBOARD_LL hook running on
// its own thread with a message pump. Returning a non-zero `LRESULT` from the
// hook proc swallows the keystroke before Windows processes it.
//
// The hook lives for the lifetime of the process. `set_target_vk` switches
// between "intercept this VK code" and "pass everything through".

#![cfg(windows)]

use std::sync::mpsc::Sender;
use std::sync::{Mutex, OnceLock};

use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::Input::KeyboardAndMouse::VK_CAPITAL;
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, GetMessageW, SetWindowsHookExW, TranslateMessage,
    HC_ACTION, KBDLLHOOKSTRUCT, MSG, WH_KEYBOARD_LL, WM_KEYDOWN, WM_SYSKEYDOWN,
};

struct HookState {
    target_vk: Mutex<Option<u32>>,
    tx: Mutex<Option<Sender<()>>>,
}

static HOOK_STATE: OnceLock<HookState> = OnceLock::new();

unsafe extern "system" fn keyboard_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code == HC_ACTION as i32 {
        let kb = unsafe { &*(lparam.0 as *const KBDLLHOOKSTRUCT) };
        if let Some(state) = HOOK_STATE.get() {
            let target = *state.target_vk.lock().unwrap();
            if let Some(vk) = target {
                if kb.vkCode == vk {
                    let msg = wparam.0 as u32;
                    if msg == WM_KEYDOWN || msg == WM_SYSKEYDOWN {
                        if let Some(tx) = state.tx.lock().unwrap().as_ref() {
                            let _ = tx.send(());
                        }
                    }
                    // Consume both press and release so the OS doesn't toggle
                    // the Caps Lock LED or apply any other side effect.
                    return LRESULT(1);
                }
            }
        }
    }
    unsafe { CallNextHookEx(None, code, wparam, lparam) }
}

/// Install the hook on a dedicated thread with a message pump and remember
/// `tx` so we can notify the rest of the app when our target key fires.
///
/// Safe to call once at startup. Subsequent calls are no-ops.
pub fn install(tx: Sender<()>) {
    if HOOK_STATE.get().is_some() {
        return;
    }
    let _ = HOOK_STATE.set(HookState {
        target_vk: Mutex::new(None),
        tx: Mutex::new(Some(tx)),
    });

    std::thread::Builder::new()
        .name("dg-caps-hook".into())
        .spawn(|| unsafe {
            let hook = match SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_proc), None, 0) {
                Ok(h) => h,
                Err(e) => {
                    eprintln!("[caps_hook] SetWindowsHookExW failed: {e:?}");
                    return;
                }
            };

            let mut msg = MSG::default();
            loop {
                let r = GetMessageW(&mut msg, None, 0, 0);
                if r.0 <= 0 {
                    break;
                }
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
            let _ = windows::Win32::UI::WindowsAndMessaging::UnhookWindowsHookEx(hook);
        })
        .expect("failed to spawn caps-hook thread");
}

/// Switch the hook to intercept the given virtual key code, or `None` to
/// disable interception (everything passes through).
pub fn set_target_vk(vk: Option<u32>) {
    if let Some(state) = HOOK_STATE.get() {
        *state.target_vk.lock().unwrap() = vk;
    }
}

/// Map our hotkey spec strings (the same format the `global_hotkey` crate
/// uses) onto a Win32 virtual key when we want low-level interception.
///
/// Returns `Some(vk)` for keys that benefit from interception (today: just
/// `CapsLock`). Anything else returns `None` and falls through to the normal
/// global-shortcut path.
pub fn vk_for_intercept(spec: &str) -> Option<u32> {
    match spec.trim() {
        "CapsLock" | "Capital" => Some(VK_CAPITAL.0 as u32),
        _ => None,
    }
}
