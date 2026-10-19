//! The screensaver: an on-demand full-screen effect drawn over everything (no idle
//! timer; any key or mouse event from the attached client wakes it).
//!
//! An effect is a small object that paints one frame of cells. To add an
//! animation, implement [`Effect`] and add a line to [`EFFECTS`] / [`make`]; the
//! session, the `tetron-wm screensaver` command, the config key and the menus
//! all pick it up by name. Effects draw ONLY with background-coloured cells (no
//! special glyphs), so they look the same in any console font.
//!
//! Honest limit: this blanks the cells, not the monitor (see the TODO on real
//! display blanking), and it is not a lock.

use crate::buffer::CellBuffer;
use crate::cell::{Cell, Rgba};
use std::time::Duration;

/// What a `Screensaver` message asks for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SaverMode {
    On,
    Off,
    Toggle,
}

/// Local wall-clock time, read once per frame (no subprocess, no chrono).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LocalTime {
    pub hour: u32,
    pub min: u32,
    pub sec: u32,
    pub year: i32,
    pub month: u32,
    pub day: u32,
    /// Days since Sunday, 0..=6.
    pub wday: u32,
}

/// The local time now, via `localtime_r` (honours `TZ`).
pub fn local_time() -> LocalTime {
    // SAFETY: `time` and `localtime_r` are given valid pointers to stack values.
    unsafe {
        let t = libc::time(std::ptr::null_mut());
        let mut tm: libc::tm = std::mem::zeroed();
        libc::localtime_r(&t, &mut tm);
        LocalTime {
            hour: tm.tm_hour.clamp(0, 23) as u32,
            min: tm.tm_min.clamp(0, 59) as u32,
            sec: tm.tm_sec.clamp(0, 60) as u32,
            year: tm.tm_year + 1900,
            month: (tm.tm_mon + 1).clamp(1, 12) as u32,
            day: tm.tm_mday.clamp(1, 31) as u32,
            wday: tm.tm_wday.clamp(0, 6) as u32,
        }
    }
}

/// What an effect is given for each frame.
#[derive(Clone, Copy, Debug)]
pub struct Ctx {
    /// Time since the saver started (drives motion, so frames are reproducible).
    pub elapsed: Duration,
    /// The wall clock, for time-based effects.
    pub now: LocalTime,
}

/// One screensaver effect.
pub trait Effect: Send {
    /// Registry / config name.
    fn name(&self) -> &'static str;
    /// Frames per second to repaint at; 0 = static (painted once per size).
    fn fps(&self) -> u32;
    /// Paint one frame, `w` × `h` cells.
    fn frame(&mut self, ctx: &Ctx, w: i32, h: i32) -> CellBuffer;
}

/// `(name, description)` of every effect, in menu order.
pub const EFFECTS: &[(&str, &str)] = &[
    ("black", "plain black screen (no animation, no cost)"),
    ("clock", "big animated clock drifting around the screen (spares burn-in)"),
];

/// The default effect.
pub const DEFAULT_EFFECT: &str = "black";

/// Build the effect called `name` (case-insensitive), if there is one.
pub fn make(name: &str) -> Option<Box<dyn Effect>> {
    match name.trim().to_ascii_lowercase().as_str() {
        "black" => Some(Box::new(Black)),
        "clock" => Some(Box::new(Clock)),
        _ => None,
    }
}

const BLACK: Rgba = Rgba { r: 0, g: 0, b: 0, a: 255 };

fn blank(w: i32, h: i32) -> CellBuffer {
    let mut buf = CellBuffer::new(w.max(1), h.max(1));
    buf.fill(Cell { ch: ' ', fg: BLACK, bg: BLACK, attrs: Default::default() });
    buf
}

/// Plain black.
pub struct Black;

impl Effect for Black {
    fn name(&self) -> &'static str {
        "black"
    }
    fn fps(&self) -> u32 {
        0
    }
    fn frame(&mut self, _ctx: &Ctx, w: i32, h: i32) -> CellBuffer {
        blank(w, h)
    }
}

// ── The clock ────────────────────────────────────────────────────────────────

