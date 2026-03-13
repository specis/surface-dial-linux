# surface-dial-linux

A Linux userspace controller for the [Microsoft Surface Dial](https://www.microsoft.com/en-us/p/surface-dial/925r551sktgn). Requires Linux Kernel 4.19 or higher.

## Overview

`surface-dial-daemon` receives raw events from the Surface Dial and translates them into conventional input events (key presses, scroll wheel, haptic feedback, etc.).

The daemon uses FreeDesktop notifications to provide visual feedback when switching between modes.

![](notif-demo.gif)

### Operating Modes

Hold the button for ~750 ms to open the meta-menu (shown via desktop notification), which lets you switch modes on the fly. The last selected mode is saved to disk, so if you only ever want one mode you can set it and forget it.

Modes in **bold** are **experimental** — they work most of the time but could use more polish.

| Mode                         | Click             | Rotate               | Notes                                                                                   |
| ---------------------------- | ----------------- | -------------------- | --------------------------------------------------------------------------------------- |
| Scroll                       | —                 | Scroll               | Fakes chunky mouse-wheel scrolling <sup>1</sup>                                         |
| **Scroll (Fake Multitouch)** | Reset touch event | Scroll               | Fakes smooth two-finger scrolling                                                       |
| Zoom                         | —                 | Zoom in/out          | Sends Ctrl+= / Ctrl+−                                                                   |
| Volume                       | Mute              | Volume up/down       |                                                                                         |
| Media                        | Play/Pause        | Next/Prev track      |                                                                                         |
| Media + Volume               | Play/Pause        | Volume up/down       | Double-click = Next Track                                                               |
| **Paddle Controller**        | Space             | Left/Right arrow key | Play [arkanoid](https://www.google.com/search?q=arkanoid+paddle) as the devs intended! |
| _Your custom modes…_         | _configurable_    | _configurable_       | Defined in `modes.yaml` — see below                                                     |

<sup>1</sup> Most Linux programs still only handle the older, chunky scroll-wheel events. See [this post](https://who-t.blogspot.com/2020/04/high-resolution-wheel-scrolling-in.html) for background.

---

## Custom Modes via `modes.yaml`

You can add your own modes **without writing any Rust code** by dropping a `modes.yaml` file into the daemon's config directory:

```
~/.config/com.prilik/surface-dial-daemon/modes.yaml
```

Each entry in the file becomes a new mode that appears at the end of the meta-menu after the built-in modes.

### Format

```yaml
- name: "Brightness"
  icon: "display-brightness-symbolic"   # FreeDesktop icon name or file:// path
  haptics: true
  steps: 36                             # haptic click divisions (0–3600)
  on_dial_cw:  ["KEY_BRIGHTNESSUP"]
  on_dial_ccw: ["KEY_BRIGHTNESSDOWN"]

- name: "Undo / Redo"
  icon: "edit-undo"
  haptics: true
  steps: 36
  on_dial_cw:  ["KEY_LEFTCTRL", "KEY_Y"]
  on_dial_ccw: ["KEY_LEFTCTRL", "KEY_Z"]
  on_btn_release: ["KEY_LEFTCTRL", "KEY_S"]

- name: "Tab Switch"
  icon: "view-paged"
  haptics: false
  steps: 90
  on_dial_cw:  ["KEY_LEFTCTRL", "KEY_TAB"]
  on_dial_ccw: ["KEY_LEFTCTRL", "KEY_LEFTSHIFT", "KEY_TAB"]
```

### Fields

| Field           | Required | Default            | Description                                           |
| --------------- | -------- | ------------------ | ----------------------------------------------------- |
| `name`          | yes      | —                  | Display name shown in the meta-menu notification      |
| `icon`          | no       | `input-keyboard`   | FreeDesktop icon name or `file:///path/to/icon.png`   |
| `haptics`       | no       | `false`            | Whether the dial clicks when rotating                 |
| `steps`         | no       | `36`               | Number of haptic divisions per full rotation (0–3600) |
| `on_btn_press`  | no       | _(no action)_      | Keys sent when the button is pressed                  |
| `on_btn_release`| no       | _(no action)_      | Keys sent when the button is released                 |
| `on_dial_cw`    | no       | _(no action)_      | Keys sent on clockwise rotation                       |
| `on_dial_ccw`   | no       | _(no action)_      | Keys sent on counter-clockwise rotation               |

Each action value is a YAML list of key names pressed **simultaneously** as a chord. An empty list or omitting the field entirely means no action is taken.

### Supported key names

Letters (`KEY_A`–`KEY_Z`), digits (`KEY_0`–`KEY_9`), function keys (`KEY_F1`–`KEY_F12`), and:

| Category    | Keys                                                                                           |
| ----------- | ---------------------------------------------------------------------------------------------- |
| Modifiers   | `KEY_LEFTSHIFT` `KEY_RIGHTSHIFT` `KEY_LEFTCTRL` `KEY_RIGHTCTRL` `KEY_LEFTALT` `KEY_RIGHTALT` `KEY_LEFTMETA` `KEY_RIGHTMETA` |
| Navigation  | `KEY_UP` `KEY_DOWN` `KEY_LEFT` `KEY_RIGHT` `KEY_HOME` `KEY_END` `KEY_PAGEUP` `KEY_PAGEDOWN`   |
| Editing     | `KEY_SPACE` `KEY_ENTER` `KEY_BACKSPACE` `KEY_DELETE` `KEY_INSERT` `KEY_TAB` `KEY_ESC` `KEY_CAPSLOCK` |
| Symbols     | `KEY_EQUAL` `KEY_MINUS` `KEY_LEFTBRACE` `KEY_RIGHTBRACE` `KEY_SEMICOLON` `KEY_APOSTROPHE` `KEY_GRAVE` `KEY_BACKSLASH` `KEY_COMMA` `KEY_DOT` `KEY_SLASH` |
| Media       | `KEY_MUTE` `KEY_VOLUMEUP` `KEY_VOLUMEDOWN` `KEY_PLAYPAUSE` `KEY_NEXTSONG` `KEY_PREVIOUSSONG` `KEY_STOPCD` |
| System      | `KEY_BRIGHTNESSUP` `KEY_BRIGHTNESSDOWN` `KEY_PRINT` `KEY_SCROLLLOCK` `KEY_PAUSE` `KEY_SLEEP` `KEY_WAKEUP` |

### What YAML modes can't do

Some built-in modes need timing logic or special output that can't be expressed as simple key chords:

- **Scroll wheel output** — use the built-in Scroll or Scroll (Fake Multitouch) modes
- **Double-click detection** — use the built-in Media + Volume mode
- **Velocity-based control** — use the built-in Paddle Controller mode

For anything beyond what YAML supports, see [Adding a coded mode](#adding-a-coded-mode) below.

---

## Building

Building `surface-dial-daemon` requires:

- Linux Kernel 4.19 or higher
- A recent Rust toolchain (install via [rustup](https://rustup.rs/))
- `libudev`, `libevdev`, `hidapi`

```bash
# e.g. on Ubuntu / Debian
sudo apt install libevdev-dev libhidapi-dev libudev-dev
```

On some Ubuntu versions you may also need:

```bash
sudo apt install librust-libdbus-sys-dev
```

Then build with Cargo:

```bash
cargo build --release
```

The binary is placed at `target/release/surface-dial-daemon`.

---

## Running

The daemon handles dial disconnect/reconnect automatically, so it can run indefinitely in the background.

**Important:** run the daemon as a _user process_, not as root. It needs access to the user D-Bus session to send notifications.

During development:

```bash
cargo run
```

---

## Installation

The following steps have been tested on Ubuntu 20.04/20.10.

```bash
# Build and install the binary to ~/.cargo/bin/
cargo install --path .

# Add yourself to the /dev/input group (usually `input` or `plugdev`)
sudo gpasswd -a $(whoami) $(stat -c "%G" /dev/input/event0)

# Install the systemd user service
mkdir -p ~/.config/systemd/user/
cp ./install/surface-dial.service ~/.config/systemd/user/surface-dial.service

# Install udev rules (grants access to /dev/uinput and the dial's hidraw device)
sudo cp ./install/10-uinput.rules /etc/udev/rules.d/10-uinput.rules
sudo cp ./install/10-surface-dial.rules /etc/udev/rules.d/10-surface-dial.rules

# Reload systemd and udev
systemctl --user daemon-reload
sudo udevadm control --reload

# Enable and start the service
systemctl --user enable surface-dial.service
systemctl --user start surface-dial.service
```

Check the service status with:

```bash
systemctl --user status surface-dial.service
```

You may need to reboot for group memberships and udev rules to take effect.

If Bluetooth pairing fails, try setting `DisableSecurity=true` in `/etc/bluetooth/network.conf`.

---

## Adding a coded mode

YAML modes cover the common case of mapping dial events to key chords. For anything more sophisticated — scroll wheel output, timing-based logic, velocity curves — implement the `ControlMode` trait directly in Rust:

1. Create `src/controller/controls/my_mode.rs` and implement `ControlMode`:

```rust
use crate::controller::{ControlMode, ControlModeMeta};
use crate::dial_device::DialHaptics;
use crate::error::Result;
use crate::fake_input;
use evdev::KeyCode;

pub struct MyMode;

impl MyMode {
    pub fn new() -> MyMode { MyMode }
}

impl ControlMode for MyMode {
    fn meta(&self) -> ControlModeMeta {
        ControlModeMeta {
            name: "My Mode".into(),
            icon: "input-keyboard".into(),
            haptics: true,
            steps: 36,
        }
    }

    fn on_btn_press(&mut self, _: &DialHaptics) -> Result<()> { Ok(()) }

    fn on_btn_release(&mut self, _: &DialHaptics) -> Result<()> {
        fake_input::key_click(&[KeyCode::KEY_ENTER]).map_err(crate::error::Error::Evdev)
    }

    fn on_dial(&mut self, _: &DialHaptics, delta: i32) -> Result<()> {
        if delta > 0 {
            fake_input::key_click(&[KeyCode::KEY_RIGHT]).map_err(crate::error::Error::Evdev)?;
        } else {
            fake_input::key_click(&[KeyCode::KEY_LEFT]).map_err(crate::error::Error::Evdev)?;
        }
        Ok(())
    }
}
```

2. Re-export it from `src/controller/controls/mod.rs`:

```rust
mod my_mode;
pub use self::my_mode::*;
```

3. Add it to the mode list in `src/main.rs`:

```rust
Box::new(controller::controls::MyMode::new()),
```

If you build something useful, consider opening a PR!

---

## Implementation Notes

Core functionality is provided by:

- **`libudev`** — monitors when the dial connects/disconnects
- **`libevdev`** — reads raw events from `/dev/input/eventXX` and emits fake input via `/dev/uinput`
- **`hidapi`** — configures dial sensitivity and haptics over HID
- **`notify-rust`** — sends desktop notifications over D-Bus
- **`serde` + `serde_yaml`** — parses `modes.yaml` for custom modes

The daemon uses threads and `mpsc` channels for non-blocking event handling. Each subsystem (event reading, haptics, paddle velocity, double-click detection) runs in its own thread and communicates via channels. All the device-level complexity is hidden behind the `ControlMode` trait — mode implementations only see clean `on_dial`, `on_btn_press`, and `on_btn_release` callbacks.

---

## Feature Roadmap

- [x] Raw Surface Dial event handling
- [x] Haptic feedback
- [x] Built-in operating modes
- [x] YAML-configurable custom modes
- [x] On-the-fly mode switching via long-press meta-menu
- [x] Last-selected mode persistence
- [x] Graceful disconnect/reconnect handling
- [x] Visual feedback via FreeDesktop notifications
- [ ] Config file for adjusting timings (long-press timeout, double-click window, etc.)
- [ ] Custom mode ordering in the meta-menu
- [ ] Context-sensitive mode switching (based on the active application)
- [ ] Windows-like wheel overlay UI
- [ ] Packaging pipeline (deb/rpm)
