//! The Scream Fortress countdown widget: the time left until the contracts end, when that is in your local time,
//! and its panel.

use serde::{Deserialize, Serialize};
use tiny_skia::{Color, Pixmap, Transform};
use windows_sys::Win32::Foundation::SYSTEMTIME;
use windows_sys::Win32::System::Time::SystemTimeToTzSpecificLocalTime;

use crate::overlay::paint::{
    edit_outline, fill, pixels, rounded_rect, Align, Painter, CREAM, ORANGE, PANEL_BG, PANEL_W,
    TAN_TEXT,
};

const DAY: i64 = 86_400;
const HOUR: i64 = 3_600;
const WEEKDAYS: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

/// When the contracts end: UNIX seconds, and the same moment on this PC's clock.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct End {
    pub unix: i64,
    pub local: LocalTime,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LocalTime {
    /// 0 = Sunday.
    pub weekday: u16,
    pub day: u16,
    pub month: u16,
    pub hour: u16,
    pub minute: u16,
}

/// How the countdown writes the time left. Whichever it is, the last day is orange.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum CountdownFormat {
    /// "21 days", then "2 d 14 h" in the last three days, "14 h 05 m" on the last day and "0:42:10" in the last hour.
    #[default]
    Adaptive,
    /// "In 21 days", "In 1 day", then "Today".
    Days,
    /// "20 d 14 h 05 m".
    Detailed,
    /// "20:14:05:09": days, hours, minutes, seconds.
    Clock,
}

#[derive(Debug, PartialEq)]
pub struct Countdown {
    pub big: String,
    pub small: String,
    pub urgency: Urgency,
}

/// The end from TF2's "YYYY-MM-DD", taken as the start of that day in UTC (Valve gives no time zone).
pub fn parse_end(date: &str) -> Option<End> {
    let mut parts = date.splitn(3, '-').map(str::parse::<u16>);
    let (year, month, day) = (
        parts.next()?.ok()?,
        parts.next()?.ok()?,
        parts.next()?.ok()?,
    );
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    let utc = SYSTEMTIME {
        wYear: year,
        wMonth: month,
        wDayOfWeek: 0,
        wDay: day,
        wHour: 0,
        wMinute: 0,
        wSecond: 0,
        wMilliseconds: 0,
    };
    // SAFETY: plain structs; a null time zone means the PC's own, with its daylight saving rule for that date
    let mut local: SYSTEMTIME = unsafe { std::mem::zeroed() };
    if unsafe { SystemTimeToTzSpecificLocalTime(std::ptr::null(), &utc, &mut local) } == 0 {
        return None;
    }
    Some(End {
        unix: days_from_civil(year.into(), month.into(), day.into()) * DAY,
        local: LocalTime {
            weekday: local.wDayOfWeek,
            day: local.wDay,
            month: local.wMonth,
            hour: local.wHour,
            minute: local.wMinute,
        },
    })
}

/// What the widget shows `left_s` seconds before the end; None from a day after it, when the widget hides.
pub fn text(left_s: i64, end: LocalTime, format: CountdownFormat) -> Option<Countdown> {
    let date = format!(
        "{} {}",
        end.day,
        MONTHS[usize::from(end.month.clamp(1, 12)) - 1]
    );
    let small = format!(
        "Contracts end {} {date}, {:02}:{:02}",
        WEEKDAYS[usize::from(end.weekday % 7)],
        end.hour,
        end.minute
    );
    if left_s <= -DAY {
        return None;
    }
    if left_s <= 0 {
        return Some(Countdown {
            big: "Ended".to_string(),
            small: format!("Contracts ended {date}"),
            urgency: Urgency::Ended,
        });
    }
    let s = left_s;
    let (days, hours, minutes, seconds) = (s / DAY, s % DAY / HOUR, s % HOUR / 60, s % 60);
    let big = match format {
        CountdownFormat::Adaptive if s >= 3 * DAY => format!("{days} days"),
        CountdownFormat::Adaptive if s >= DAY => format!("{days} d {hours} h"),
        CountdownFormat::Adaptive if s >= HOUR => format!("{hours} h {minutes:02} m"),
        CountdownFormat::Adaptive => format!("0:{minutes:02}:{seconds:02}"),
        CountdownFormat::Days if days == 0 => "Today".to_string(),
        CountdownFormat::Days if days == 1 => "In 1 day".to_string(),
        CountdownFormat::Days => format!("In {days} days"),
        CountdownFormat::Detailed if days == 0 => format!("{hours} h {minutes:02} m"),
        CountdownFormat::Detailed => format!("{days} d {hours} h {minutes:02} m"),
        CountdownFormat::Clock => format!("{days:02}:{hours:02}:{minutes:02}:{seconds:02}"),
    };
    let urgency = if s < DAY {
        Urgency::LastDay
    } else {
        Urgency::Calm
    };
    Some(Countdown {
        big,
        small,
        urgency,
    })
}

