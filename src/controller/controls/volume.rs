use crate::controller::{ControlMode, ControlModeMeta};
use crate::dial_device::DialHaptics;
use crate::error::{Error, Result};
use crate::fake_input;

use evdev::KeyCode;

pub struct Volume {}

impl Volume {
    pub fn new() -> Volume {
        Volume {}
    }
}

impl ControlMode for Volume {
    fn meta(&self) -> ControlModeMeta {
        ControlModeMeta {
            name: "Volume".into(),
            icon: "audio-volume-high".into(),
            haptics: true,
            steps: 36 * 2,
        }
    }

    fn on_btn_press(&mut self, _: &DialHaptics) -> Result<()> {
        Ok(())
    }

    fn on_btn_release(&mut self, _: &DialHaptics) -> Result<()> {
        eprintln!("mute");
        fake_input::key_click(&[KeyCode::KEY_MUTE]).map_err(Error::Evdev)?;
        Ok(())
    }

    fn on_dial(&mut self, _: &DialHaptics, delta: i32) -> Result<()> {
        if delta > 0 {
            eprintln!("volume up");
            fake_input::key_click(&[KeyCode::KEY_LEFTSHIFT, KeyCode::KEY_VOLUMEUP])
                .map_err(Error::Evdev)?;
        } else {
            eprintln!("volume down");
            fake_input::key_click(&[KeyCode::KEY_LEFTSHIFT, KeyCode::KEY_VOLUMEDOWN])
                .map_err(Error::Evdev)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn volume_meta() {
        let meta = Volume::new().meta();
        assert_eq!(meta.name, "Volume");
        assert!(meta.haptics);
        assert_eq!(meta.steps, 72);
    }
}
