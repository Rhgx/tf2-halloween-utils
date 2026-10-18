//! The gargoyle radar widget: an RCON thread (your position, the demo recording and the filler commands) and a
//! demo thread (your gargoyle) feed `Live`; `panel.rs` draws it.

pub mod demo;
pub mod nav;
pub mod panel;
pub mod rcon;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use demo::{DemoFinder, DemoFollower};
use nav::Reading;
use rcon::Rcon;

use crate::overlay::run::wait_for_frame;
use crate::overlay::{lock, nap, Link, Live, Tone};

pub const RECORD_RETRY: Duration = Duration::from_secs(10);

/// How long a gargoyle stays (tf_halloween_gift_lifetime). It is a cheat cvar, so this holds on every server
/// without sv_cheats.
pub const GARGOYLE_LIFETIME: Duration = Duration::from_secs(240);

pub const DEMO_NAME: &str = "gargoyle_radar";

/// With the filler at a 5 s delay the demo grows by up to about 500 MB an hour, and TF2 only starts it over on a
/// new map. Past this size it is started over once no gargoyle is being followed.
const DEMO_LIMIT: u64 = 100_000_000;

/// TF2 holds the demo in memory and writes it out in bursts of about this many bytes (measured: 672 to 704 KB),
/// about once a minute on a quiet local server. Console commands are recorded into the demo too, so harmless
/// filler commands over RCON make the bursts, and with them a new gargoyle, arrive sooner.
pub const TYPICAL_BURST: f32 = 700_000.0;

pub const FILLER_MAX: f32 = 300_000.0; // bytes per second

pub const FILLER_COMMAND: &str = "alias gargoyle_radar_fill";

/// What edit mode shows, so the height pin is visible.
pub const SAMPLE: Reading = Reading {
    angle_deg: 35.0,
    dist_hu: nav::hu(48.0),
    height_hu: nav::hu(4.0),
};

/// Whole seconds until a gargoyle that spawned at `spawned` despawns.
pub fn seconds_left(spawned: Option<Instant>) -> Option<u32> {
    spawned.map(|t| GARGOYLE_LIFETIME.saturating_sub(t.elapsed()).as_secs() as u32)
}

