use crate::controller::{ControlMode, ControlModeMeta};
use crate::dial_device::DialHaptics;
use crate::error::{Error, Result};
use crate::fake_input;

use evdev::KeyCode;

pub struct Media {}

impl Media {
    pub fn new() -> Media {
        Media {}
    }
}

impl ControlMode for Media {
    fn meta(&self) -> ControlModeMeta {
        ControlModeMeta {
            name: "Media".into(),
            icon: "applications-multimedia".into(),
            haptics: true,
            steps: 36,
        }
    }

    fn on_btn_press(&mut self, _: &DialHaptics) -> Result<()> {
        Ok(())
    }

    fn on_btn_release(&mut self, _: &DialHaptics) -> Result<()> {
        fake_input::key_click(&[KeyCode::KEY_PLAYPAUSE]).map_err(Error::Evdev)?;
        Ok(())
    }

    fn on_dial(&mut self, _: &DialHaptics, delta: i32) -> Result<()> {
        if delta > 0 {
            eprintln!("next song");
            fake_input::key_click(&[KeyCode::KEY_NEXTSONG]).map_err(Error::Evdev)?;
        } else {
            eprintln!("last song");
            fake_input::key_click(&[KeyCode::KEY_PREVIOUSSONG]).map_err(Error::Evdev)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn media_meta() {
        let meta = Media::new().meta();
        assert_eq!(meta.name, "Media");
        assert!(meta.haptics);
        assert_eq!(meta.steps, 36);
    }
}
