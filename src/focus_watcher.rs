use std::collections::HashMap;
use std::path::PathBuf;
use std::thread;

use serde::Deserialize;
use zbus::zvariant::OwnedValue;

use crate::controller::ModeSwitcher;

// ---------------------------------------------------------------------------
// Profile config (deserialized from ~/.config/surface-dial/profiles.toml)
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct Profile {
    #[allow(dead_code)]
    name: String,
    /// Substring matched (case-insensitive) against the focused window's app-id.
    /// Omit to mark this entry as the default/fallback profile.
    match_app_id: Option<String>,
    /// Name of the ControlMode to activate (e.g. "DbusMode", "Scroll").
    mode: String,
}

#[derive(Deserialize)]
struct ProfilesConfig {
    profile: Vec<Profile>,
}

// ---------------------------------------------------------------------------
// GNOME Shell Introspect proxy  (polls GetWindows every second)
// ---------------------------------------------------------------------------

#[zbus::proxy(
    interface = "org.gnome.Shell.Introspect",
    default_service = "org.gnome.Shell",
    default_path = "/org/gnome/Shell/Introspect",
    gen_blocking = false
)]
trait ShellIntrospect {
    fn get_windows(
        &self,
    ) -> zbus::Result<HashMap<u64, HashMap<String, OwnedValue>>>;
}

// ---------------------------------------------------------------------------
// FocusWatcher
// ---------------------------------------------------------------------------

pub struct FocusWatcher;

impl FocusWatcher {
    /// Start the focus-watcher in a background thread.
    ///
    /// If `~/.config/surface-dial/profiles.toml` does not exist the function
    /// returns immediately and no background thread is spawned — the feature
    /// is entirely opt-in.
    pub fn start(switcher: ModeSwitcher) {
        let config_path = Self::config_path();

        if !config_path.exists() {
            eprintln!(
                "FocusWatcher: {:?} not found — focus-based mode switching disabled",
                config_path
            );
            return;
        }

        let config_str = match std::fs::read_to_string(&config_path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("FocusWatcher: failed to read {:?}: {}", config_path, e);
                return;
            }
        };

        let config: ProfilesConfig = match toml::from_str(&config_str) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("FocusWatcher: failed to parse profiles config: {}", e);
                return;
            }
        };

        thread::spawn(move || {
            let rt = match tokio::runtime::Runtime::new() {
                Ok(rt) => rt,
                Err(e) => {
                    eprintln!("FocusWatcher: failed to create tokio runtime: {}", e);
                    return;
                }
            };

            rt.block_on(async move {
                let conn = match zbus::Connection::session().await {
                    Ok(c) => c,
                    Err(e) => {
                        eprintln!(
                            "FocusWatcher: failed to connect to session bus: {}",
                            e
                        );
                        return;
                    }
                };

                let proxy = match ShellIntrospectProxy::new(&conn).await {
                    Ok(p) => p,
                    Err(e) => {
                        eprintln!(
                            "FocusWatcher: failed to create GNOME Shell proxy \
                             (is GNOME Shell running?): {}",
                            e
                        );
                        return;
                    }
                };

                let mut last_app_id = String::new();
                let mut interval =
                    tokio::time::interval(std::time::Duration::from_secs(1));

                loop {
                    interval.tick().await;

                    let windows = match proxy.get_windows().await {
                        Ok(w) => w,
                        Err(e) => {
                            eprintln!("FocusWatcher: get_windows failed: {}", e);
                            continue;
                        }
                    };

                    let focused_app_id = match Self::find_focused_app_id(&windows) {
                        Some(id) => id,
                        None => continue,
                    };

                    if focused_app_id == last_app_id {
                        continue;
                    }
                    last_app_id = focused_app_id.clone();

                    let mode = Self::match_profile(&config.profile, &focused_app_id);
                    eprintln!(
                        "FocusWatcher: focused '{}' → switching to mode '{}'",
                        focused_app_id, mode
                    );
                    switcher.switch_to(&mode);
                }
            });
        });
    }

    /// Walk the window map and return the `app-id` of the focused window.
    fn find_focused_app_id(
        windows: &HashMap<u64, HashMap<String, OwnedValue>>,
    ) -> Option<String> {
        use zbus::zvariant::Value;

        for props in windows.values() {
            let focused = props
                .get("has-focus")
                .and_then(|v| {
                    if let Value::Bool(b) = &**v {
                        Some(*b)
                    } else {
                        None
                    }
                })
                .unwrap_or(false);

            if focused {
                return props.get("app-id").and_then(|v| {
                    if let Value::Str(s) = &**v {
                        Some(s.as_str().to_string())
                    } else {
                        None
                    }
                });
            }
        }

        None
    }

    /// Return the mode name for `app_id` by scanning profiles in order.
    /// The first profile whose `match_app_id` is a case-insensitive substring
    /// of `app_id` wins.  If no rule matches, the first profile without a
    /// `match_app_id` (the "default" entry) is used.  Falls back to "Scroll"
    /// if no default profile is present.
    fn match_profile(profiles: &[Profile], app_id: &str) -> String {
        let app_id_lower = app_id.to_lowercase();

        for profile in profiles {
            if let Some(ref match_id) = profile.match_app_id {
                if app_id_lower.contains(&match_id.to_lowercase()) {
                    return profile.mode.clone();
                }
            }
        }

        // Fall back to the first profile without a match_app_id.
        for profile in profiles {
            if profile.match_app_id.is_none() {
                return profile.mode.clone();
            }
        }

        "Scroll".into()
    }

    fn config_path() -> PathBuf {
        directories::BaseDirs::new()
            .map(|d| d.config_dir().join("surface-dial").join("profiles.toml"))
            .unwrap_or_else(|| {
                let home = std::env::var("HOME").unwrap_or_default();
                PathBuf::from(format!(
                    "{}/.config/surface-dial/profiles.toml",
                    home
                ))
            })
    }
}
