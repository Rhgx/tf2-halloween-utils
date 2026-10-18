//! Direction, distance and height from your pose to the gargoyle. Used raw, like TF2's own CTF flag
//! compass (CTFArrowPanel::GetAngleRotation), so the arrow stays snappy.

use serde::{Deserialize, Serialize};

use super::rcon::Pose;

/// A hammer unit, TF2's own and what its positions come in, is 0.75 inch.
pub const METRES_PER_HU: f32 = 0.01905;

/// `metres` in hammer units, for limits that read better in metres.
pub const fn hu(metres: f32) -> f32 {
    metres / METRES_PER_HU
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Reading {
    /// Signed horizontal angle from where you look to the gargoyle; 0 = ahead, positive = to your right.
    pub angle_deg: f32,
    /// Distance and height in hammer units; height is relative to your feet, positive = above you.
    pub dist_hu: f32,
    pub height_hu: f32,
}

/// Wraps an angle into (-180, 180].
fn wrap(deg: f32) -> f32 {
    let a = (deg + 180.0).rem_euclid(360.0) - 180.0;
    if a == -180.0 {
        180.0
    } else {
        a
    }
}

pub fn reading(pose: &Pose, gargoyle: [f32; 3]) -> Reading {
    let (dx, dy) = (gargoyle[0] - pose.eye[0], gargoyle[1] - pose.eye[1]);
    // TF2 yaw grows counter-clockwise from +X, so a target clockwise of your view (to your right) has a
    // smaller bearing than your yaw
    let bearing = dy.atan2(dx).to_degrees();
    let height = gargoyle[2] - pose.feet[2];
    Reading {
        angle_deg: wrap(pose.yaw - bearing),
        dist_hu: (dx * dx + dy * dy + height * height).sqrt(),
        height_hu: height,
    }
}

/// The unit the panel writes distances in.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum Units {
    #[default]
    Metres,
    Feet,
    /// TF2's own unit, 0.75 inch.
    Hammer,
}

impl Units {
    /// One of this unit: its length in hammer units, and its label.
    const fn spec(self) -> (f32, &'static str) {
        match self {
            Units::Hammer => (1.0, "hu"),
            Units::Feet => (16.0, "ft"), // 12 in / 0.75 in, exact
            Units::Metres => (hu(1.0), "m"),
        }
    }

    /// The smallest change the panel shows of a `value` in this unit: tenths under 10, then whole units. Hammer
    /// units go in fives, since your position jitters by a few of them.
    fn step(self, value: f32) -> f32 {
        match self {
            Units::Hammer => 5.0,
            _ if value.abs() < 10.0 => 0.1,
            _ => 1.0,
        }
    }

    /// A length in hammer units, written in this unit as the panel shows it: "4.6 m", "15 ft", "240 hu".
    pub fn amount(self, hu: f32) -> String {
        let (size, unit) = self.spec();
        let v = hu / size;
        if self.step(v) < 1.0 {
            format!("{v:.1} {unit}")
        } else {
            format!("{v:.0} {unit}")
        }
    }
}

/// Holds the distance and height the panel shows steady. Your reported position jitters a little (view bob,
/// small movements), so a value near a rounding edge would flip between two numbers every frame; a shown number
/// changes only once the real one is three quarters of a step away, in the unit shown. The arrow is not held:
/// it stays snappy.
#[derive(Default)]
pub struct Steady {
    dist: Option<f32>,
    height: Option<f32>,
}

impl Steady {
    pub fn hold(&mut self, r: Reading, units: Units) -> Reading {
        let (size, _) = units.spec();
        let hold = |shown: &mut Option<f32>, hu: f32| {
            let value = hu / size;
            held(shown, value, units.step(value)) * size
        };
        Reading {
            dist_hu: hold(&mut self.dist, r.dist_hu),
            height_hu: hold(&mut self.height, r.height_hu),
            ..r
        }
    }
}