/// Polls `getpos` over RCON once per screen refresh, keeps a demo recording going and sends the filler.
pub fn rcon_worker(live: Arc<Mutex<Live>>, stop: Arc<AtomicBool>) {
    let mut tick = Instant::now();
    let mut link: Option<(Rcon, u16)> = None;
    let mut last_record: Option<Instant> = None;
    let mut filler_owed = 0.0f32;
    let mut last_fill = Instant::now();
    let filler_text = format!("{FILLER_COMMAND} \"{}\"", "x".repeat(480));
    let (mut replies, mut counted_since) = (0u32, Instant::now());
    while !stop.load(Ordering::Relaxed) {
        wait_for_frame(tick);
        tick = Instant::now();
        let (port, password, tf2_running) = {
            let l = lock(&live);
            (l.settings.port, l.settings.password.clone(), l.tf2_running)
        };
        // a changed port means reconnecting
        if link.as_ref().is_some_and(|(_, p)| *p != port) {
            link = None;
        }
        if !tf2_running {
            link = None;
            lock(&live).set_link(Link::Waiting);
            if nap(&stop, Duration::from_millis(500)) {
                break;
            }
            continue;
        }
        if link.is_none() {
            match Rcon::connect(port, &password) {
                Ok(mut r) => {
                    // TF2 also prints each getpos reply in its console; this hides them there (every reply
                    // line has "setang") while RCON still gets them
                    let _ = r.exec("con_filter_enable 1;con_filter_text_out setang");
                    link = Some((r, port));
                    lock(&live).set_link(Link::Connected);
                }
                Err(e) => {
                    let l = match e.kind() {
                        std::io::ErrorKind::PermissionDenied => Link::WrongPassword,
                        // a timeout means TF2 is busy (loading a map), not that RCON is off
                        std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock => {
                            Link::Waiting
                        }
                        _ => Link::Unreachable,
                    };
                    lock(&live).set_link(l);
                    if nap(&stop, Duration::from_secs(2)) {
                        break;
                    }
                    continue;
                }
            }
        }
        let Some((r, _)) = link.as_mut() else {
            continue;
        };
        let pose = match r.exec("getpos;getpos 2") {
            Ok(reply) => rcon::parse_pose(&reply),
            Err(e) => {
                link = None;
                let busy = matches!(
                    e.kind(),
                    std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
                );
                lock(&live).set_link(if busy {
                    Link::Waiting
                } else {
                    Link::Unreachable
                });
                continue;
            }
        };
        if pose.is_some() {
            replies += 1;
        }
        let rate = {
            let mut l = lock(&live);
            if let Some(pose) = pose {
                l.pose = Some((pose, Instant::now()));
            }
            if counted_since.elapsed() >= Duration::from_secs(1) {
                l.position_hz = (replies > 0).then_some(replies);
                (replies, counted_since) = (0, Instant::now());
            }
            // filler only while in a server (getpos answers) and only into our own recording, not one the user made
            let ours = l
                .demo_file
                .as_deref()
                .and_then(|f| std::path::Path::new(f).file_stem())
                .is_some_and(|s| s == DEMO_NAME);
            if pose.is_some() && ours {
                l.filler
            } else {
                0.0
            }
        };

        // never catch up on more than a second
        filler_owed = (filler_owed + rate * last_fill.elapsed().as_secs_f32()).min(rate);
        last_fill = Instant::now();
        let mut sent_ok = true;
        // one command per packet: TF2 drops the RCON connection when a packet holds more than about 510 bytes
        while filler_owed >= filler_text.len() as f32 && sent_ok {
            sent_ok = r.fire(&filler_text).is_ok();
            filler_owed -= filler_text.len() as f32;
        }
        if !sent_ok {
            link = None;
            lock(&live).set_link(Link::Unreachable);
            continue;
        }

        if std::mem::take(&mut lock(&live).restart_demo) {
            // in one command, so the new file starts at once; `record` overwrites the old one. If TF2 ignores it,
            // the retry below records again once the demo stops growing.
            let _ = r.exec(&format!("stop;record {DEMO_NAME}"));
            last_record = Some(Instant::now());
            lock(&live).event(
                Tone::Info,
                "Started the demo over at 100 MB, to save disk space.",
            );
        }
        let recording = lock(&live)
            .demo_grew
            .is_some_and(|t| t.elapsed() < RECORD_RETRY);
        if !recording && last_record.is_none_or(|t| t.elapsed() >= RECORD_RETRY) {
            // fails harmlessly in the main menu, or with "Already recording." if a demo is running
            let _ = r.exec(&format!("record {DEMO_NAME}"));
            last_record = Some(Instant::now());
        }
    }
}