/// What edit mode shows when TF2's files give no end date, so the widget can still be placed.
pub fn sample(format: CountdownFormat) -> Countdown {
    let end = LocalTime {
        weekday: 0,
        day: 8,
        month: 11,
        hour: 1,
        minute: 0,
    };
    text(21 * DAY, end, format).expect("21 days ahead always has text")
}

/// Days since 1970-01-01 (Howard Hinnant's days_from_civil).
pub fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (month + 9) % 12;
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

pub const COUNTDOWN_H: f32 = 56.0;

/// Window size for the countdown panel, with room for the drop shadow.
pub fn countdown_pixels(k: f32) -> (u32, u32) {
    pixels(COUNTDOWN_H, k)
}

/// How the countdown is coloured: cream while there is time, orange on the last day, dimmed once ended.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Urgency {
    Calm,
    LastDay,
    Ended,
}

impl Painter {
    /// The Scream Fortress countdown: the time left large, the end date under it.
    #[allow(clippy::too_many_arguments)]
    pub fn countdown(
        &self,
        pm: &mut Pixmap,
        k: f32,
        opacity: f32,
        big: &str,
        small: &str,
        urgency: Urgency,
        edit: bool,
    ) {
        pm.fill(Color::TRANSPARENT);
        let t = Transform::from_scale(k, k);
        let bg = rounded_rect(0.0, 0.0, PANEL_W, COUNTDOWN_H, 4.0);
        fill(pm, &bg, PANEL_BG, opacity, t, k);
        let (big_colour, small_colour) = match urgency {
            Urgency::Calm => (CREAM, TAN_TEXT),
            Urgency::LastDay => (ORANGE, ORANGE),
            Urgency::Ended => (TAN_TEXT, TAN_TEXT),
        };
        let x = 12.0 * k;
        self.text(
            pm,
            &self.big,
            x,
            30.0 * k,
            big,
            26.0 * k,
            big_colour,
            Align::Left,
            k,
        );
        self.text(
            pm,
            &self.small,
            x,
            48.0 * k,
            small,
            14.0 * k,
            small_colour,
            Align::Left,
            k,
        );
        if edit {
            edit_outline(pm, COUNTDOWN_H, t);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const END: LocalTime = LocalTime {
        weekday: 0,
        day: 8,
        month: 11,
        hour: 1,
        minute: 0,
    };

    fn shown(left_s: i64) -> Option<(String, Urgency)> {
        text(left_s, END, CountdownFormat::Adaptive).map(|c| (c.big, c.urgency))
    }

    #[test]
    fn steps_from_days_to_a_clock() {
        let small = text(21 * DAY, END, CountdownFormat::Adaptive)
            .unwrap()
            .small;
        assert_eq!(small, "Contracts end Sun 8 Nov, 01:00");
        assert_eq!(
            shown(21 * DAY + 5 * HOUR),
            Some(("21 days".into(), Urgency::Calm))
        );
        assert_eq!(shown(3 * DAY), Some(("3 days".into(), Urgency::Calm)));
        assert_eq!(
            shown(2 * DAY + 14 * HOUR + 59),
            Some(("2 d 14 h".into(), Urgency::Calm))
        );
        assert_eq!(
            shown(14 * HOUR + 5 * 60),
            Some(("14 h 05 m".into(), Urgency::LastDay))
        );
        assert_eq!(
            shown(42 * 60 + 10),
            Some(("0:42:10".into(), Urgency::LastDay))
        );
        assert_eq!(shown(0), Some(("Ended".into(), Urgency::Ended)));
        let ended = text(-HOUR, END, CountdownFormat::Adaptive).unwrap();
        assert_eq!(ended.small, "Contracts ended 8 Nov");
        assert_eq!(shown(-DAY), None);
    }

    #[test]
    fn the_other_formats() {
        let big = |left_s: i64, format| text(left_s, END, format).map(|c| (c.big, c.urgency));
        let left = 20 * DAY + 14 * HOUR + 5 * 60 + 9;
        let calm = |s: &str| Some((s.to_string(), Urgency::Calm));
        let last_day = |s: &str| Some((s.to_string(), Urgency::LastDay));
        assert_eq!(big(left, CountdownFormat::Days), calm("In 20 days"));
        assert_eq!(big(DAY + 5, CountdownFormat::Days), calm("In 1 day"));
        assert_eq!(big(5 * HOUR, CountdownFormat::Days), last_day("Today"));
        assert_eq!(big(left, CountdownFormat::Detailed), calm("20 d 14 h 05 m"));
        assert_eq!(
            big(5 * HOUR + 60, CountdownFormat::Detailed),
            last_day("5 h 01 m")
        );
        assert_eq!(big(left, CountdownFormat::Clock), calm("20:14:05:09"));
        assert_eq!(big(65, CountdownFormat::Clock), last_day("00:00:01:05"));
        // every format ends the same way
        for format in [
            CountdownFormat::Days,
            CountdownFormat::Detailed,
            CountdownFormat::Clock,
        ] {
            assert_eq!(big(0, format).map(|(b, _)| b).as_deref(), Some("Ended"));
            assert_eq!(big(-DAY, format), None);
        }
    }

    #[test]
    fn end_date_in_unix_time() {
        assert_eq!(days_from_civil(1970, 1, 1), 0);
        assert_eq!(days_from_civil(2026, 11, 8) * DAY, 1_794_096_000);
        assert_eq!(parse_end("2026-11-08").unwrap().unix, 1_794_096_000);
        assert_eq!(parse_end("2026-13-01"), None);
        assert_eq!(parse_end("soon"), None);
    }

    /// Renders the countdown's states and writes countdown_preview.bmp to the temp folder.
    /// Run with: cargo test --release -- --ignored preview
    #[test]
    #[ignore]
    fn preview() {
        use crate::overlay::paint::preview::{painter, save_strip};
        let painter = painter();
        let k = 1.5;
        // the countdown's states: days, days and hours, the last day, the last hour, ended, edit mode
        let (cw, ch) = countdown_pixels(k);
        let tiles: Vec<Pixmap> = [
            ("21 days", Urgency::Calm, false),
            ("2 d 14 h", Urgency::Calm, false),
            ("14 h 05 m", Urgency::LastDay, false),
            ("0:42:10", Urgency::LastDay, false),
            ("Ended", Urgency::Ended, false),
            ("21 days", Urgency::Calm, true),
        ]
        .iter()
        .map(|(big, urgency, edit)| {
            let small = if *urgency == Urgency::Ended {
                "Contracts ended 8 Nov"
            } else {
                "Contracts end Sun 8 Nov, 01:00"
            };
            let mut pm = Pixmap::new(cw, ch).unwrap();
            painter.countdown(&mut pm, k, 0.78, big, small, *urgency, *edit);
            pm
        })
        .collect();
        save_strip("countdown_preview.bmp", &tiles);
        println!("tf2 fonts: {}", painter.tf2_fonts);
    }
}
