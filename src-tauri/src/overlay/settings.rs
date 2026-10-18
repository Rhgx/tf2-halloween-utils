//! The overlay's settings (the radar and the other widgets), saved as JSON next to the app's other files. The
//! RCON port and password are created on first run and must match the TF2 launch options the app shows.

use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hasher};
use std::io;
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::widgets::countdown::{CountdownFormat, COUNTDOWN_H};
use super::widgets::radar::nav::Units;
use super::widgets::radar::panel::PANEL_H;
use crate::store;

const DEFAULT_POS_X: f32 = 0.42;
const DEFAULT_POS_Y: f32 = 0.68;
/// Under the radar's default spot, as `countdown_by_radar` places it (0.68 plus 86 of 1080 units).
const DEFAULT_COUNTDOWN: Widget = Widget {
    enabled: true,
    pos_x: DEFAULT_POS_X,
    pos_y: 0.76,
    opacity: DEFAULT_OPACITY,
    scale: DEFAULT_SCALE,
};
const DEFAULT_SCALE: f32 = 1.0;
const DEFAULT_OPACITY: f32 = 0.78;
// Keys every layout names the same way, that TF2 leaves unbound (tf/cfg/config_default.cfg), and that a 60% keyboard
// reaches with Fn (Fn+3, Fn+4).
pub const DEFAULT_TOGGLE_KEY: u16 = 0x72; // F3
pub const DEFAULT_EDIT_KEY: u16 = 0x73; // F4

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct OverlaySettings {
    /// The overlay as a whole; each widget also has its own switch.
    pub enabled: bool,
    /// The radar widget. Its RCON and demo threads run only while it and the overlay are on.
    pub radar_enabled: bool,
    pub countdown: Widget,
    /// How the countdown writes the time left.
    pub countdown_format: CountdownFormat,
    pub port: u16,
    pub password: String,
    /// Longest wait, in seconds, before a new gargoyle reaches the demo file (filler commands keep it short).
    pub demo_delay_s: f32,
    /// The radar panel's opacity, scale and centre (as fractions of the TF2 client area).
    pub opacity: f32,
    pub scale: f32,
    pub pos_x: f32,
    pub pos_y: f32,
    /// Follow any gargoyle, not just yours (for testing with `ent_create tf_halloween_gift_pickup`).
    pub any_gargoyle: bool,
    /// Show how long until the gargoyle despawns on the radar panel.
    pub despawn_timer: bool,
    /// The unit the radar writes distances in.
    pub radar_units: Units,
    /// RCON has connected with this port and password, so the launch options are in place.
    pub setup_done: bool,
    /// Windows virtual-key codes of the in-game hotkeys: turn the overlay on and off, start and save edit mode.
    pub toggle_key: u16,
    pub edit_key: u16,
}

/// A widget's switch, centre (as fractions of the TF2 client area) and look. The radar's predate this, so in the
/// file they stay top-level fields; `radar()` gathers them in this shape.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Widget {
    pub enabled: bool,
    pub pos_x: f32,
    pub pos_y: f32,
    pub opacity: f32,
    pub scale: f32,
}

impl Default for Widget {
    fn default() -> Self {
        DEFAULT_COUNTDOWN
    }
}

impl Default for OverlaySettings {
    fn default() -> Self {
        OverlaySettings {
            enabled: false,
            radar_enabled: true,
            countdown: DEFAULT_COUNTDOWN,
            countdown_format: CountdownFormat::Adaptive,
            port: 21770,
            password: String::new(),
            demo_delay_s: 5.0,
            opacity: DEFAULT_OPACITY,
            scale: DEFAULT_SCALE,
            pos_x: DEFAULT_POS_X,
            pos_y: DEFAULT_POS_Y,
            any_gargoyle: false,
            despawn_timer: true,
            radar_units: Units::Metres,
            setup_done: false,
            toggle_key: DEFAULT_TOGGLE_KEY,
            edit_key: DEFAULT_EDIT_KEY,
        }
    }
}

