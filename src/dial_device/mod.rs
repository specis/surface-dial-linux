use std::sync::mpsc;
use std::time::Duration;

use crate::error::{Error, Result};

mod events;
mod haptics;

use haptics::{DialHapticsWorker, DialHapticsWorkerMsg};

pub use haptics::DialHaptics;

/// Encapsulates all the the nitty-gritty (and pretty gnarly) device handling
/// code, exposing a simple interface to wait for incoming [`DialEvent`]s.
pub struct DialDevice {
    // configurable constants
    long_press_timeout: Duration,

    // handles
    haptics: DialHaptics,
    events: mpsc::Receiver<events::RawInputEvent>,

    // mutable state
    possible_long_press: bool,
}

#[derive(Debug)]
pub struct DialEvent {
    pub time: Duration,
    pub kind: DialEventKind,
}

#[derive(Debug)]
pub enum DialEventKind {
    Connect,
    Disconnect,

    Ignored,
    ButtonPress,
    ButtonRelease,
    Dial(i32),

    /// NOTE: this is a synthetic event, and is _not_ directly provided by the
    /// dial itself.
    ButtonLongPress,
}

impl DialDevice {
    pub fn new(long_press_timeout: Duration) -> Result<DialDevice> {
        let (events_tx, events_rx) = mpsc::channel();
        let (haptics_msg_tx, haptics_msg_rx) = mpsc::channel();

        // TODO: interleave control events with regular events
        // (once we figure out what control events actually do...)

        std::thread::spawn({
            let haptics_msg_tx = haptics_msg_tx.clone();
            let mut worker = events::EventsWorker::new(
                events::DialInputKind::MultiAxis,
                events_tx,
                haptics_msg_tx,
            );
            move || {
                worker.run().unwrap();
                eprintln!("the events worker died!");
            }
        });

        std::thread::spawn({
            let mut worker = DialHapticsWorker::new(haptics_msg_rx)?;
            move || {
                if let Err(err) = worker.run() {
                    eprintln!("Unexpected haptics worker error! {}", err);
                }
                eprintln!("the haptics worker died!");
                // there's no coming back from this.
                std::process::exit(0);
            }
        });

        Ok(DialDevice {
            long_press_timeout,
            events: events_rx,
            haptics: DialHaptics::new(haptics_msg_tx)?,

            possible_long_press: false,
        })
    }

    /// Blocks until a new dial event comes occurs.
    // TODO?: rewrite code using async/await?
    // TODO?: "cheat" by exposing an async interface to the current next_event impl
    pub fn next_event(&mut self) -> Result<DialEvent> {
        let evt = if self.possible_long_press {
            self.events.recv_timeout(self.long_press_timeout)
        } else {
            self.events
                .recv()
                .map_err(|_| mpsc::RecvTimeoutError::Disconnected)
        };

        let event = match evt {
            Ok(events::RawInputEvent::Event(event)) => {
                let event =
                    DialEvent::from_raw_evt(event).ok_or(Error::UnexpectedEvt(event))?;

                match event.kind {
                    DialEventKind::ButtonPress => self.possible_long_press = true,
                    DialEventKind::ButtonRelease => self.possible_long_press = false,
                    _ => {}
                }

                event
            }
            Ok(events::RawInputEvent::Connect) => {
                DialEvent {
                    time: Duration::from_secs(0), // this could be improved...
                    kind: DialEventKind::Connect,
                }
            }
            Ok(events::RawInputEvent::Disconnect) => {
                DialEvent {
                    time: Duration::from_secs(0), // this could be improved...
                    kind: DialEventKind::Disconnect,
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                self.possible_long_press = false;
                DialEvent {
                    time: Duration::from_secs(0), // this could be improved...
                    kind: DialEventKind::ButtonLongPress,
                }
            }
            Err(_e) => panic!("Could not recv event"),
        };

        Ok(event)
    }

    pub fn haptics(&self) -> &DialHaptics {
        &self.haptics
    }
}

impl DialEvent {
    pub(crate) fn from_raw_evt(evt: evdev::InputEvent) -> Option<DialEvent> {
        use evdev::*;

        let time = evt
            .timestamp()
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .unwrap_or_default();

        let evt_kind = match evt.destructure() {
            EventSummary::Synchronization(..) | EventSummary::Misc(..) => DialEventKind::Ignored,
            EventSummary::Key(_, KeyCode::BTN_0, 0) => DialEventKind::ButtonRelease,
            EventSummary::Key(_, KeyCode::BTN_0, 1) => DialEventKind::ButtonPress,
            EventSummary::RelativeAxis(_, RelativeAxisCode::REL_DIAL, value) => {
                DialEventKind::Dial(value)
            }
            _ => return None,
        };

        Some(DialEvent {
            time,
            kind: evt_kind,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use evdev::{EventType, InputEvent, KeyCode, RelativeAxisCode};

    fn make_key(code: KeyCode, value: i32) -> InputEvent {
        InputEvent::new(EventType::KEY.0, code.0, value)
    }

    fn make_rel(code: RelativeAxisCode, value: i32) -> InputEvent {
        InputEvent::new(EventType::RELATIVE.0, code.0, value)
    }

    #[test]
    fn btn0_press_is_button_press() {
        let evt = DialEvent::from_raw_evt(make_key(KeyCode::BTN_0, 1)).unwrap();
        assert!(matches!(evt.kind, DialEventKind::ButtonPress));
    }

    #[test]
    fn btn0_release_is_button_release() {
        let evt = DialEvent::from_raw_evt(make_key(KeyCode::BTN_0, 0)).unwrap();
        assert!(matches!(evt.kind, DialEventKind::ButtonRelease));
    }

    #[test]
    fn rel_dial_positive_is_dial() {
        let evt = DialEvent::from_raw_evt(make_rel(RelativeAxisCode::REL_DIAL, 3)).unwrap();
        assert!(matches!(evt.kind, DialEventKind::Dial(3)));
    }

    #[test]
    fn rel_dial_negative_is_dial() {
        let evt = DialEvent::from_raw_evt(make_rel(RelativeAxisCode::REL_DIAL, -2)).unwrap();
        assert!(matches!(evt.kind, DialEventKind::Dial(-2)));
    }

    #[test]
    fn synchronization_event_is_ignored() {
        // EV_SYN = 0, SYN_REPORT = 0
        let evt = DialEvent::from_raw_evt(InputEvent::new(0, 0, 0)).unwrap();
        assert!(matches!(evt.kind, DialEventKind::Ignored));
    }

    #[test]
    fn misc_event_is_ignored() {
        use evdev::MiscCode;
        let evt =
            DialEvent::from_raw_evt(InputEvent::new(EventType::MISC.0, MiscCode::MSC_SCAN.0, 0))
                .unwrap();
        assert!(matches!(evt.kind, DialEventKind::Ignored));
    }

    #[test]
    fn unknown_key_returns_none() {
        // KEY_A is not BTN_0
        assert!(DialEvent::from_raw_evt(make_key(KeyCode::KEY_A, 1)).is_none());
    }

    #[test]
    fn non_dial_rel_axis_returns_none() {
        // REL_X is not REL_DIAL
        assert!(DialEvent::from_raw_evt(make_rel(RelativeAxisCode::REL_X, 1)).is_none());
    }
}
