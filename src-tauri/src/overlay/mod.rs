//! The in-game overlay: widgets drawn in click-through windows over TF2 (borderless windowed). While it is on, the
//! overlay thread (`run.rs`) follows TF2's window, draws the widgets that are on and handles the hotkeys and edit
//! mode; while the radar widget is on too, the radar's RCON and demo threads run (`widgets/radar`). They share
//! `Live`, which the Overlay tab reads through `Overlay::view`. Each group has its own stop flag and is joined
//! when switched off; the overlay thread destroys its own windows on the way out. Windows only, see main.rs.

mod edit;
mod input;
mod paint;
mod run;
mod settings;
mod widgets;
mod window;

use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::thread::{self, sleep, JoinHandle};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{VK_ESCAPE, VK_LBUTTON};

use input::{key_name, Keys, EDIT_MODE_KEYS, IGNORED_KEYS};
use run::overlay_thread;
use settings::{OverlaySettings, Widget};
use widgets::countdown::CountdownFormat;
use widgets::radar::nav::Units;
use widgets::radar::{demo_worker, nav, rcon::Pose, rcon_worker, seconds_left, TYPICAL_BURST};

const POSE_STALE: Duration = Duration::from_millis(1500);

const EVENTS_KEPT: usize = 8;

#[derive(Clone, Copy, PartialEq, Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Link {
    Off,
    Waiting,
    Unreachable,
    WrongPassword,
    Connected,
}

#[derive(Clone, Copy, PartialEq, Debug, Serialize)]
#[serde(rename_all = "lowercase")]
enum Tone {
    Info,
    Good,
    Warn,
    Bad,
    Gargoyle,
}

#[derive(Clone, Serialize)]
struct Event {
    /// Milliseconds since the UNIX epoch.
    at: u64,
    text: String,
    tone: Tone,
}

/// Everything the overlay's threads share, and what the Overlay tab shows.
struct Live {
    file: PathBuf,
    settings: OverlaySettings,
    /// The `tf` folder of the TF2 that is running (from its window), for the demo files and the fonts.
    tf_dir: Option<PathBuf>,
    tf2_running: bool,
    link: Link,
    pose: Option<(Pose, Instant)>,
    position_hz: Option<u32>,
    /// A recording of a match is being written (none in menus, after disconnecting, or after `stop`).
    in_match: bool,
    gargoyle: Option<[f32; 3]>,
    /// When the gargoyle spawned, if it spawned while the demo was recording.
    gargoyle_spawned: Option<Instant>,
    map: Option<String>,
    demo_file: Option<String>,
    demo_interval_s: Option<f32>,
    demo_grew: Option<Instant>,
    /// Filler bytes per second to send, set by the demo thread.
    filler: f32,
    /// "Move radar" was pressed in the app; edit mode starts when TF2 is next in front.
    edit_requested: bool,
    editing: bool,
    events: VecDeque<Event>,
}

impl Live {
    fn event(&mut self, tone: Tone, text: impl Into<String>) {
        let text = text.into();
        println!("radar: {text}");
        let at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_millis() as u64);
        self.events.push_front(Event { at, text, tone });
        self.events.truncate(EVENTS_KEPT);
    }

    fn set_link(&mut self, link: Link) {
        if self.link == link {
            return;
        }
        self.link = link;
        match link {
            Link::Connected => {
                self.event(Tone::Good, "Connected to TF2 over RCON.");
                if !self.settings.setup_done {
                    self.settings.setup_done = true;
                    self.save();
                }
            }
            Link::WrongPassword => self.event(
                Tone::Bad,
                "TF2 rejected the RCON password: check the launch options.",
            ),
            Link::Unreachable => self.event(
                Tone::Warn,
                "Cannot reach TF2's RCON: add the launch options, then restart TF2.",
            ),
            Link::Waiting | Link::Off => {}
        }
    }

    fn save(&mut self) {
        if let Err(err) = self.settings.save(&self.file) {
            self.event(Tone::Bad, format!("Could not save radar settings: {err}"));
        }
    }
}

fn lock(live: &Mutex<Live>) -> MutexGuard<'_, Live> {
    live.lock().unwrap_or_else(PoisonError::into_inner)
}

/// A thread body: the shared state and the stop flag of its group.
type Worker = fn(Arc<Mutex<Live>>, Arc<AtomicBool>);