/// 3 × 5 bitmaps of the digits, one string per row ('1' = lit).
const DIGITS: [[&str; 5]; 10] = [
    ["111", "101", "101", "101", "111"], // 0
    ["010", "110", "010", "010", "111"], // 1
    ["111", "001", "111", "100", "111"], // 2
    ["111", "001", "111", "001", "111"], // 3
    ["101", "101", "111", "001", "001"], // 4
    ["111", "100", "111", "001", "111"], // 5
    ["111", "100", "111", "101", "111"], // 6
    ["111", "001", "001", "001", "001"], // 7
    ["111", "101", "111", "101", "111"], // 8
    ["111", "101", "111", "001", "111"], // 9
];

fn digit_rows(d: u32) -> [&'static str; 5] {
    DIGITS[(d % 10) as usize]
}

/// Cells per lit pixel horizontally: 2 makes a square-ish pixel in a ~1:2 cell.
const PX_W: i32 = 2;
/// Width of one big digit in cells (3 pixels).
const DIGIT_W: i32 = 3 * PX_W;
/// Big digit height in rows.
const DIGIT_H: i32 = 5;
/// Gap between glyphs in cells.
const GAP: i32 = 1;
/// Width of the big colon in cells (1 pixel).
const COLON_W: i32 = PX_W;

/// Width of the big `HH:MM:SS` block: 6 digits, 2 colons, 7 gaps.
pub const BIG_W: i32 = 6 * DIGIT_W + 2 * COLON_W + 7 * GAP;
/// Height of the big block plus a blank row and the date line.
pub const BIG_H: i32 = DIGIT_H + 2;

const WEEKDAYS: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
const MONTHS: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

/// `HH:MM:SS`.
pub fn time_text(t: &LocalTime) -> String {
    format!("{:02}:{:02}:{:02}", t.hour, t.min, t.sec.min(59))
}

/// `Sun 18 Oct`.
pub fn date_text(t: &LocalTime) -> String {
    format!("{} {:02} {}", WEEKDAYS[(t.wday % 7) as usize], t.day, MONTHS[((t.month.max(1) - 1) % 12) as usize])
}

/// A triangle wave: moves 0 → `range` → 0 → … at `speed` units per second, so a
/// drifting thing bounces off the edges. Pure, so it can be tested.
pub fn bounce(elapsed: Duration, speed_per_sec: f64, range: i32) -> i32 {
    if range <= 0 {
        return 0;
    }
    let period = 2.0 * range as f64;
    let p = (elapsed.as_secs_f64() * speed_per_sec) % period;
    let pos = if p <= range as f64 { p } else { period - p };
    (pos.floor() as i32).clamp(0, range)
}

/// A big `HH:MM:SS` clock (with the date under it) that drifts slowly around the
/// screen so no pixel stays lit, with a blinking colon. Falls back to a one-line
/// clock on a small screen.
pub struct Clock;

impl Clock {
    fn lit(buf: &mut CellBuffer, x: i32, y: i32, w: i32, color: Rgba) {
        for dx in 0..w {
            buf.set(x + dx, y, Cell { ch: ' ', fg: color, bg: color, attrs: Default::default() });
        }
    }

    fn big_glyph(buf: &mut CellBuffer, x: i32, y: i32, rows: [&str; 5], color: Rgba) {
        for (ry, row) in rows.iter().enumerate() {
            for (rx, c) in row.chars().enumerate() {
                if c == '1' {
                    Self::lit(buf, x + rx as i32 * PX_W, y + ry as i32, PX_W, color);
                }
            }
        }
    }

    fn text(buf: &mut CellBuffer, x: i32, y: i32, s: &str, fg: Rgba) {
        buf.write_str(x, y, s, fg, BLACK);
    }
}