// A missing file means the defaults, and missing fields take their defaults (serde default). A file that cannot
// be read is replaced by the defaults with the reason printed. A password generated here is saved straight away
// so it stays the same on the next launch.
pub fn load(file: &Path) -> OverlaySettings {
    let mut settings = match store::read_optional(file) {
        Ok(None) => OverlaySettings::default(),
        Ok(Some(text)) => match serde_json::from_str::<OverlaySettings>(&text) {
            Ok(mut settings) => {
                // a file from before the countdown: it goes under the radar where that is now, not where the
                // radar starts out, or the two could overlap
                let has_countdown = serde_json::from_str::<serde_json::Value>(&text)
                    .is_ok_and(|v| v.get("countdown").is_some());
                if !has_countdown {
                    settings.countdown = settings.countdown_by_radar();
                }
                settings
            }
            Err(err) => {
                eprintln!(
                    "{} is not valid overlay settings, using defaults: {err}",
                    file.display()
                );
                OverlaySettings::default()
            }
        },
        Err(err) => {
            eprintln!("could not read {}, using defaults: {err}", file.display());
            OverlaySettings::default()
        }
    };
    settings.clamp();
    if settings.password.is_empty() {
        settings.password = random_password();
        if let Err(err) = settings.save(file) {
            eprintln!("could not save {}: {err}", file.display());
        }
    }
    settings
}

impl OverlaySettings {
    pub fn save(&self, file: &Path) -> io::Result<()> {
        store::write_atomic(file, &serde_json::to_string_pretty(self)?)
    }

    pub fn clamp(&mut self) {
        self.demo_delay_s = self.demo_delay_s.clamp(2.0, 20.0);
        self.pos_x = self.pos_x.clamp(0.0, 1.0);
        self.pos_y = self.pos_y.clamp(0.0, 1.0);
        self.countdown.pos_x = self.countdown.pos_x.clamp(0.0, 1.0);
        self.countdown.pos_y = self.countdown.pos_y.clamp(0.0, 1.0);
        self.scale = self.scale.clamp(0.5, 2.5);
        self.opacity = self.opacity.clamp(0.0, 1.0);
        self.countdown.scale = self.countdown.scale.clamp(0.5, 2.5);
        self.countdown.opacity = self.countdown.opacity.clamp(0.0, 1.0);
    }

    /// The radar's switch, position and look, in the shape the other widgets have.
    pub fn radar(&self) -> Widget {
        Widget {
            enabled: self.radar_enabled,
            pos_x: self.pos_x,
            pos_y: self.pos_y,
            opacity: self.opacity,
            scale: self.scale,
        }
    }

    pub fn reset_layout(&mut self) {
        (self.pos_x, self.pos_y, self.scale, self.opacity) =
            (DEFAULT_POS_X, DEFAULT_POS_Y, DEFAULT_SCALE, DEFAULT_OPACITY);
        self.countdown = Widget {
            enabled: self.countdown.enabled,
            ..self.countdown_by_radar()
        };
    }

    /// The countdown just under the radar, or just above it when there is no room below, looking like the radar
    /// (before each widget had its own look, the radar's applied to both).
    fn countdown_by_radar(&self) -> Widget {
        // centre to centre: half of each panel and a small gap, in 1080p units, as fractions of the screen height
        let half_countdown = COUNTDOWN_H / 2.0 * self.scale / 1080.0;
        let step = (PANEL_H / 2.0 + 8.0 + COUNTDOWN_H / 2.0) * self.scale / 1080.0;
        let below = self.pos_y + step;
        Widget {
            enabled: true,
            pos_x: self.pos_x,
            pos_y: if below + half_countdown <= 1.0 {
                below
            } else {
                self.pos_y - step
            },
            opacity: self.opacity,
            scale: self.scale,
        }
    }

