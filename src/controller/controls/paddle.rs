use std::cmp::Ordering;
use std::sync::mpsc;
use std::thread::JoinHandle;
use std::time::Duration;

use crate::controller::{ControlMode, ControlModeMeta};
use crate::dial_device::DialHaptics;
use crate::error::Result;
use crate::fake_input;

use evdev::KeyCode;

// everything is done in a worker, as we need use `recv_timeout` as a (very)
// poor man's `select!`.

/// Apply a rotation delta to the current velocity, resetting on direction change.
/// Returns `(new_velocity, new_last_delta)`.
pub(crate) fn apply_delta(velocity: i32, last_delta: i32, delta: i32, cap: i32) -> (i32, i32) {
    let v = if (delta < 0) != (last_delta < 0) {
        0
    } else {
        velocity
    };
    let v = (v + delta).clamp(-cap, cap);
    (v, delta)
}

/// Apply one falloff step to the velocity (called on timeout).
/// Returns the new velocity after decaying toward zero.
pub(crate) fn apply_falloff(velocity: i32, falloff_divisor: i32, cap: i32) -> i32 {
    let falloff = velocity.abs() / falloff_divisor + 1;
    let v = match velocity.cmp(&0) {
        Ordering::Less => velocity + falloff,
        Ordering::Greater => velocity - falloff,
        Ordering::Equal => 0,
    };
    v.clamp(-cap, cap)
}

enum Msg {
    Kill,
    ButtonDown,
    ButtonUp,
    Delta(i32),
    Enabled(bool),
}

struct Worker {
    msg: mpsc::Receiver<Msg>,

    timeout: u64,
    falloff: i32,
    cap: i32,
    deadzone: i32,

    enabled: bool,
    last_delta: i32,
    velocity: i32,
}

impl Worker {
    pub fn new(msg: mpsc::Receiver<Msg>) -> Worker {
        Worker {
            msg,

            // tweak these for "feel"
            timeout: 5,
            falloff: 10,
            cap: 250,
            deadzone: 10,

            enabled: false,
            last_delta: 0,
            velocity: 0,
        }
    }

    pub fn run(&mut self) {
        loop {
            let msg = if self.enabled {
                self.msg.recv_timeout(Duration::from_millis(self.timeout))
            } else {
                self.msg
                    .recv()
                    .map_err(|_| mpsc::RecvTimeoutError::Disconnected)
            };

            match msg {
                Ok(Msg::Kill) => return,
                Ok(Msg::Enabled(enabled)) => {
                    self.enabled = enabled;
                    if !enabled {
                        fake_input::key_release(&[
                            KeyCode::KEY_SPACE,
                            KeyCode::KEY_LEFT,
                            KeyCode::KEY_RIGHT,
                        ])
                        .unwrap()
                    }
                }
                Ok(Msg::ButtonDown) => fake_input::key_press(&[KeyCode::KEY_SPACE]).unwrap(),
                Ok(Msg::ButtonUp) => fake_input::key_release(&[KeyCode::KEY_SPACE]).unwrap(),
                Ok(Msg::Delta(delta)) => {
                    let (v, ld) =
                        apply_delta(self.velocity, self.last_delta, delta, self.cap);
                    self.velocity = v;
                    self.last_delta = ld;
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    self.velocity = apply_falloff(self.velocity, self.falloff, self.cap);
                }
                Err(other) => panic!("{}", other),
            }

            // clamp velocity within the cap bounds
            if self.velocity > self.cap {
                self.velocity = self.cap;
            } else if self.velocity < -self.cap {
                self.velocity = -self.cap;
            }

            if self.velocity.abs() < self.deadzone {
                fake_input::key_release(&[KeyCode::KEY_LEFT, KeyCode::KEY_RIGHT]).unwrap();
                continue;
            }

            match self.velocity.cmp(&0) {
                Ordering::Equal => {}
                Ordering::Less => fake_input::key_press(&[KeyCode::KEY_LEFT]).unwrap(),
                Ordering::Greater => fake_input::key_press(&[KeyCode::KEY_RIGHT]).unwrap(),
            }

            // eprintln!("{:?}", self.velocity);
        }
    }
}

/// A bit of a misnomer, since it's only left-right.
pub struct Paddle {
    _worker: JoinHandle<()>,
    msg: mpsc::Sender<Msg>,
}

impl Drop for Paddle {
    fn drop(&mut self) {
        let _ = self.msg.send(Msg::Kill);
    }
}