struct Running {
    stop: Arc<AtomicBool>,
    threads: Vec<JoinHandle<()>>,
}

/// The overlay as the app holds it (managed Tauri state).
pub struct Overlay {
    live: Arc<Mutex<Live>>,
    /// The overlay thread, while the overlay is on.
    overlay: Mutex<Option<Running>>,
    /// The RCON and demo threads, while the overlay and its radar widget are on.
    tracking: Mutex<Option<Running>>,
}

impl Running {
    fn start(live: &Arc<Mutex<Live>>, workers: &[(&str, Worker)]) -> Running {
        let stop = Arc::new(AtomicBool::new(false));
        let threads = workers
            .iter()
            .map(|(name, body)| {
                let (live, stop, body) = (live.clone(), stop.clone(), *body);
                thread::Builder::new()
                    .name(name.to_string())
                    .spawn(move || body(live, stop))
                    .expect("could not start an overlay thread")
            })
            .collect();
        Running { stop, threads }
    }

    fn stop(self) {
        self.stop.store(true, Ordering::Relaxed);
        for thread in self.threads {
            let _ = thread.join();
        }
    }
}

/// What the Overlay tab gets from `get_overlay`.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OverlayView {
    settings: SettingsView,
    launch_options: String,
    setup_done: bool,
    status: StatusView,
    events: Vec<Event>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SettingsView {
    enabled: bool,
    radar: Widget,
    countdown: Widget,
    countdown_format: CountdownFormat,
    port: u16,
    demo_delay_s: f32,
    any_gargoyle: bool,
    despawn_timer: bool,
    radar_units: Units,
    toggle_key: KeyView,
    edit_key: KeyView,
}

#[derive(Serialize)]
struct KeyView {
    code: u16,
    name: String,
}

impl KeyView {
    fn new(code: u16) -> KeyView {
        KeyView {
            code,
            name: key_name(code),
        }
    }
}

/// Which hotkey `capture_key` changes.
#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum KeySlot {
    Toggle,
    Edit,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct StatusView {
    tf2_running: bool,
    link: Link,
    position_hz: Option<u32>,
    demo: Option<DemoView>,
    gargoyle: Option<GargoyleView>,
    reading: Option<nav::Reading>,
    editing: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DemoView {
    file: String,
    interval_s: Option<f32>,
    filler_kbps: f32,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GargoyleView {
    map: Option<String>,
    x: f32,
    y: f32,
    z: f32,
    /// Seconds until it despawns, if known.
    left_s: Option<u32>,
}

/// The settings the Overlay tab can change. Widget positions change in edit mode only. Missing fields stay as they are.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsPatch {
    radar: Option<WidgetPatch>,
    countdown: Option<WidgetPatch>,
    countdown_format: Option<CountdownFormat>,
    port: Option<u16>,
    demo_delay_s: Option<f32>,
    any_gargoyle: Option<bool>,
    despawn_timer: Option<bool>,
    radar_units: Option<Units>,
}

/// A widget's switch and look; its position changes in edit mode only.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WidgetPatch {
    enabled: Option<bool>,
    opacity: Option<f32>,
    scale: Option<f32>,
}

impl Overlay {
    pub fn load(file: &Path) -> Overlay {
        let settings = settings::load(file);
        let live = Live {
            file: file.to_path_buf(),
            filler: TYPICAL_BURST / settings.demo_delay_s,
            settings,
            tf_dir: None,
            tf2_running: false,
            link: Link::Off,
            pose: None,
            position_hz: None,
            in_match: false,
            gargoyle: None,
            gargoyle_spawned: None,
            map: None,
            demo_file: None,
            demo_interval_s: None,
            demo_grew: None,
            edit_requested: false,
            editing: false,
            events: VecDeque::new(),
        };
        Overlay {
            live: Arc::new(Mutex::new(live)),
            overlay: Mutex::new(None),
            tracking: Mutex::new(None),
        }
    }

    /// Turns the overlay as a whole on or off and remembers the choice. Stopping waits for the threads (about two
    /// seconds at most, while an RCON call times out), so call it off the main thread.
    pub fn set_enabled(&self, on: bool) {
        {
            let mut live = lock(&self.live);
            if live.settings.enabled != on {
                live.settings.enabled = on;
                live.save();
            }
        }
        self.apply();
    }