    /// Launch options that enable TF2's RCON on the local network, protected by the password, plus -condebug for
    /// match tracking.
    pub fn launch_options(&self) -> String {
        format!(
            "-usercon +ip 0.0.0.0 +hostport {} +net_start +rcon_password {} +sv_rcon_whitelist_address 127.0.0.1 -condebug",
            self.port, self.password
        )
    }
}

/// 24 hex characters from the OS-seeded hasher keys, unguessable: it is what protects RCON on the local network.
fn random_password() -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    (0..2u128)
        .map(|i| {
            let mut h = RandomState::new().build_hasher();
            h.write_u128(nanos ^ i);
            format!("{:016x}", h.finish())
        })
        .collect::<String>()[..24]
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_file_gives_the_defaults() -> io::Result<()> {
        let dir = store::scratch_dir("radar-settings-defaults")?;
        let loaded = load(&dir.join("radar.json"));
        assert_eq!(loaded.password.len(), 24, "a password is generated");
        let expected = OverlaySettings {
            password: loaded.password.clone(),
            ..OverlaySettings::default()
        };
        assert_eq!(loaded, expected);
        std::fs::remove_dir_all(&dir)
    }

    #[test]
    fn password_is_generated_once_and_kept() -> io::Result<()> {
        let dir = store::scratch_dir("radar-settings-password")?;
        let file = dir.join("radar.json");
        let first = load(&file).password;
        assert_eq!(load(&file).password, first);
        std::fs::remove_dir_all(&dir)
    }

    #[test]
    fn clamp_keeps_values_in_range() {
        let mut settings = OverlaySettings {
            demo_delay_s: 0.5,
            pos_x: -1.0,
            pos_y: 3.0,
            scale: 9.0,
            opacity: -0.5,
            ..OverlaySettings::default()
        };
        settings.clamp();
        let got = (
            settings.demo_delay_s,
            settings.pos_x,
            settings.pos_y,
            settings.scale,
            settings.opacity,
        );
        assert_eq!(got, (2.0, 0.0, 1.0, 2.5, 0.0));
    }

    #[test]
    fn a_file_from_before_the_widgets_puts_the_countdown_by_the_radar() -> io::Result<()> {
        let dir = store::scratch_dir("overlay-settings-countdown")?;
        let file = dir.join("radar.json");
        std::fs::write(
            &file,
            r#"{ "enabled": true, "posX": 0.3, "posY": 0.82, "opacity": 0.5, "password": "x" }"#,
        )?;
        let old = load(&file);
        assert!(old.enabled && old.radar_enabled && old.countdown.enabled);
        // under a radar that sits low, clear of it
        // and looks like the radar did, since the radar's look applied to both before
        assert_eq!((old.countdown.pos_x, old.countdown.opacity), (0.3, 0.5));
        assert!((old.countdown.pos_y - (0.82 + 86.0 / 1080.0)).abs() < 1e-5);
        // no room below a radar at the bottom: above it
        std::fs::write(&file, r#"{ "posY": 0.97, "password": "x" }"#)?;
        assert!(load(&file).countdown.pos_y < 0.97);
        // a saved countdown stays where it is
        std::fs::write(
            &file,
            r#"{ "posY": 0.82, "countdown": { "posY": 0.1 }, "password": "x" }"#,
        )?;
        assert_eq!(load(&file).countdown.pos_y, 0.1);
        std::fs::remove_dir_all(&dir)
    }

    #[test]
    fn the_default_countdown_sits_under_the_default_radar() {
        let at = OverlaySettings::default().countdown_by_radar().pos_y;
        assert!((at - DEFAULT_COUNTDOWN.pos_y).abs() < 0.001, "{at}");
    }

    #[test]
    fn launch_options_end_with_condebug() {
        let settings = OverlaySettings {
            password: "secret".to_string(),
            ..OverlaySettings::default()
        };
        let options = settings.launch_options();
        assert!(options.ends_with(" -condebug"), "{options}");
        assert!(options.contains("+rcon_password secret"), "{options}");
    }
}
