//! YAML-driven control modes.
//!
//! Place a `modes.yaml` file in the daemon's config directory
//! (`~/.config/com.prilik/surface-dial-daemon/modes.yaml`) to define custom
//! modes without writing Rust code.
//!
//! ## Example `modes.yaml`
//!
//! ```yaml
//! - name: "Brightness"
//!   icon: "display-brightness-symbolic"  # FreeDesktop icon name or file:// path
//!   haptics: true
//!   steps: 36                            # haptic click divisions (0-3600)
//!   on_dial_cw:  ["KEY_BRIGHTNESSUP"]
//!   on_dial_ccw: ["KEY_BRIGHTNESSDOWN"]
//!
//! - name: "Undo / Redo"
//!   icon: "edit-undo"
//!   haptics: true
//!   steps: 36
//!   on_dial_cw:  ["KEY_LEFTCTRL", "KEY_Y"]
//!   on_dial_ccw: ["KEY_LEFTCTRL", "KEY_Z"]
//!   on_btn_release: ["KEY_LEFTCTRL", "KEY_S"]
//! ```
//!
//! Each action field is a list of keys pressed **simultaneously** (a chord).
//! An empty list (or omitting the field entirely) means no action.
//!
//! ## Supported key names
//!
//! Any key listed by [`crate::fake_input::parse_key_code`] is valid:
//! letters (`KEY_A`–`KEY_Z`), digits (`KEY_0`–`KEY_9`), function keys
//! (`KEY_F1`–`KEY_F12`), modifiers, navigation, media, brightness, and more.

use serde::Deserialize;

use crate::controller::{ControlMode, ControlModeMeta};
use crate::dial_device::DialHaptics;
use crate::error::{Error, Result};
use crate::fake_input;
use evdev::KeyCode;

// ---------------------------------------------------------------------------
// YAML schema
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct ModeSpec {
    name: String,
    #[serde(default = "default_icon")]
    icon: String,
    #[serde(default)]
    haptics: bool,
    #[serde(default = "default_steps")]
    steps: u16,
    #[serde(default)]
    on_btn_press: Vec<String>,
    #[serde(default)]
    on_btn_release: Vec<String>,
    /// Keys sent on clockwise rotation (positive delta).
    #[serde(default)]
    on_dial_cw: Vec<String>,
    /// Keys sent on counter-clockwise rotation (negative delta).
    #[serde(default)]
    on_dial_ccw: Vec<String>,
}

fn default_icon() -> String {
    "input-keyboard".into()
}

fn default_steps() -> u16 {
    36
}

// ---------------------------------------------------------------------------
// ConfigMode
// ---------------------------------------------------------------------------

pub struct ConfigMode {
    name: String,
    icon: String,
    haptics: bool,
    steps: u16,
    btn_press: Vec<KeyCode>,
    btn_release: Vec<KeyCode>,
    dial_cw: Vec<KeyCode>,
    dial_ccw: Vec<KeyCode>,
}

impl ConfigMode {
    fn from_spec(spec: ModeSpec) -> Result<Self> {
        Ok(ConfigMode {
            btn_press: resolve_keys("on_btn_press", &spec.on_btn_press)?,
            btn_release: resolve_keys("on_btn_release", &spec.on_btn_release)?,
            dial_cw: resolve_keys("on_dial_cw", &spec.on_dial_cw)?,
            dial_ccw: resolve_keys("on_dial_ccw", &spec.on_dial_ccw)?,
            name: spec.name,
            icon: spec.icon,
            haptics: spec.haptics,
            steps: spec.steps,
        })
    }
}

fn resolve_keys(field: &str, names: &[String]) -> Result<Vec<KeyCode>> {
    names
        .iter()
        .map(|n| {
            fake_input::parse_key_code(n).ok_or_else(|| {
                Error::ConfigFile(format!(
                    "modes.yaml: unknown key {:?} in field `{}`",
                    n, field
                ))
            })
        })
        .collect()
}

impl ControlMode for ConfigMode {
    fn meta(&self) -> ControlModeMeta {
        ControlModeMeta {
            name: self.name.clone(),
            icon: self.icon.clone(),
            haptics: self.haptics,
            steps: self.steps,
        }
    }

    fn on_btn_press(&mut self, _: &DialHaptics) -> Result<()> {
        if !self.btn_press.is_empty() {
            fake_input::key_click(&self.btn_press).map_err(Error::Evdev)?;
        }
        Ok(())
    }

    fn on_btn_release(&mut self, _: &DialHaptics) -> Result<()> {
        if !self.btn_release.is_empty() {
            fake_input::key_click(&self.btn_release).map_err(Error::Evdev)?;
        }
        Ok(())
    }

