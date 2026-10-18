//! The overlay thread: follows TF2's window, draws the widgets that are on, and handles the hotkeys and edit mode.

use std::mem::zeroed;
use std::ptr::{null, null_mut};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::sleep;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use tiny_skia::Pixmap;
use windows_sys::Win32::Graphics::Dwm::DwmFlush;
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, GetForegroundWindow, PeekMessageW, RegisterClassW, TranslateMessage, MSG,
    PM_REMOVE, WNDCLASSW,
};

use super::edit::{edit_frame, place, Drag, WidgetId};
use super::input::{key_name, Keys};
use super::paint::Painter;
use super::widgets::countdown;
use super::widgets::radar::{nav, panel, seconds_left, SAMPLE};
use super::window::{tf2_window, tf_dir_of, wide, window_proc, Layer, Rect};
use super::{lock, nap, Link, Live, Tone, POSE_STALE};

pub const IDLE: Duration = Duration::from_millis(100); // overlay loop pace while TF2 is not in front

pub const NOTICE: Duration = Duration::from_secs(2);

/// Waits for the next frame the desktop composes, so the loops run at the screen's refresh rate.
pub fn wait_for_frame(since: Instant) {
    // DwmFlush can return at once when nothing is being composed; never run faster than 250 per second
    if unsafe { DwmFlush() } < 0 || since.elapsed() < Duration::from_millis(4) {
        sleep(
            Duration::from_millis(4)
                .saturating_sub(since.elapsed())
                .max(Duration::from_millis(1)),
        );
    }
}

