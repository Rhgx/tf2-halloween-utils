//! The radar panel: a ground compass in perspective (bright front edge, dim back edge), an arrow lying in that
//! plane, a height pin, the distance and the despawn countdown.

use tiny_skia::{Color, Path as SkPath, PathBuilder, Pixmap, Transform};

use super::nav::{hu, Reading, Units, METRES_PER_HU};
use crate::overlay::paint::{
    edit_outline, fill, line, pixels, polygon, rounded_rect, stroke, Align, Painter, Rgb, CREAM,
    ORANGE, PANEL_BG, PANEL_W, RIM, TAN_DARK, TAN_TEXT,
};

pub const PANEL_H: f32 = 100.0;

pub const LEVEL: f32 = hu(1.5);
pub const ARRIVED: f32 = hu(2.5);
pub const RING_CX: f32 = 56.0;
pub const RING_CY: f32 = 50.0;
pub const RING_RX: f32 = 40.0;
pub const RING_RY: f32 = 18.0; // 0.45 of rx: a compass on the floor seen from about 27 degrees above
/// The height pin grows 4 panel units per metre.
pub const PIN_PX_PER_HU: f32 = 4.0 * METRES_PER_HU;
pub const PIN_MAX: f32 = 24.0;
pub const TEXT_X: f32 = 112.0;
/// The text column's right edge: the texts end here, so what follows a changing number stays put.
pub const TEXT_RIGHT: f32 = PANEL_W - 8.0;

/// Window size for the radar panel, with room for the drop shadow.
pub fn panel_pixels(k: f32) -> (u32, u32) {
    pixels(PANEL_H, k)
}

/// Height pin from the arrow tip: a stalk of `len` panel units (positive = up) ending in a rimmed dot.
pub fn pin(pm: &mut Pixmap, tip: (f32, f32), len: f32, colour: Rgb, t: Transform, k: f32) {
    let end = (tip.0, tip.1 - len);
    if len != 0.0 {
        stroke(pm, &line(tip, end), colour, 2.0, None, t, k);
    }
    let dot = PathBuilder::from_circle(end.0, end.1, 2.5).unwrap();
    fill(pm, &dot, colour, 1.0, t, k);
    stroke(pm, &dot, RIM, 1.0, None, t, 0.0);
}

/// Back (upper) or front (lower) half of the compass ellipse.
pub fn half_ellipse(back: bool) -> SkPath {
    let start = if back { std::f32::consts::PI } else { 0.0 };
    let mut pb = PathBuilder::new();
    for i in 0..=32 {
        let a = start + std::f32::consts::PI * i as f32 / 32.0;
        let p = (RING_CX + RING_RX * a.cos(), RING_CY + RING_RY * a.sin());
        if i == 0 {
            pb.move_to(p.0, p.1)
        } else {
            pb.line_to(p.0, p.1)
        }
    }
    pb.finish().unwrap()
}