/// Follows the demo TF2 is writing, publishes your gargoyle and tunes the filler.
pub fn demo_worker(live: Arc<Mutex<Live>>, stop: Arc<AtomicBool>) {
    let mut follower: Option<DemoFollower> = None;
    let mut caught_up = false;
    let mut last_burst: Option<Instant> = None;
    let mut last_scan: Option<Instant> = None;
    let mut finder = DemoFinder::default();
    let mut had_gargoyle = false;
    let mut restarted: Option<Instant> = None;
    while !nap(&stop, Duration::from_millis(250)) {
        let (tf_dir, any_owner, delay_s) = {
            let l = lock(&live);
            (
                l.tf_dir.clone(),
                l.settings.any_gargoyle,
                l.settings.demo_delay_s,
            )
        };
        let Some(tf_dir) = tf_dir else { continue };
        // listing the demo folders opens every .dem file, so only once a second
        let scan = last_scan.is_none_or(|t| t.elapsed() >= Duration::from_secs(1));
        if scan {
            last_scan = Some(Instant::now());
        }
        let found = if scan {
            finder.active(&tf_dir, Duration::from_secs(30))
        } else {
            follower.as_ref().map(|f| f.path().to_path_buf())
        };
        match found {
            Some(active) if follower.as_ref().is_none_or(|f| f.path() != active) => {
                let name = active
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                lock(&live).event(Tone::Good, format!("Reading the match from demo {name}."));
                follower = Some(DemoFollower::new(&active));
                caught_up = false;
            }
            // nothing has been written for 30 s: not recording, so not in a match
            None if follower.take().is_some() => {
                lock(&live).event(Tone::Info, "The demo stopped growing: not in a match.")
            }
            _ => {}
        }
        let mut burst = 0;
        if let Some(f) = follower.as_mut() {
            f.any_owner = any_owner;
            let before = f.bytes_read();
            if f.poll() {
                burst = f.bytes_read() - before;
            } else {
                follower = None;
            }
        }
        let gargoyle = follower.as_ref().and_then(|f| f.my_gargoyle());
        let age = follower.as_ref().and_then(|f| f.my_gargoyle_age());
        let mut l = lock(&live);
        if burst > 0 {
            // the first read is the file so far, not one burst
            if caught_up {
                let interval = last_burst.map_or(delay_s, |t| t.elapsed().as_secs_f32().max(0.25));
                // aim for one burst every `delay_s` seconds: add what the match itself does not write (half a
                // step at a time, so it settles instead of swinging)
                l.filler = (l.filler + 0.5 * burst as f32 * (1.0 / delay_s - 1.0 / interval))
                    .clamp(0.0, FILLER_MAX);
                l.demo_interval_s = Some(interval);
            }
            caught_up = true;
            last_burst = Some(Instant::now());
            l.demo_grew = Some(Instant::now());
        }
        l.map = follower.as_ref().and_then(|f| f.map.clone());
        if gargoyle.is_some() != had_gargoyle {
            match gargoyle {
                Some(g) => {
                    let map = l.map.clone().unwrap_or_else(|| "this map".to_string());
                    l.event(
                        Tone::Gargoyle,
                        format!(
                            "Gargoyle at ({:.0}, {:.0}, {:.0}) on {map}.",
                            g[0], g[1], g[2]
                        ),
                    );
                }
                None => l.event(Tone::Warn, "Gargoyle gone."),
            }
            had_gargoyle = gargoyle.is_some();
        }
        // ask the RCON thread to start our demo over once it is big, unless a gargoyle is being followed (its
        // despawn time would be lost). The file shrinks a moment later, so not again for a while.
        if let Some(f) = follower.as_ref() {
            if too_big(f.path(), f.bytes_read())
                && gargoyle.is_none()
                && restarted.is_none_or(|t| t.elapsed() > Duration::from_secs(30))
            {
                l.restart_demo = true;
                restarted = Some(Instant::now());
            }
        }
        l.in_match = follower.as_ref().is_some_and(|f| !f.ended);
        l.gargoyle = gargoyle;
        if gargoyle.is_none() {
            l.gargoyle_spawned = None;
        } else if burst > 0 {
            // the newest tick just read is about now (TF2 writes the demo in bursts), so the age dates the spawn
            l.gargoyle_spawned = age.and_then(|age| Instant::now().checked_sub(age));
        }
        l.demo_file = follower
            .as_ref()
            .and_then(|f| f.path().file_name())
            .map(|n| n.to_string_lossy().into_owned());
        if follower.is_none() {
            l.demo_interval_s = None;
        }
    }
}

/// Our own recording past the size limit; never a demo you are recording yourself.
fn too_big(path: &std::path::Path, size: u64) -> bool {
    path.file_stem().is_some_and(|s| s == DEMO_NAME) && size > DEMO_LIMIT
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn only_our_big_demo_is_started_over() {
        let ours = Path::new(r"C:\tf\gargoyle_radar.dem");
        assert!(too_big(ours, DEMO_LIMIT + 1));
        assert!(!too_big(ours, DEMO_LIMIT));
        assert!(!too_big(
            Path::new(r"C:\tf\demos\my_match.dem"),
            DEMO_LIMIT * 5
        ));
    }
}
