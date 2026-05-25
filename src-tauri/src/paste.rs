// Simulated Ctrl+V via enigo. Used after transcription completes so the text
// lands in whatever window the user had focus in before they pressed the hotkey.

use anyhow::{anyhow, Result};
use enigo::{Direction, Enigo, Key, Keyboard, Settings};

pub fn send_ctrl_v() -> Result<()> {
    let mut enigo = Enigo::new(&Settings::default())
        .map_err(|e| anyhow!("enigo init: {e:?}"))?;
    enigo
        .key(Key::Control, Direction::Press)
        .map_err(|e| anyhow!("press ctrl: {e:?}"))?;
    enigo
        .key(Key::Unicode('v'), Direction::Click)
        .map_err(|e| anyhow!("click v: {e:?}"))?;
    enigo
        .key(Key::Control, Direction::Release)
        .map_err(|e| anyhow!("release ctrl: {e:?}"))?;
    Ok(())
}