impl Paddle {
    pub fn new() -> Paddle {
        let (msg_tx, msg_rx) = mpsc::channel();

        let worker = std::thread::spawn(move || Worker::new(msg_rx).run());

        Paddle {
            _worker: worker,
            msg: msg_tx,
        }
    }
}

impl ControlMode for Paddle {
    fn meta(&self) -> ControlModeMeta {
        ControlModeMeta {
            name: "Paddle".into(),
            icon: "input-gaming".into(),
            haptics: false,
            steps: 3600,
        }
    }

    fn on_start(&mut self, _haptics: &DialHaptics) -> Result<()> {
        let _ = self.msg.send(Msg::Enabled(true));
        Ok(())
    }

    fn on_end(&mut self, _haptics: &DialHaptics) -> Result<()> {
        let _ = self.msg.send(Msg::Enabled(false));
        Ok(())
    }

    fn on_btn_press(&mut self, _: &DialHaptics) -> Result<()> {
        let _ = self.msg.send(Msg::ButtonDown);
        Ok(())
    }

    fn on_btn_release(&mut self, _: &DialHaptics) -> Result<()> {
        let _ = self.msg.send(Msg::ButtonUp);
        Ok(())
    }

    fn on_dial(&mut self, _: &DialHaptics, delta: i32) -> Result<()> {
        let _ = self.msg.send(Msg::Delta(delta));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CAP: i32 = 250;
    const FALLOFF_DIV: i32 = 10;

    // apply_delta tests

    #[test]
    fn delta_accumulates_same_direction() {
        let (v, ld) = apply_delta(0, 0, 5, CAP);
        assert_eq!(v, 5);
        assert_eq!(ld, 5);

        let (v, ld) = apply_delta(v, ld, 3, CAP);
        assert_eq!(v, 8);
        assert_eq!(ld, 3);
    }

    #[test]
    fn delta_resets_on_direction_change() {
        // establish positive velocity
        let (v, ld) = apply_delta(20, 5, 5, CAP);
        assert_eq!(v, 25);

        // reverse direction
        let (v, _ld) = apply_delta(v, ld, -1, CAP);
        // velocity reset to 0 before applying -1
        assert_eq!(v, -1);
    }

    #[test]
    fn delta_clamped_to_cap() {
        let (v, _) = apply_delta(245, 1, 10, CAP);
        assert_eq!(v, CAP);
    }

    #[test]
    fn delta_clamped_to_negative_cap() {
        let (v, _) = apply_delta(-245, -1, -10, CAP);
        assert_eq!(v, -CAP);
    }

    #[test]
    fn first_delta_from_zero_does_not_reset() {
        // last_delta=0, delta positive: (0<0)==(0<0) so no reset
        let (v, _) = apply_delta(0, 0, 7, CAP);
        assert_eq!(v, 7);
    }

    // apply_falloff tests

    #[test]
    fn falloff_reduces_positive_velocity() {
        let v = apply_falloff(50, FALLOFF_DIV, CAP);
        // falloff = 50/10 + 1 = 6; 50 - 6 = 44
        assert_eq!(v, 44);
    }

    #[test]
    fn falloff_reduces_negative_velocity() {
        let v = apply_falloff(-50, FALLOFF_DIV, CAP);
        // falloff = 50/10 + 1 = 6; -50 + 6 = -44
        assert_eq!(v, -44);
    }

    #[test]
    fn falloff_on_zero_stays_zero() {
        assert_eq!(apply_falloff(0, FALLOFF_DIV, CAP), 0);
    }

    #[test]
    fn falloff_small_velocity_goes_to_zero() {
        // velocity=1: falloff = 1/10 + 1 = 1; 1 - 1 = 0
        assert_eq!(apply_falloff(1, FALLOFF_DIV, CAP), 0);
        assert_eq!(apply_falloff(-1, FALLOFF_DIV, CAP), 0);
    }

    #[test]
    fn falloff_result_clamped() {
        // shouldn't happen in practice but verify clamp is applied
        assert!(apply_falloff(CAP, FALLOFF_DIV, CAP) <= CAP);
    }

    // ControlModeMeta tests

    #[test]
    fn paddle_meta() {
        let meta = Paddle::new().meta();
        assert_eq!(meta.name, "Paddle");
        assert!(!meta.haptics);
        assert_eq!(meta.steps, 3600);
    }
}