/// The in-game widgets: follows TF2's window, draws the radar and the countdown, handles the two hotkeys and
/// edit mode.
pub fn overlay_thread(live: Arc<Mutex<Live>>, stop: Arc<AtomicBool>) {
    let mut painter = match Painter::load(None) {
        Ok(p) => p,
        Err(e) => {
            lock(&live).event(Tone::Bad, format!("The overlay cannot start: {e}"));
            return;
        }
    };
    let class = wide("GargoyleRadarOverlay");
    unsafe {
        let wc = WNDCLASSW {
            lpfnWndProc: Some(window_proc),
            hInstance: GetModuleHandleW(null()),
            lpszClassName: class.as_ptr(),
            ..zeroed()
        };
        // fails harmlessly with "class already exists" when the overlay is switched on a second time
        RegisterClassW(&wc);
    }
    let (mut radar_win, mut countdown_win, mut help_win, mut corner_win) = (
        Layer::new(&class),
        Layer::new(&class),
        Layer::new(&class),
        Layer::new(&class),
    );

    let mut keys = Keys::default();
    let mut paused = false;
    let mut editing = false;
    let mut drag: Option<Drag> = None;
    let mut grabbed = WidgetId::Radar;
    let mut notice: Option<(&str, Instant)> = None;
    let mut had_gargoyle = false;
    let mut steady = nav::Steady::default();
    let mut fonts_from_tf2 = false;
    let mut help_card: Option<(String, Pixmap)> = None;
    // the contracts' end, read once from the running TF2's files (the inner None: they hold no date)
    let mut end: Option<Option<countdown::End>> = None;
    while !stop.load(Ordering::Relaxed) {
        let frame_start = Instant::now();
        unsafe {
            let mut msg: MSG = zeroed();
            while PeekMessageW(&mut msg, null_mut(), 0, 0, PM_REMOVE) != 0 {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }

        let window = tf2_window();
        let tf_dir = {
            let mut l = lock(&live);
            l.tf2_running = window.is_some();
            if let Some(tf) = &window {
                if l.tf_dir.is_none() {
                    l.tf_dir = tf_dir_of(tf.hwnd);
                }
                if !fonts_from_tf2 {
                    if let Some(Ok(p)) = l.tf_dir.as_deref().map(|d| Painter::load(Some(d))) {
                        fonts_from_tf2 = p.tf2_fonts;
                        painter = p;
                    }
                }
            }
            l.tf_dir.clone()
        };
        if end.is_none() {
            if let Some(tf_dir) = tf_dir {
                // items_game.txt is large, so this reads it outside the lock, once
                end = Some(
                    tf_dir
                        .parent()
                        .and_then(crate::tf2::items_game::contracts_end)
                        .as_deref()
                        .and_then(countdown::parse_end),
                );
            }
        }
        let (toggle_key, edit_key) = {
            let l = lock(&live);
            (l.settings.toggle_key, l.settings.edit_key)
        };
        let Some(tf) = window.filter(|tf| unsafe { GetForegroundWindow() } == tf.hwnd) else {
            radar_win.hide();
            countdown_win.hide();
            help_win.hide();
            corner_win.hide();
            for vk in [toggle_key, edit_key] {
                keys.pressed(vk); // swallow presses made outside the game
            }
            nap(&stop, IDLE.saturating_sub(frame_start.elapsed()));
            continue;
        };

        if keys.pressed(toggle_key) {
            paused = !paused;
            notice = Some((
                if paused { "Overlay off" } else { "Overlay on" },
                Instant::now(),
            ));
        }
        let requested = std::mem::take(&mut lock(&live).edit_requested);
        if keys.pressed(edit_key) || (requested && !editing) {
            editing = !editing;
            drag = None;
            radar_win.set_click_through(!editing);
            countdown_win.set_click_through(!editing);
            if !editing {
                let mut l = lock(&live);
                l.settings.clamp();
                l.save();
                notice = Some(("Overlay layout saved", Instant::now()));
            }
        }

        let s = tf.h as f32 / 1080.0;
        let shown: Vec<(WidgetId, Rect)> = [
            (WidgetId::Radar, &radar_win),
            (WidgetId::Countdown, &countdown_win),
        ]
        .into_iter()
        .filter(|(_, win)| win.visible)
        .map(|(id, win)| (id, win.rect))
        .collect();
        let (settings, link, pose, gargoyle, left_s) = {
            let mut l = lock(&live);
            l.editing = editing;
            if editing {
                edit_frame(
                    &mut l.settings,
                    &mut keys,
                    &mut drag,
                    &mut grabbed,
                    &tf,
                    s,
                    &shown,
                );
            }
            let pose = l
                .pose
                .filter(|(_, t)| t.elapsed() < POSE_STALE)
                .map(|(p, _)| p);
            (
                l.settings.clone(),
                l.link,
                pose,
                l.gargoyle.filter(|_| l.in_match),
                seconds_left(l.gargoyle_spawned).filter(|_| l.settings.despawn_timer),
            )
        };
        // where edit mode's help card goes: under the first widget on screen
        let mut anchor: Option<(i32, i32, (u32, u32))> = None;

        if had_gargoyle && gargoyle.is_none() {
            notice = Some(("Gargoyle gone", Instant::now()));
        }
        had_gargoyle = gargoyle.is_some();

        let reading = match (settings.radar_enabled, editing, paused, pose, gargoyle) {
            (false, ..) => None,
            (true, true, ..) => Some(SAMPLE),
            (true, false, false, Some(pose), Some(g)) => {
                Some(steady.hold(nav::reading(&pose, g), settings.radar_units))
            }
            _ => None,
        };
        match reading {
            Some(r) => {
                let k = s * settings.scale;
                let size = panel::panel_pixels(k);
                let (x, y) = place(&tf, (settings.pos_x, settings.pos_y), size);
                anchor = Some((x, y, size));
                // redraw only when the picture changes; a drag only moves the window
                let (px, py) = (tf.x + x, tf.y + y);
                let left_s = if editing {
                    settings.despawn_timer.then_some(192)
                } else {
                    left_s
                };
                if radar_win.stale(format!(
                    "{r:?} {:?} {left_s:?} {k} {} {editing} {fonts_from_tf2}",
                    settings.radar_units, settings.opacity
                )) {
                    if let Some(mut pm) = Pixmap::new(size.0, size.1) {
                        painter.panel(
                            &mut pm,
                            k,
                            settings.opacity,
                            &r,
                            settings.radar_units,
                            left_s,
                            editing,
                        );
                        radar_win.show(px, py, &pm);
                    }
                } else {
                    radar_win.move_to(px, py);
                }
            }
            None => radar_win.hide(),
        }

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_secs() as i64);
        let counting = match end.flatten() {
            Some(e) if settings.countdown.enabled && (editing || !paused) => {
                countdown::text(e.unix - now, e.local, settings.countdown_format)
            }
            _ => None,
        };
        // edit mode shows it even without a date to count to, so it can be placed
        let counting = counting.or_else(|| {
            (editing && settings.countdown.enabled)
                .then(|| countdown::sample(settings.countdown_format))
        });
        match counting {
            Some(c) => {
                let (k, opacity) = (s * settings.countdown.scale, settings.countdown.opacity);
                let size = countdown::countdown_pixels(k);
                let (x, y) = place(
                    &tf,
                    (settings.countdown.pos_x, settings.countdown.pos_y),
                    size,
                );
                anchor = anchor.or(Some((x, y, size)));
                let (px, py) = (tf.x + x, tf.y + y);
                if countdown_win.stale(format!("{c:?} {k} {opacity} {editing} {fonts_from_tf2}")) {
                    if let Some(mut pm) = Pixmap::new(size.0, size.1) {
                        painter
                            .countdown(&mut pm, k, opacity, &c.big, &c.small, c.urgency, editing);
                        countdown_win.show(px, py, &pm);
                    }
                } else {
                    countdown_win.move_to(px, py);
                }
            }
            None => countdown_win.hide(),
        }

        match anchor.filter(|_| editing) {
            Some((x, y, (pw, ph))) => {
                // the card's text is the slow part to draw, so it is drawn once per size and opacity
                let card_opacity = settings.opacity.max(settings.countdown.opacity).max(0.6);
                let save_key = key_name(edit_key);
                let card_key = format!("{s} {card_opacity} {fonts_from_tf2} {save_key}");
                if help_card.as_ref().is_none_or(|(key, _)| *key != card_key) {
                    help_card = Some((
                        card_key.clone(),
                        painter.help_card(s, card_opacity, &save_key),
                    ));
                }
                if let Some((_, card)) = &help_card {
                    let (cw, ch) = (card.width() as i32, card.height() as i32);
                    let below = y + ph as i32 + (8.0 * s) as i32;
                    let cy = if below + ch <= tf.h {
                        below
                    } else {
                        (y - ch - (8.0 * s) as i32).max(0)
                    };
                    let cx = (x + pw as i32 / 2 - cw / 2).clamp(0, (tf.w - cw).max(0));
                    if help_win.stale(card_key) {
                        help_win.show(tf.x + cx, tf.y + cy, card);
                    } else {
                        help_win.move_to(tf.x + cx, tf.y + cy);
                    }
                }
            }
            None => help_win.hide(),
        }

        // corner text: short notices, otherwise only the radar's faults; a blank corner is the normal state
        let fault = match (settings.radar_enabled, link, pose) {
            _ if paused => None,
            (false, ..) | (true, Link::Connected, Some(_)) => None,
            (true, Link::Connected, None) => Some("Radar: waiting for position"),
            _ => Some("Radar: not connected to TF2"),
        };
        let edit_text = format!("Edit mode: {} saves", key_name(edit_key));
        let text = if editing {
            Some(edit_text.as_str())
        } else {
            notice
                .filter(|(_, t)| t.elapsed() < NOTICE)
                .map(|(n, _)| n)
                .or(fault)
        };
        match text {
            Some(text) if corner_win.stale(format!("{text} {fonts_from_tf2} {tf:?}")) => {
                let (cw, ch) = ((420.0 * s) as u32, (40.0 * s) as u32);
                let Some(mut pm) = Pixmap::new(cw, ch) else {
                    continue;
                };
                painter.corner(&mut pm, text, s);
                corner_win.show(
                    tf.x + tf.w - (25.0 * s) as i32 - cw as i32,
                    tf.y + tf.h - (19.0 * s) as i32 - ch as i32,
                    &pm,
                );
            }
            Some(_) => {}
            None => corner_win.hide(),
        }
        wait_for_frame(frame_start);
    }
    lock(&live).editing = false;
    // the four layers drop here, destroying their windows on this thread
}
