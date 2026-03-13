use std::io;

use evdev::uinput::VirtualDevice;
use evdev::{
    AbsoluteAxisCode, AbsInfo, AttributeSet, EventType, InputEvent, KeyCode, MiscCode, PropType,
    RelativeAxisCode, UinputAbsSetup,
};
use parking_lot::Mutex;

/// Lists every key supported by the virtual keyboard device.
/// Used both when registering the device and when parsing key names from config.
macro_rules! with_keyboard_keys {
    ($mac:ident) => {
        $mac!(
            // Letters
            KEY_A, KEY_B, KEY_C, KEY_D, KEY_E, KEY_F, KEY_G, KEY_H, KEY_I, KEY_J,
            KEY_K, KEY_L, KEY_M, KEY_N, KEY_O, KEY_P, KEY_Q, KEY_R, KEY_S, KEY_T,
            KEY_U, KEY_V, KEY_W, KEY_X, KEY_Y, KEY_Z,
            // Numbers
            KEY_0, KEY_1, KEY_2, KEY_3, KEY_4, KEY_5, KEY_6, KEY_7, KEY_8, KEY_9,
            // Function keys
            KEY_F1, KEY_F2, KEY_F3, KEY_F4, KEY_F5, KEY_F6,
            KEY_F7, KEY_F8, KEY_F9, KEY_F10, KEY_F11, KEY_F12,
            // Modifiers
            KEY_LEFTSHIFT, KEY_RIGHTSHIFT,
            KEY_LEFTCTRL, KEY_RIGHTCTRL,
            KEY_LEFTALT, KEY_RIGHTALT,
            KEY_LEFTMETA, KEY_RIGHTMETA,
            // Navigation
            KEY_UP, KEY_DOWN, KEY_LEFT, KEY_RIGHT,
            KEY_HOME, KEY_END, KEY_PAGEUP, KEY_PAGEDOWN,
            // Editing
            KEY_SPACE, KEY_ENTER, KEY_BACKSPACE, KEY_DELETE, KEY_INSERT,
            KEY_TAB, KEY_ESC, KEY_CAPSLOCK,
            // Symbols
            KEY_EQUAL, KEY_MINUS, KEY_LEFTBRACE, KEY_RIGHTBRACE,
            KEY_SEMICOLON, KEY_APOSTROPHE, KEY_GRAVE, KEY_BACKSLASH,
            KEY_COMMA, KEY_DOT, KEY_SLASH,
            // Media
            KEY_MUTE, KEY_VOLUMEUP, KEY_VOLUMEDOWN,
            KEY_PLAYPAUSE, KEY_NEXTSONG, KEY_PREVIOUSSONG, KEY_STOPCD,
            // System / misc
            KEY_BRIGHTNESSUP, KEY_BRIGHTNESSDOWN,
            KEY_PRINT, KEY_SCROLLLOCK, KEY_PAUSE,
            KEY_SLEEP, KEY_WAKEUP
        )
    };
}

/// Returns the `KeyCode` for a key name string (e.g. `"KEY_VOLUMEUP"`), or
/// `None` if the name is not supported by the virtual keyboard device.
pub fn parse_key_code(name: &str) -> Option<KeyCode> {
    macro_rules! do_match {
        ($($k:ident),+) => {
            match name {
                $(stringify!($k) => Some(KeyCode::$k),)+
                _ => None,
            }
        };
    }
    with_keyboard_keys!(do_match)
}

// this should be a fairly high number, as the axis is from 0..(MT_BASELINE*2)
const MT_BASELINE: i32 = std::i32::MAX / 8;
// higher = slower scrolling
const MT_SENSITIVITY: i32 = 48;

pub struct FakeInputs {
    keyboard: Mutex<VirtualDevice>,
    touchpad: Mutex<VirtualDevice>,
}

