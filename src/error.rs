use std::fmt;
use std::io;

use evdev::InputEvent;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    ConfigFile(String),
    OpenDevInputDir(io::Error),
    OpenEventFile(std::path::PathBuf, io::Error),
    HidError(hidapi::HidError),
    MissingDial,
    MultipleDials,
    UnexpectedEvt(InputEvent),
    Evdev(io::Error),
    Notif(notify_rust::error::Error),
    TermSig,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::ConfigFile(e) => write!(f, "Could not open config file: {}", e),
            Error::OpenDevInputDir(e) => write!(f, "Could not open /dev/input directory: {}", e),
            Error::OpenEventFile(path, e) => write!(f, "Could not open {:?}: {}", path, e),
            Error::HidError(e) => write!(f, "HID API Error: {}", e),
            Error::MissingDial => write!(f, "Could not find the Surface Dial"),
            Error::MultipleDials => write!(f, "Found multiple dials"),
            Error::UnexpectedEvt(evt) => write!(f, "Unexpected event: {:?}", evt),
            Error::Evdev(e) => write!(f, "Evdev error: {}", e),
            Error::Notif(e) => write!(f, "Notification error: {}", e),
            Error::TermSig => write!(f, "Received termination signal (either SIGTERM or SIGINT)"),
        }
    }
}

impl std::error::Error for Error {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_config_file() {
        let err = Error::ConfigFile("could not open config directory".into());
        assert_eq!(
            format!("{}", err),
            "Could not open config file: could not open config directory"
        );
    }

    #[test]
    fn display_open_dev_input_dir() {
        let err = Error::OpenDevInputDir(io::Error::from_raw_os_error(2));
        assert!(format!("{}", err).starts_with("Could not open /dev/input directory:"));
    }

    #[test]
    fn display_open_event_file() {
        let path = std::path::PathBuf::from("/dev/input/event0");
        let err = Error::OpenEventFile(path, io::Error::from_raw_os_error(13));
        assert!(format!("{}", err).starts_with("Could not open"));
        assert!(format!("{}", err).contains("event0"));
    }

    #[test]
    fn display_missing_dial() {
        assert_eq!(
            format!("{}", Error::MissingDial),
            "Could not find the Surface Dial"
        );
    }

    #[test]
    fn display_multiple_dials() {
        assert_eq!(
            format!("{}", Error::MultipleDials),
            "Found multiple dials"
        );
    }

    #[test]
    fn display_evdev() {
        let err = Error::Evdev(io::Error::from_raw_os_error(5));
        assert!(format!("{}", err).starts_with("Evdev error:"));
    }

    #[test]
    fn display_term_sig() {
        assert_eq!(
            format!("{}", Error::TermSig),
            "Received termination signal (either SIGTERM or SIGINT)"
        );
    }

    #[test]
    fn error_implements_std_error() {
        let err: Box<dyn std::error::Error> = Box::new(Error::MissingDial);
        assert!(format!("{}", err).contains("Surface Dial"));
    }
}
