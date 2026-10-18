//! Edit mode, one frame at a time: drag a widget to move it (it snaps to the screen edges and the other widgets),
//! drag a corner grip to resize them all, and the keyboard fallback.

use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    VK_ADD, VK_DOWN, VK_HOME, VK_LBUTTON, VK_LEFT, VK_NEXT, VK_OEM_MINUS, VK_OEM_PLUS, VK_PRIOR,
    VK_RIGHT, VK_SHIFT, VK_SUBTRACT, VK_UP,
};

use super::input::{cursor, Keys};
use super::paint::GRIP;
use super::settings::OverlaySettings;
use super::widgets::{countdown, radar::panel};
use super::window::{Rect, Tf2};

/// The overlay's widgets, as edit mode tells them apart.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum WidgetId {
    Radar,
    Countdown,
}

impl WidgetId {
    /// Window size at scale `k`.
    pub fn pixels(self, k: f32) -> (u32, u32) {
        match self {
            WidgetId::Radar => panel::panel_pixels(k),
            WidgetId::Countdown => countdown::countdown_pixels(k),
        }
    }

    /// The widget's centre, as fractions of the TF2 client area.
    pub fn pos(self, s: &mut OverlaySettings) -> (&mut f32, &mut f32) {
        match self {
            WidgetId::Radar => (&mut s.pos_x, &mut s.pos_y),
            WidgetId::Countdown => (&mut s.countdown.pos_x, &mut s.countdown.pos_y),
        }
    }

    /// The widget's opacity and scale.
    pub fn look(self, s: &mut OverlaySettings) -> (&mut f32, &mut f32) {
        match self {
            WidgetId::Radar => (&mut s.opacity, &mut s.scale),
            WidgetId::Countdown => (&mut s.countdown.opacity, &mut s.countdown.scale),
        }
    }
}

#[derive(Clone, Copy)]
pub enum Drag {
    /// Cursor offset from the widget's top-left corner when the drag started.
    Move { widget: WidgetId, dx: i32, dy: i32 },
    /// The widget's top-left corner stays put; the size follows how far the cursor moved from where it grabbed.
    Resize {
        widget: WidgetId,
        start: (i32, i32),
        size: (i32, i32),
        scale: f32,
        left: i32,
        top: i32,
    },
}

/// Top-left of a window of `size` centred at `pos` (fractions of the TF2 client area), kept fully inside it.
pub fn place(tf: &Tf2, pos: (f32, f32), size: (u32, u32)) -> (i32, i32) {
    let (w, h) = (size.0 as i32, size.1 as i32);
    (
        ((pos.0 * tf.w as f32) as i32 - w / 2).clamp(0, (tf.w - w).max(0)),
        ((pos.1 * tf.h as f32) as i32 - h / 2).clamp(0, (tf.h - h).max(0)),
    )
}

/// Moves a span `size` long starting at `start` so one of its edges lands on a nearby line, if one is within
/// `reach`: the screen edges `0` and `limit`, and both edges of each `(start, size)` in `others`.
pub fn snap(start: i32, size: i32, limit: i32, others: &[(i32, i32)], reach: i32) -> i32 {
    let lines = [0, limit]
        .into_iter()
        .chain(others.iter().flat_map(|&(a, len)| [a, a + len]));
    let nearest = lines
        .flat_map(|line| [line - start, line - (start + size)])
        .filter(|d| d.abs() <= reach)
        .min_by_key(|d| d.abs());
    start + nearest.unwrap_or(0)
}