lazy_static::lazy_static! {
    pub static ref FAKE_INPUTS: FakeInputs = {
        let keyboard = (|| -> io::Result<_> {
            macro_rules! build_set {
                ($($k:ident),+) => { AttributeSet::from_iter([$(KeyCode::$k,)+]) };
            }
            let keys = with_keyboard_keys!(build_set);
            let rel_axes = AttributeSet::from_iter([
                RelativeAxisCode::REL_WHEEL,
                RelativeAxisCode::REL_WHEEL_HI_RES,
            ]);
            let msc_codes = AttributeSet::from_iter([MiscCode::MSC_SCAN]);

            let device = VirtualDevice::builder()?
                .name("Surface Dial Virtual Keyboard/Mouse")
                .with_keys(&keys)?
                .with_relative_axes(&rel_axes)?
                .with_msc(&msc_codes)?
                .build()?;

            Ok(Mutex::new(device))
        })()
        .expect("failed to install virtual mouse/keyboard device");

        let touchpad = (|| -> io::Result<_> {
            let keys = AttributeSet::from_iter([
                KeyCode::BTN_LEFT,
                KeyCode::BTN_TOOL_FINGER,
                KeyCode::BTN_TOUCH,
                KeyCode::BTN_TOOL_DOUBLETAP,
                KeyCode::BTN_TOOL_TRIPLETAP,
                KeyCode::BTN_TOOL_QUADTAP,
            ]);
            let props = AttributeSet::from_iter([PropType::BUTTONPAD, PropType::POINTER]);

            let device = VirtualDevice::builder()?
                .name("Surface Dial Virtual Touchpad")
                .with_properties(&props)?
                .with_keys(&keys)?
                .with_absolute_axis(&UinputAbsSetup::new(
                    AbsoluteAxisCode::ABS_MT_SLOT,
                    AbsInfo::new(0, 0, 4, 0, 0, 0),
                ))?
                .with_absolute_axis(&UinputAbsSetup::new(
                    AbsoluteAxisCode::ABS_MT_TRACKING_ID,
                    AbsInfo::new(0, 0, 65535, 0, 0, 0),
                ))?
                .with_absolute_axis(&UinputAbsSetup::new(
                    AbsoluteAxisCode::ABS_MT_POSITION_X,
                    AbsInfo::new(MT_BASELINE, 0, MT_BASELINE * 2, 0, 0, MT_SENSITIVITY),
                ))?
                .with_absolute_axis(&UinputAbsSetup::new(
                    AbsoluteAxisCode::ABS_MT_POSITION_Y,
                    AbsInfo::new(MT_BASELINE, 0, MT_BASELINE * 2, 0, 0, MT_SENSITIVITY),
                ))?
                .with_absolute_axis(&UinputAbsSetup::new(
                    AbsoluteAxisCode::ABS_X,
                    AbsInfo::new(MT_BASELINE, 0, MT_BASELINE * 2, 0, 0, MT_SENSITIVITY),
                ))?
                .with_absolute_axis(&UinputAbsSetup::new(
                    AbsoluteAxisCode::ABS_Y,
                    AbsInfo::new(MT_BASELINE, 0, MT_BASELINE * 2, 0, 0, MT_SENSITIVITY),
                ))?
                .build()?;

            Ok(Mutex::new(device))
        })()
        .expect("failed to install virtual touchpad device");

        // HACK: give the kernel a chance to register the new devices. If this
        // line is omitted, the first fake input is likely to be dropped.
        std::thread::sleep(std::time::Duration::from_millis(500));

        FakeInputs { keyboard, touchpad }
    };
}

fn abs_event(code: AbsoluteAxisCode, value: i32) -> InputEvent {
    InputEvent::new(EventType::ABSOLUTE.0, code.0, value)
}

fn key_event(code: KeyCode, value: i32) -> InputEvent {
    InputEvent::new(EventType::KEY.0, code.0, value)
}

pub fn key_click(keys: &[KeyCode]) -> io::Result<()> {
    key_press(keys)?;
    key_release(keys)?;
    Ok(())
}

pub fn key_press(keys: &[KeyCode]) -> io::Result<()> {
    let events: Vec<InputEvent> = keys
        .iter()
        .map(|key| InputEvent::new(EventType::KEY.0, key.0, 1))
        .collect();
    FAKE_INPUTS.keyboard.lock().emit(&events)?;
    Ok(())
}