    /// Starts or stops the thread groups to match the switches: the overlay thread while the overlay is on, the
    /// RCON and demo threads while the radar widget is on too. May wait for threads to stop, like `set_enabled`.
    pub fn apply(&self) {
        let (overlay_on, radar_on) = {
            let s = &lock(&self.live).settings;
            (s.enabled, s.enabled && s.radar_enabled)
        };
        let mut overlay = self.overlay.lock().unwrap_or_else(PoisonError::into_inner);
        let mut tracking = self.tracking.lock().unwrap_or_else(PoisonError::into_inner);
        // the radar's threads stop first and start last, so they never run without the overlay thread
        if !radar_on {
            if let Some(run) = tracking.take() {
                run.stop();
                let mut live = lock(&self.live);
                live.link = Link::Off;
                live.pose = None;
                live.position_hz = None;
                live.in_match = false;
                live.gargoyle = None;
                live.gargoyle_spawned = None;
                live.demo_file = None;
                live.demo_interval_s = None;
                live.event(Tone::Info, "Radar off.");
            }
        }
        match (overlay_on, overlay.is_some()) {
            (true, false) => {
                lock(&self.live).event(Tone::Info, "Overlay on.");
                *overlay = Some(Running::start(&self.live, &[("overlay", overlay_thread)]));
            }
            (false, true) => {
                if let Some(run) = overlay.take() {
                    run.stop();
                }
                let mut live = lock(&self.live);
                live.tf2_running = false;
                live.edit_requested = false;
                live.editing = false;
                live.event(Tone::Info, "Overlay off.");
            }
            _ => {}
        }
        if radar_on && tracking.is_none() {
            {
                let mut live = lock(&self.live);
                live.link = Link::Waiting;
                live.event(Tone::Info, "Radar on.");
            }
            *tracking = Some(Running::start(
                &self.live,
                &[("radar-rcon", rcon_worker), ("radar-demo", demo_worker)],
            ));
        }
    }

    /// Stops everything (the app is quitting); the switches stay as they are for the next start.
    pub fn stop(&self) {
        for group in [&self.tracking, &self.overlay] {
            if let Some(run) = group.lock().unwrap_or_else(PoisonError::into_inner).take() {
                run.stop();
            }
        }
    }

    /// Changes settings; a widget switch can start or stop the radar's threads, so call it off the main thread.
    pub fn update(&self, patch: SettingsPatch) {
        self.change(patch);
        self.apply();
    }

    fn change(&self, patch: SettingsPatch) {
        let mut live = lock(&self.live);
        let s = &mut live.settings;
        if let Some(p) = patch.radar {
            s.radar_enabled = p.enabled.unwrap_or(s.radar_enabled);
            s.opacity = p.opacity.unwrap_or(s.opacity);
            s.scale = p.scale.unwrap_or(s.scale);
        }
        if let Some(p) = patch.countdown {
            let c = &mut s.countdown;
            c.enabled = p.enabled.unwrap_or(c.enabled);
            c.opacity = p.opacity.unwrap_or(c.opacity);
            c.scale = p.scale.unwrap_or(c.scale);
        }
        s.countdown_format = patch.countdown_format.unwrap_or(s.countdown_format);
        if let Some(port) = patch.port.filter(|port| *port != s.port) {
            // a new port needs new launch options, so setup starts over (the RCON thread reconnects by itself)
            s.port = port;
            s.setup_done = false;
        }
        s.demo_delay_s = patch.demo_delay_s.unwrap_or(s.demo_delay_s);
        s.any_gargoyle = patch.any_gargoyle.unwrap_or(s.any_gargoyle);
        s.despawn_timer = patch.despawn_timer.unwrap_or(s.despawn_timer);
        s.radar_units = patch.radar_units.unwrap_or(s.radar_units);
        s.clamp();
        live.save();
    }

    pub fn request_edit(&self) {
        lock(&self.live).edit_requested = true;
    }