    fn on_dial(&mut self, _: &DialHaptics, delta: i32) -> Result<()> {
        let keys = if delta > 0 { &self.dial_cw } else { &self.dial_ccw };
        if !keys.is_empty() {
            fake_input::key_click(keys).map_err(Error::Evdev)?;
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Loading
// ---------------------------------------------------------------------------

/// Reads `modes.yaml` from the daemon config directory and returns a
/// `ConfigMode` for every entry.  If the file does not exist, returns an
/// empty `Vec`.  Errors in the file are propagated so the daemon can report
/// them clearly at startup.
pub fn load_yaml_modes() -> Result<Vec<Box<dyn ControlMode>>> {
    let path = crate::config::config_dir()?.join("modes.yaml");

    if !path.exists() {
        return Ok(Vec::new());
    }

    let content = std::fs::read_to_string(&path)
        .map_err(|e| Error::ConfigFile(format!("could not read modes.yaml: {}", e)))?;

    let specs: Vec<ModeSpec> = serde_yaml::from_str(&content)
        .map_err(|e| Error::ConfigFile(format!("could not parse modes.yaml: {}", e)))?;

    specs
        .into_iter()
        .map(|s| ConfigMode::from_spec(s).map(|m| Box::new(m) as Box<dyn ControlMode>))
        .collect()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_yaml(yaml: &str) -> Vec<ModeSpec> {
        serde_yaml::from_str(yaml).expect("valid yaml")
    }

    #[test]
    fn minimal_spec_deserializes() {
        let specs = parse_yaml("- name: \"Test\"");
        assert_eq!(specs.len(), 1);
        assert_eq!(specs[0].name, "Test");
        assert_eq!(specs[0].haptics, false);
        assert_eq!(specs[0].steps, 36); // default
        assert!(specs[0].on_dial_cw.is_empty());
    }

    #[test]
    fn full_spec_deserializes() {
        let specs = parse_yaml(
            r#"
- name: "Brightness"
  icon: "display-brightness-symbolic"
  haptics: true
  steps: 72
  on_btn_release: ["KEY_SLEEP"]
  on_dial_cw:  ["KEY_BRIGHTNESSUP"]
  on_dial_ccw: ["KEY_BRIGHTNESSDOWN"]
"#,
        );
        let s = &specs[0];
        assert_eq!(s.name, "Brightness");
        assert_eq!(s.icon, "display-brightness-symbolic");
        assert!(s.haptics);
        assert_eq!(s.steps, 72);
        assert_eq!(s.on_btn_release, ["KEY_SLEEP"]);
        assert_eq!(s.on_dial_cw, ["KEY_BRIGHTNESSUP"]);
        assert_eq!(s.on_dial_ccw, ["KEY_BRIGHTNESSDOWN"]);
    }

    #[test]
    fn multiple_modes_deserialize() {
        let specs = parse_yaml(
            r#"
- name: "Mode A"
- name: "Mode B"
  steps: 90
"#,
        );
        assert_eq!(specs.len(), 2);
        assert_eq!(specs[1].steps, 90);
    }

    #[test]
    fn resolve_known_keys_succeeds() {
        let keys = resolve_keys("test", &["KEY_LEFTCTRL".into(), "KEY_Z".into()]).unwrap();
        assert_eq!(keys, vec![KeyCode::KEY_LEFTCTRL, KeyCode::KEY_Z]);
    }

    #[test]
    fn resolve_unknown_key_errors() {
        let result = resolve_keys("test", &["KEY_UNKNOWN_XYZ".into()]);
        assert!(result.is_err());
        let msg = format!("{}", result.unwrap_err());
        assert!(msg.contains("KEY_UNKNOWN_XYZ"));
    }

    #[test]
    fn parse_key_code_roundtrip() {
        // spot-check a sample of keys across categories
        for name in &[
            "KEY_A", "KEY_Z", "KEY_0", "KEY_9",
            "KEY_F1", "KEY_F12",
            "KEY_LEFTSHIFT", "KEY_RIGHTCTRL",
            "KEY_UP", "KEY_PAGEDOWN",
            "KEY_ENTER", "KEY_ESC", "KEY_TAB",
            "KEY_MUTE", "KEY_VOLUMEUP", "KEY_PLAYPAUSE",
            "KEY_BRIGHTNESSUP", "KEY_BRIGHTNESSDOWN",
        ] {
            assert!(
                fake_input::parse_key_code(name).is_some(),
                "{name} should be a supported key"
            );
        }
    }

    #[test]
    fn parse_key_code_unknown_returns_none() {
        assert!(fake_input::parse_key_code("NOT_A_KEY").is_none());
        assert!(fake_input::parse_key_code("").is_none());
    }
}