/// One frame of edit mode: drag a widget to move it (it snaps to the screen edges and the other widgets), drag its
/// corner grip to resize it, and a keyboard fallback that moves, resizes and fades the widget grabbed last.
#[allow(clippy::too_many_arguments)]
pub fn edit_frame(
    settings: &mut OverlaySettings,
    keys: &mut Keys,
    drag: &mut Option<Drag>,
    grabbed: &mut WidgetId,
    tf: &Tf2,
    s: f32,
    shown: &[(WidgetId, Rect)],
) {
    let (mx, my) = cursor();
    if keys.pressed(VK_LBUTTON) {
        let hit = shown
            .iter()
            .find(|(_, (x, y, w, h))| mx >= *x && mx < x + w && my >= *y && my < y + h);
        if let Some(&(widget, (px, py, pw, ph))) = hit {
            *grabbed = widget;
            let scale = *widget.look(settings).1;
            let grip = (GRIP + 6.0) * s * scale;
            let on_grip =
                (mx - px) as f32 >= pw as f32 - grip && (my - py) as f32 >= ph as f32 - grip;
            *drag = Some(if on_grip {
                Drag::Resize {
                    widget,
                    start: (mx, my),
                    size: (pw, ph),
                    scale,
                    left: px - tf.x,
                    top: py - tf.y,
                }
            } else {
                Drag::Move {
                    widget,
                    dx: mx - px,
                    dy: my - py,
                }
            });
        }
    }
    if !Keys::down(VK_LBUTTON) {
        *drag = None;
    }
    match *drag {
        Some(Drag::Move { widget, dx, dy }) => {
            let (w, h) = widget.pixels(s * *widget.look(settings).1);
            let (w, h) = (w as i32, h as i32);
            // the other widgets' spans on each axis, in client coordinates
            let others: Vec<(i32, i32, i32, i32)> = shown
                .iter()
                .filter(|(id, _)| *id != widget)
                .map(|&(_, (x, y, w, h))| (x - tf.x, y - tf.y, w, h))
                .collect();
            let across: Vec<(i32, i32)> = others.iter().map(|o| (o.0, o.2)).collect();
            let down: Vec<(i32, i32)> = others.iter().map(|o| (o.1, o.3)).collect();
            let reach = (8.0 * s).round() as i32;
            let left = snap(mx - dx - tf.x, w, tf.w, &across, reach);
            let top = snap(my - dy - tf.y, h, tf.h, &down, reach);
            let (x, y) = widget.pos(settings);
            *x = (left + w / 2) as f32 / tf.w as f32;
            *y = (top + h / 2) as f32 / tf.h as f32;
        }
        Some(Drag::Resize {
            widget,
            start,
            size,
            scale,
            left,
            top,
        }) => {
            // the grabbed corner follows the cursor: grow by whichever of width or height grew more, so dragging
            // across or down both work, and never jump on the first frame
            let grow_w = (size.0 + mx - start.0) as f32 / size.0.max(1) as f32;
            let grow_h = (size.1 + my - start.1) as f32 / size.1.max(1) as f32;
            let scale = (scale * grow_w.max(grow_h)).clamp(0.5, 2.5);
            *widget.look(settings).1 = scale;
            // keep the widget's top-left corner where it was: the centre is half the new size from it
            let (pw, ph) = widget.pixels(s * scale);
            let (x, y) = widget.pos(settings);
            *x = (left as f32 + pw as f32 / 2.0) / tf.w as f32;
            *y = (top as f32 + ph as f32 / 2.0) / tf.h as f32;
        }
        None => {}
    }

    let step = if Keys::down(VK_SHIFT) { 10.0 } else { 1.0 };
    for (vk, dx, dy) in [
        (VK_LEFT, -1.0, 0.0),
        (VK_RIGHT, 1.0, 0.0),
        (VK_UP, 0.0, -1.0),
        (VK_DOWN, 0.0, 1.0),
    ] {
        if keys.repeating(vk) {
            let (x, y) = grabbed.pos(settings);
            *x += dx * step / tf.w as f32;
            *y += dy * step / tf.h as f32;
        }
    }
    let (opacity, scale) = grabbed.look(settings);
    if keys.repeating(VK_OEM_PLUS) || keys.repeating(VK_ADD) {
        *scale *= 1.05;
    }
    if keys.repeating(VK_OEM_MINUS) || keys.repeating(VK_SUBTRACT) {
        *scale /= 1.05;
    }
    if keys.repeating(VK_PRIOR) {
        *opacity += 0.05;
    }
    if keys.repeating(VK_NEXT) {
        *opacity -= 0.05;
    }
    if keys.pressed(VK_HOME) {
        settings.reset_layout();
    }
    settings.clamp();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapping_to_edges_and_other_widgets() {
        // near the screen edges
        assert_eq!(snap(5, 100, 1000, &[], 8), 0);
        assert_eq!(snap(895, 100, 1000, &[], 8), 900);
        // the left edge meets another widget's right edge, or lines up with its left edge
        assert_eq!(snap(306, 50, 1000, &[(200, 100)], 8), 300);
        assert_eq!(snap(195, 50, 1000, &[(200, 100)], 8), 200);
        // nothing within reach: stays put
        assert_eq!(snap(500, 50, 1000, &[(200, 100)], 8), 500);
    }
}
