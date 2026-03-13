use crate::controller::{ControlMode, ControlModeMeta};
use crate::dial_device::DialHaptics;
use crate::error::{Error, Result};
use crate::fake_input;

use evdev::KeyCode;

pub struct Zoom {}

impl Zoom {
    pub fn new() -> Zoom {
        Zoom {}
    }
}

impl ControlMode for Zoom {
    fn meta(&self) -> ControlModeMeta {
        ControlModeMeta {
            name: "Zoom".into(),
            icon: "zoom-in".into(),
            haptics: true,
            steps: 36,
        }
    }

    fn on_btn_press(&mut self, _: &DialHaptics) -> Result<()> {
        Ok(())
    }

    fn on_btn_release(&mut self, _haptics: &DialHaptics) -> Result<()> {
        Ok(())
    }

    fn on_dial(&mut self, _: &DialHaptics, delta: i32) -> Result<()> {
        if delta > 0 {
            eprintln!("zoom in");
            fake_input::key_click(&[KeyCode::KEY_LEFTCTRL, KeyCode::KEY_EQUAL])
                .map_err(Error::Evdev)?;
        } else {
            eprintln!("zoom out");
            fake_input::key_click(&[KeyCode::KEY_LEFTCTRL, KeyCode::KEY_MINUS])
                .map_err(Error::Evdev)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zoom_meta() {
        let meta = Zoom::new().meta();
        assert_eq!(meta.name, "Zoom");
        assert!(meta.haptics);
        assert_eq!(meta.steps, 36);
    }
}