fn held(shown: &mut Option<f32>, value: f32, step: f32) -> f32 {
    match *shown {
        Some(s) if (value - s).abs() < 0.75 * step => s,
        _ => *shown.insert((value / step).round() * step),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shown_distance_does_not_flicker_at_a_rounding_edge() {
        let mut shown = None;
        let seen: Vec<f32> = [23.4, 23.6, 23.45, 23.55, 23.8, 23.4, 22.2]
            .into_iter()
            .map(|v| held(&mut shown, v, 1.0))
            .collect();
        // jitter around 23.5 stays 23; it moves on at 23.8, keeps 24 back at 23.4, and follows a real change
        assert_eq!(seen, [23.0, 23.0, 23.0, 23.0, 24.0, 24.0, 22.0]);
        // a tenth
        let mut shown = None;
        let near = |a: f32, b: f32| (a - b).abs() < 1e-4;
        assert!(near(held(&mut shown, 8.44, 0.1), 8.4));
        assert!(near(held(&mut shown, 8.46, 0.1), 8.4));
        assert!(near(held(&mut shown, 8.6, 0.1), 8.6));
    }

    fn pose(yaw: f32) -> Pose {
        Pose {
            eye: [0.0, 0.0, 64.0],
            pitch: 0.0,
            yaw,
            feet: [0.0, 0.0, 0.0],
        }
    }

    #[test]
    fn directions_follow_tf2_yaw() {
        // facing +Y (yaw 90): +X is to the right, -X to the left, -Y behind
        assert!((reading(&pose(90.0), [100.0, 0.0, 0.0]).angle_deg - 90.0).abs() < 1e-3);
        assert!((reading(&pose(90.0), [-100.0, 0.0, 0.0]).angle_deg + 90.0).abs() < 1e-3);
        assert!((reading(&pose(90.0), [0.0, -100.0, 0.0]).angle_deg.abs() - 180.0).abs() < 1e-3);
        assert!(reading(&pose(90.0), [0.0, 100.0, 0.0]).angle_deg.abs() < 1e-3);
        // wrap-around: facing yaw -170, target at bearing 170 is 20 degrees to the right
        let r = reading(
            &pose(-170.0),
            [
                (170f32).to_radians().cos() * 100.0,
                (170f32).to_radians().sin() * 100.0,
                0.0,
            ],
        );
        assert!((r.angle_deg + 20.0).abs() < 1e-3 || (r.angle_deg - 20.0).abs() < 1e-3);
        assert!((r.angle_deg - (-170.0 - 170.0 + 360.0)).abs() < 1e-3);
    }

    #[test]
    fn distance_and_height_use_the_feet() {
        let r = reading(&pose(0.0), [300.0, 400.0, 105.0]);
        assert!((r.dist_hu - (300f32 * 300.0 + 400.0 * 400.0 + 105.0 * 105.0).sqrt()).abs() < 1e-4);
        assert!((r.height_hu - 105.0).abs() < 1e-5);
    }

    #[test]
    fn units_write_the_same_distance() {
        assert_eq!(Units::Metres.amount(hu(4.6)), "4.6 m");
        assert_eq!(Units::Metres.amount(hu(23.4)), "23 m");
        assert_eq!(Units::Feet.amount(16.0), "1.0 ft");
        assert_eq!(Units::Feet.amount(hu(23.4)), "77 ft");
        assert_eq!(Units::Hammer.amount(100.0), "100 hu");
        // hammer units are held in fives, so a few units of jitter do not show
        let mut steady = Steady::default();
        let r = |dist_hu: f32| Reading {
            angle_deg: 0.0,
            dist_hu,
            height_hu: 0.0,
        };
        assert_eq!(steady.hold(r(240.0), Units::Hammer).dist_hu, 240.0);
        assert_eq!(steady.hold(r(241.5), Units::Hammer).dist_hu, 240.0);
        assert_eq!(steady.hold(r(244.0), Units::Hammer).dist_hu, 245.0);
    }
}