pub fn key_release(keys: &[KeyCode]) -> io::Result<()> {
    let events: Vec<InputEvent> = keys
        .iter()
        .map(|key| InputEvent::new(EventType::KEY.0, key.0, 0))
        .collect();
    FAKE_INPUTS.keyboard.lock().emit(&events)?;
    Ok(())
}

pub fn scroll_step(dir: ScrollStep) -> io::Result<()> {
    let events = [
        InputEvent::new(
            EventType::RELATIVE.0,
            RelativeAxisCode::REL_WHEEL.0,
            match dir {
                ScrollStep::Down => -1,
                ScrollStep::Up => 1,
            },
        ),
        InputEvent::new(
            EventType::RELATIVE.0,
            RelativeAxisCode::REL_WHEEL_HI_RES.0,
            match dir {
                ScrollStep::Down => -120,
                ScrollStep::Up => 120,
            },
        ),
    ];
    FAKE_INPUTS.keyboard.lock().emit(&events)?;
    Ok(())
}

pub fn scroll_mt_start() -> io::Result<()> {
    let mut touchpad = FAKE_INPUTS.touchpad.lock();

    touchpad.emit(&[
        abs_event(AbsoluteAxisCode::ABS_MT_SLOT, 0),
        abs_event(AbsoluteAxisCode::ABS_MT_TRACKING_ID, 1),
        abs_event(AbsoluteAxisCode::ABS_MT_POSITION_X, MT_BASELINE),
        abs_event(AbsoluteAxisCode::ABS_MT_POSITION_Y, MT_BASELINE),
        key_event(KeyCode::BTN_TOUCH, 1),
        key_event(KeyCode::BTN_TOOL_FINGER, 1),
        abs_event(AbsoluteAxisCode::ABS_X, MT_BASELINE),
        abs_event(AbsoluteAxisCode::ABS_Y, MT_BASELINE),
    ])?;

    touchpad.emit(&[
        abs_event(AbsoluteAxisCode::ABS_MT_SLOT, 1),
        abs_event(AbsoluteAxisCode::ABS_MT_TRACKING_ID, 2),
        abs_event(AbsoluteAxisCode::ABS_MT_POSITION_X, MT_BASELINE / 2),
        abs_event(AbsoluteAxisCode::ABS_MT_POSITION_Y, MT_BASELINE),
        key_event(KeyCode::BTN_TOOL_FINGER, 0),
        key_event(KeyCode::BTN_TOOL_DOUBLETAP, 1),
    ])?;

    Ok(())
}

pub fn scroll_mt_step(delta: i32) -> io::Result<()> {
    FAKE_INPUTS.touchpad.lock().emit(&[
        abs_event(AbsoluteAxisCode::ABS_MT_SLOT, 0),
        abs_event(AbsoluteAxisCode::ABS_MT_POSITION_Y, MT_BASELINE + delta),
        abs_event(AbsoluteAxisCode::ABS_MT_SLOT, 1),
        abs_event(AbsoluteAxisCode::ABS_MT_POSITION_Y, MT_BASELINE + delta),
        abs_event(AbsoluteAxisCode::ABS_Y, MT_BASELINE + delta),
    ])?;
    Ok(())
}

pub fn scroll_mt_end() -> io::Result<()> {
    FAKE_INPUTS.touchpad.lock().emit(&[
        abs_event(AbsoluteAxisCode::ABS_MT_SLOT, 0),
        abs_event(AbsoluteAxisCode::ABS_MT_TRACKING_ID, -1),
        abs_event(AbsoluteAxisCode::ABS_MT_SLOT, 1),
        abs_event(AbsoluteAxisCode::ABS_MT_TRACKING_ID, -1),
        key_event(KeyCode::BTN_TOUCH, 0),
        key_event(KeyCode::BTN_TOOL_DOUBLETAP, 0),
    ])?;
    Ok(())
}

pub enum ScrollStep {
    Up,
    Down,
}