    /// Waits for the next key or mouse button press (up to 10 s) and binds it. Esc cancels and keeps the old key.
    /// Blocks, so call it off the main thread.
    pub fn capture_key(&self, slot: KeySlot) -> Result<(), String> {
        // let go of whatever pressed the Change button first (a click, Enter or Space)
        let released = Instant::now();
        while (3..=0xFEu16).chain([VK_LBUTTON]).any(Keys::down)
            && released.elapsed() < Duration::from_secs(2)
        {
            sleep(Duration::from_millis(10));
        }
        let asked = Instant::now();
        let vk = loop {
            if asked.elapsed() > Duration::from_secs(10) {
                return Err("No key was pressed within 10 seconds.".to_string());
            }
            if let Some(vk) = (3..=0xFEu16).find(|vk| !IGNORED_KEYS.contains(vk) && Keys::down(*vk))
            {
                break vk;
            }
            sleep(Duration::from_millis(10));
        };
        if vk == VK_ESCAPE {
            return Ok(());
        }
        if EDIT_MODE_KEYS.contains(&vk) {
            return Err(format!(
                "{} is used in edit mode. Pick another key.",
                key_name(vk)
            ));
        }
        let mut live = lock(&self.live);
        let other = match slot {
            KeySlot::Toggle => live.settings.edit_key,
            KeySlot::Edit => live.settings.toggle_key,
        };
        if vk == other {
            return Err(format!("{} is already the other hotkey.", key_name(vk)));
        }
        match slot {
            KeySlot::Toggle => live.settings.toggle_key = vk,
            KeySlot::Edit => live.settings.edit_key = vk,
        }
        live.save();
        Ok(())
    }

    /// Puts one hotkey back to its default. If the other hotkey sits on that default, it swaps to this one's key
    /// so the two never collide.
    pub fn reset_key(&self, slot: KeySlot) {
        let mut live = lock(&self.live);
        let s = &mut live.settings;
        let (this, other, default) = match slot {
            KeySlot::Toggle => (
                &mut s.toggle_key,
                &mut s.edit_key,
                settings::DEFAULT_TOGGLE_KEY,
            ),
            KeySlot::Edit => (
                &mut s.edit_key,
                &mut s.toggle_key,
                settings::DEFAULT_EDIT_KEY,
            ),
        };
        if *other == default {
            *other = *this;
        }
        *this = default;
        live.save();
    }

    pub fn reset_layout(&self) {
        let mut live = lock(&self.live);
        live.settings.reset_layout();
        live.save();
    }

    pub fn view(&self) -> OverlayView {
        let live = lock(&self.live);
        let s = &live.settings;
        let pose = live
            .pose
            .filter(|(_, t)| t.elapsed() < POSE_STALE)
            .map(|(p, _)| p);
        let gargoyle = live.gargoyle.filter(|_| live.in_match);
        OverlayView {
            settings: SettingsView {
                enabled: s.enabled,
                radar: s.radar(),
                countdown: s.countdown,
                countdown_format: s.countdown_format,
                port: s.port,
                demo_delay_s: s.demo_delay_s,
                any_gargoyle: s.any_gargoyle,
                despawn_timer: s.despawn_timer,
                radar_units: s.radar_units,
                toggle_key: KeyView::new(s.toggle_key),
                edit_key: KeyView::new(s.edit_key),
            },
            launch_options: s.launch_options(),
            setup_done: s.setup_done,
            status: StatusView {
                tf2_running: live.tf2_running,
                link: live.link,
                position_hz: live.position_hz,
                demo: live.demo_file.clone().map(|file| DemoView {
                    file,
                    interval_s: live.demo_interval_s,
                    filler_kbps: live.filler / 1000.0,
                }),
                gargoyle: gargoyle.map(|[x, y, z]| GargoyleView {
                    map: live.map.clone(),
                    x,
                    y,
                    z,
                    left_s: seconds_left(live.gargoyle_spawned).filter(|_| s.despawn_timer),
                }),
                reading: pose.zip(gargoyle).map(|(pose, g)| nav::reading(&pose, g)),
                editing: live.editing || live.edit_requested,
            },
            events: live.events.iter().cloned().collect(),
        }
    }
}

/// Sleeps in short steps so a stop is noticed quickly. Returns true when the radar is stopping.
fn nap(stop: &AtomicBool, total: Duration) -> bool {
    let start = Instant::now();
    while start.elapsed() < total {
        if stop.load(Ordering::Relaxed) {
            return true;
        }
        sleep(Duration::from_millis(50).min(total.saturating_sub(start.elapsed())));
    }
    stop.load(Ordering::Relaxed)
}