impl Effect for Clock {
    fn name(&self) -> &'static str {
        "clock"
    }
    fn fps(&self) -> u32 {
        2 // the seconds and the colon blink change once a second
    }
    fn frame(&mut self, ctx: &Ctx, w: i32, h: i32) -> CellBuffer {
        let mut buf = blank(w, h);
        let color = crate::theme::current().accent;
        let dim = crate::theme::current().dim;
        let date = date_text(&ctx.now);
        let t = &ctx.now;

        if w >= BIG_W + 2 && h >= BIG_H + 2 {
            // Drift: 1 cell per second sideways, 1 row per 3 seconds down.
            let x0 = bounce(ctx.elapsed, 1.0, w - BIG_W);
            let y0 = bounce(ctx.elapsed, 1.0 / 3.0, h - BIG_H);
            let colon_on = t.sec.is_multiple_of(2);
            let digits = [t.hour / 10, t.hour % 10, t.min / 10, t.min % 10, t.sec.min(59) / 10, t.sec.min(59) % 10];
            let mut x = x0;
            for (i, d) in digits.iter().enumerate() {
                Self::big_glyph(&mut buf, x, y0, digit_rows(*d), color);
                x += DIGIT_W + GAP;
                if i == 1 || i == 3 {
                    if colon_on {
                        Self::lit(&mut buf, x, y0 + 1, COLON_W, color);
                        Self::lit(&mut buf, x, y0 + 3, COLON_W, color);
                    }
                    x += COLON_W + GAP;
                }
            }
            // The date, centred under the digits.
            let dx = x0 + (BIG_W - date.chars().count() as i32) / 2;
            Self::text(&mut buf, dx, y0 + DIGIT_H + 1, &date, dim);
        } else {
            // Small screen: one line, still drifting.
            let text = time_text(t);
            let tw = text.chars().count() as i32;
            let x0 = bounce(ctx.elapsed, 1.0, (w - tw).max(0));
            let y0 = bounce(ctx.elapsed, 1.0 / 3.0, (h - 2).max(0));
            Self::text(&mut buf, x0, y0, &text, color);
            if h >= 2 && w >= date.chars().count() as i32 {
                let dx = x0 + (tw - date.chars().count() as i32) / 2;
                Self::text(&mut buf, dx.max(0), y0 + 1, &date, dim);
            }
        }
        buf
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(h: u32, m: u32, s: u32) -> LocalTime {
        LocalTime { hour: h, min: m, sec: s, year: 2026, month: 10, day: 18, wday: 0 }
    }

    fn ctx(secs: u64, t: LocalTime) -> Ctx {
        Ctx { elapsed: Duration::from_secs(secs), now: t }
    }

    fn lit_cells(buf: &CellBuffer) -> Vec<(i32, i32)> {
        let mut v = Vec::new();
        for y in 0..buf.height() {
            for x in 0..buf.width() {
                let c = buf.get(x, y).unwrap();
                if c.bg != BLACK || c.ch != ' ' {
                    v.push((x, y));
                }
            }
        }
        v
    }

    #[test]
    fn registry_lists_every_effect_and_builds_it() {
        assert!(EFFECTS.len() >= 2);
        for (name, desc) in EFFECTS {
            assert!(!desc.is_empty());
            let e = make(name).unwrap_or_else(|| panic!("{name} is listed but not built"));
            assert_eq!(e.name(), *name);
        }
        assert!(make("CLOCK").is_some(), "names are case-insensitive");
        assert!(make("no-such-effect").is_none());
        assert!(make(DEFAULT_EFFECT).is_some());
    }

    #[test]
    fn black_is_static_and_all_black() {
        let mut b = Black;
        assert_eq!(b.fps(), 0);
        let f = b.frame(&ctx(0, at(1, 2, 3)), 40, 10);
        assert_eq!((f.width(), f.height()), (40, 10));
        assert!(lit_cells(&f).is_empty());
    }

    #[test]
    fn digit_bitmaps_are_well_formed() {
        for d in 0..10 {
            let rows = digit_rows(d);
            assert_eq!(rows.len(), 5);
            assert!(rows.iter().all(|r| r.len() == 3 && r.chars().all(|c| c == '0' || c == '1')), "digit {d}");
        }
        // Spot checks: 1 is narrow, 8 is the full block with a middle bar, 7 hangs off the top.
        assert_eq!(digit_rows(1)[0], "010");
        assert_eq!(digit_rows(8)[2], "111");
        assert_eq!(digit_rows(7), ["111", "001", "001", "001", "001"]);
        assert_ne!(digit_rows(7), digit_rows(3));
    }

    #[test]
    fn bounce_stays_in_range_and_turns_around() {
        let range = 10;
        let mut last = bounce(Duration::from_secs(0), 1.0, range);
        assert_eq!(last, 0);
        let mut max = 0;
        for s in 0..60 {
            let p = bounce(Duration::from_secs(s), 1.0, range);
            assert!((0..=range).contains(&p), "t={s} p={p}");
            assert!((p - last).abs() <= 1, "moves at most one cell per second");
            max = max.max(p);
            last = p;
        }
        assert_eq!(max, range, "it reaches the far edge, then comes back");
        assert_eq!(bounce(Duration::from_secs(20), 1.0, range), 0, "full period returns to the start");
        assert_eq!(bounce(Duration::from_secs(5), 1.0, 0), 0, "no room to move");
    }

    #[test]
    fn clock_draws_inside_the_screen_and_changes_with_time_and_position() {
        let mut c = Clock;
        let (w, h) = (100, 30);
        let a = c.frame(&ctx(0, at(12, 34, 56)), w, h);
        let cells = lit_cells(&a);
        assert!(!cells.is_empty());
        assert!(cells.iter().all(|(x, y)| *x >= 0 && *x < w && *y >= 0 && *y < h));
        // A different time paints different digits.
        let b = c.frame(&ctx(0, at(21, 43, 8)), w, h);
        assert_ne!(lit_cells(&a), lit_cells(&b));
        // It drifts: the same time later is somewhere else.
        let later = c.frame(&ctx(10, at(12, 34, 56)), w, h);
        assert_ne!(lit_cells(&a), lit_cells(&later), "the clock moves to avoid burn-in");
        // The block fits its advertised size.
        let xs: Vec<i32> = cells.iter().map(|c| c.0).collect();
        assert!(xs.iter().max().unwrap() - xs.iter().min().unwrap() < BIG_W);
    }

    #[test]
    fn the_colon_blinks_each_second() {
        let mut c = Clock;
        // At elapsed 0 the block sits at x = 0, y = 0; the first colon follows two
        // digits and their gaps, its dots on rows 1 and 3.
        let colon_x = 2 * (DIGIT_W + GAP);
        let lit_at = |b: &CellBuffer, x: i32, y: i32| b.get(x, y).unwrap().bg != BLACK;
        let even = c.frame(&ctx(0, at(0, 0, 0)), 100, 30);
        let odd = c.frame(&ctx(0, at(0, 0, 1)), 100, 30);
        assert!(lit_at(&even, colon_x, 1) && lit_at(&even, colon_x, 3), "colon visible on even seconds");
        assert!(!lit_at(&odd, colon_x, 1) && !lit_at(&odd, colon_x, 3), "colon hidden on odd seconds");
        assert!(!lit_at(&even, colon_x, 2), "only the two dots, not a bar");
    }

    #[test]
    fn small_screens_get_a_one_line_clock() {
        let mut c = Clock;
        for (w, h) in [(30, 3), (20, 2), (12, 1), (80, 8), (40, 24)] {
            let f = c.frame(&ctx(5, at(9, 8, 7)), w, h);
            assert_eq!((f.width(), f.height()), (w, h));
            let text: String = (0..f.height()).flat_map(|y| (0..f.width()).map(move |x| (x, y))).map(|(x, y)| f.get(x, y).unwrap().ch).collect();
            assert!(text.contains("09:08:07") || w < 8, "{w}x{h}: {text:?}");
        }
        // A 100x30 screen takes the big layout, which has no time text at all.
        let big = c.frame(&ctx(5, at(9, 8, 7)), 100, 30);
        let text: String = (0..big.height()).flat_map(|y| (0..big.width()).map(move |x| (x, y))).map(|(x, y)| big.get(x, y).unwrap().ch).collect();
        assert!(!text.contains("09:08:07"));
        assert!(text.contains("Sun 18 Oct"), "the date is under the digits");
    }

    #[test]
    fn local_time_is_sane() {
        let t = local_time();
        assert!(t.hour < 24 && t.min < 60 && t.sec <= 60 && (1..=12).contains(&t.month) && (1..=31).contains(&t.day));
        assert!(t.year >= 2024);
        assert_eq!(time_text(&at(3, 4, 5)), "03:04:05");
        assert_eq!(date_text(&at(0, 0, 0)), "Sun 18 Oct");
    }
}