impl Painter {
    /// The radar panel, with distances in `units`. `left_s` is how long until the gargoyle despawns, when known.
    /// `edit` adds the dashed outline and resize grip.
    #[allow(clippy::too_many_arguments)]
    pub fn panel(
        &self,
        pm: &mut Pixmap,
        k: f32,
        opacity: f32,
        r: &Reading,
        units: Units,
        left_s: Option<u32>,
        edit: bool,
    ) {
        pm.fill(Color::TRANSPARENT);
        let t = Transform::from_scale(k, k);
        let bg = rounded_rect(0.0, 0.0, PANEL_W, PANEL_H, 4.0);
        fill(pm, &bg, PANEL_BG, opacity, t, k);

        let arrived = r.dist_hu < ARRIVED;
        let accent = if arrived { ORANGE } else { CREAM };
        let flat = RING_RY / RING_RX;
        // the arrow lies on the ground plane: forward is up the screen, foreshortened by `flat`
        let (sin, cos) = r.angle_deg.to_radians().sin_cos();
        let ground = |side: f32, fwd: f32| (RING_CX + side, RING_CY - fwd * flat);
        let tip = if arrived {
            (RING_CX, RING_CY)
        } else {
            ground(sin * 0.85 * RING_RX, cos * 0.85 * RING_RX)
        };
        let level = r.height_hu.abs() < LEVEL;
        let pin_len = if level {
            0.0
        } else {
            (r.height_hu * PIN_PX_PER_HU).clamp(-PIN_MAX, PIN_MAX)
        };

        // below you: the pin hangs under the floor, so the ring is drawn over it
        if pin_len < 0.0 {
            pin(pm, tip, pin_len, accent, t, k);
        }
        stroke(pm, &half_ellipse(true), TAN_DARK, 2.5, None, t, k);
        stroke(
            pm,
            &line(
                (RING_CX, RING_CY - RING_RY),
                (RING_CX, RING_CY - RING_RY - 6.0),
            ),
            TAN_DARK,
            2.0,
            None,
            t,
            k,
        );
        stroke(pm, &half_ellipse(false), accent, 2.5, None, t, k);
        if !arrived {
            let base = ground(-sin * 0.35 * RING_RX, -cos * 0.35 * RING_RX);
            // the width is only partly foreshortened, or the dart turns into a sliver at diagonal angles
            // perpendicular in the ground plane is (cos, -sin); forward maps to screen -y, so the wing's y is +sin
            let (wx, wy) = (cos * 0.42 * RING_RX, sin * 0.42 * RING_RX * flat.sqrt());
            let dart = polygon(&[
                tip,
                (base.0 + wx, base.1 + wy),
                (RING_CX, RING_CY),
                (base.0 - wx, base.1 - wy),
            ]);
            fill(pm, &dart, CREAM, 1.0, t, k);
        }
        if pin_len >= 0.0 {
            pin(pm, tip, pin_len, accent, t, k);
        }

        let distance = if arrived {
            "HERE".to_string()
        } else {
            units.amount(r.dist_hu)
        };
        // the texts get what is right of the compass, ending at its right edge so units and left never move as the
        // digits change; long ones (hundreds of hammer units) shrink to fit
        let room = (TEXT_RIGHT - TEXT_X) * k;
        let fit = |font, text: &str, px: f32| px * (room / self.measure(font, text, px)).min(1.0);
        self.text(
            pm,
            &self.big,
            TEXT_RIGHT * k,
            44.0 * k,
            &distance,
            fit(&self.big, &distance, 30.0 * k),
            accent,
            Align::Right,
            k,
        );
        let (height, colour) = if arrived {
            let look = if level {
                "level"
            } else if r.height_hu > 0.0 {
                "look up"
            } else {
                "look down"
            };
            (look.to_string(), ORANGE)
        } else if level {
            ("level".to_string(), TAN_TEXT)
        } else {
            let amount = units.amount(r.height_hu.abs());
            // below is where you cannot see it: dimmed, so above and below differ at a glance
            if r.height_hu > 0.0 {
                (format!("{amount} above"), CREAM)
            } else {
                (format!("{amount} below"), TAN_TEXT)
            }
        };
        self.text(
            pm,
            &self.small,
            TEXT_RIGHT * k,
            70.0 * k,
            &height,
            fit(&self.small, &height, 15.0 * k),
            colour,
            Align::Right,
            k,
        );
        if let Some(s) = left_s {
            // the last half minute is when it is worth hurrying
            let colour = if s <= 30 { ORANGE } else { TAN_TEXT };
            self.text(
                pm,
                &self.small,
                TEXT_RIGHT * k,
                90.0 * k,
                &format!("{}:{:02} left", s / 60, s % 60),
                15.0 * k,
                colour,
                Align::Right,
                k,
            );
        }

        if edit {
            edit_outline(pm, PANEL_H, t);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::overlay::paint::preview::{painter, save_strip};

    /// Renders the states onto a dark and a bright strip and writes radar_preview.bmp to the temp folder.
    /// Run with: cargo test --release -- --ignored preview
    #[test]
    #[ignore]
    fn preview() {
        let painter = painter();
        let k = 1.5;
        let states = [
            (
                Reading {
                    angle_deg: 0.0,
                    dist_hu: hu(63.0),
                    height_hu: hu(0.4),
                },
                false,
            ),
            (
                Reading {
                    angle_deg: 60.0,
                    dist_hu: hu(8.4),
                    height_hu: hu(4.6),
                },
                false,
            ),
            (
                Reading {
                    angle_deg: -140.0,
                    dist_hu: hu(31.0),
                    height_hu: hu(-12.0),
                },
                false,
            ),
            (
                Reading {
                    angle_deg: 95.0,
                    dist_hu: hu(18.0),
                    height_hu: hu(-3.0),
                },
                false,
            ),
            (
                Reading {
                    angle_deg: 0.0,
                    dist_hu: hu(1.6),
                    height_hu: hu(2.2),
                },
                false,
            ),
            (
                Reading {
                    angle_deg: 35.0,
                    dist_hu: hu(48.0),
                    height_hu: hu(4.0),
                },
                true,
            ),
        ];
        let left = [Some(212), Some(95), Some(24), None, Some(8), Some(192)];
        // the far one in hammer units shows the text shrinking to fit
        let units = [
            Units::Metres,
            Units::Feet,
            Units::Hammer,
            Units::Metres,
            Units::Metres,
            Units::Metres,
        ];
        let (pw, ph) = panel_pixels(k);
        let tiles: Vec<Pixmap> = states
            .iter()
            .zip(left)
            .zip(units)
            .map(|(((r, edit), left), units)| {
                let mut pm = Pixmap::new(pw, ph).unwrap();
                painter.panel(&mut pm, k, 0.78, r, units, left, *edit);
                pm
            })
            .collect();
        save_strip("radar_preview.bmp", &tiles);
    }
}
