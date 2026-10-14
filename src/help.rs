//! The in-app help overlay: a centered keyboard-shortcut cheatsheet, toggled
//! with `leader ?`. Content is a pure data model so it is unit-testable; the
//! renderer lays it out into a bordered overlay.

use crate::buffer::CellBuffer;
use crate::cell::Cell;
use crate::compositor::Layer;
use crate::geometry::Point;

/// A titled group of `(keys, description)` shortcut rows.
pub struct HelpSection {
    pub title: &'static str,
    pub rows: &'static [(&'static str, &'static str)],
    /// Free-text tips shown under the rows (one line each).
    pub notes: &'static [&'static str],
}

/// All shortcut groups shown in the overlay.
pub fn help_sections() -> &'static [HelpSection] {
    &[
        HelpSection {
            title: "Windows",
            rows: &[
                ("m / n", "maximize / minimize"),
                ("Esc", "close window (asks first for apps)"),
                ("[ / ]", "snap left / right half"),
            ],
            notes: &[],
        },
        HelpSection {
            title: "Tiling",
            rows: &[
                ("t", "tile all into the grid"),
                ("T", "toggle auto-tile mode"),
                ("s", "save a screenshot (PNG)"),
                ("1–9", "send window to cell N"),
            ],
            notes: &[],
        },
        HelpSection {
            title: "Apps",
            rows: &[
                ("Space", "spotlight search"),
                ("l / Enter", "app menu (launcher)"),
                ("S / ,", "store / settings"),
                ("A", "activity monitor"),
            ],
            notes: &[],
        },
        HelpSection {
            title: "Clipboard",
            rows: &[
                ("C", "clipboard history (also the menubar \u{2398} segment)"),
                ("x / y", "select mode: the next drag selects text"),
                ("c / v", "copy selection / paste clipboard (p also pastes)"),
            ],
            notes: &[
                "Copying out of w3m, carbonyl or vim (they grab the mouse):",
                "  Ctrl+Space x, drag over the text, then Ctrl+Space C to pick",
                "  it, or Ctrl+Space v to paste it into another window.",
            ],
        },
        HelpSection {
            title: "Session",
            rows: &[
                ("q / Q", "detach / shut down"),
                ("?", "this help"),
            ],
            notes: &[],
        },
    ]
}

#[allow(clippy::vec_init_then_push)]
fn body_lines() -> Vec<(String, bool)> {
    let sections = help_sections();
    let mut lines: Vec<(String, bool)> = Vec::new(); // (text, is_heading)
    lines.push(("Leader = Ctrl+Space  (press, release, then a key)".into(), false));
    lines.push(("Remap in config.toml: leader = \"ctrl+b\" and a [keys] table".into(), false));
    lines.push(("Empty desktop: Enter or Space opens the launcher".into(), false));
    lines.push((String::new(), false));
    for s in sections {
        lines.push((s.title.to_uppercase(), true));
        for (keys, desc) in s.rows {
            lines.push((format!("  {keys:<8} {desc}"), false));
        }
        for note in s.notes {
            lines.push((format!("  {note}"), false));
        }
    }
    lines.push((String::new(), false));
    lines.push(("In overlays:  ↑↓ move · →← expand · Enter open · Esc cancel".into(), false));
    lines.push(("Menubar:  tetron-wm = launcher · ⊞/▦ = view · ✦ = assistant · host ▾ = power/Systems".into(), false));
    lines.push((String::new(), false));
    lines.push(("↑↓ PgUp PgDn Home End scroll · any other key closes".into(), false));

    lines
}

fn box_height(h: i32) -> i32 {
    (body_lines().len() as i32 + 3).min(h - 1)
}

/// Body rows that fit inside the box (between title and bottom border).
fn visible_rows(h: i32) -> usize {
    (box_height(h) - 2).max(1) as usize
}

/// Largest useful scroll offset for a screen `h` rows tall (0 when it all fits).
pub fn max_scroll(h: i32) -> usize {
    body_lines().len().saturating_sub(visible_rows(h))
}

/// Render the help overlay centered on a `w × h` screen.
pub fn render_help(w: i32, h: i32, scroll: usize) -> Vec<Layer> {
    let t = crate::theme::current();
    let lines = body_lines();
    let scroll = scroll.min(max_scroll(h));

    let inner_w = lines.iter().map(|(l, _)| l.chars().count()).max().unwrap_or(40) as i32;
    let box_w = (inner_w + 4).clamp(30, w - 2);
    let box_h = box_height(h);
    let origin = Point::new((w - box_w) / 2, ((h - box_h) / 2).max(0));

    let mut buf = CellBuffer::new(box_w, box_h);
    buf.fill(Cell { ch: ' ', fg: t.text, bg: t.window_bg, attrs: Default::default() });
    // Title bar + rounded border.
    for x in 0..box_w {
        buf.set(x, 0, Cell { ch: ' ', fg: t.title_fg, bg: t.title_focus, attrs: Default::default() });
    }
    buf.write_str(2, 0, " tetron-wm — keyboard shortcuts ", t.title_fg, t.title_focus);
    let b = |ch: char| Cell { ch, fg: t.border, bg: t.window_bg, attrs: Default::default() };
    for y in 1..box_h {
        buf.set(0, y, b('│'));
        buf.set(box_w - 1, y, b('│'));
    }
    for x in 0..box_w {
        buf.set(x, box_h - 1, b('─'));
    }
    buf.set(0, box_h - 1, b('╰'));
    buf.set(box_w - 1, box_h - 1, b('╯'));

    for (i, (text, heading)) in lines.iter().skip(scroll).enumerate() {
        let y = 1 + i as i32;
        if y >= box_h - 1 {
            break;
        }
        let fg = if *heading { t.accent } else { t.text };
        buf.write_str(2, y, text, fg, t.window_bg);
    }

    if scroll > 0 {
        buf.write_str(box_w - 4, 0, " ▲ ", t.title_fg, t.title_focus);
    }
    if scroll < max_scroll(h) {
        buf.write_str(box_w - 12, box_h - 1, " ↓ more ↓ ", t.accent, t.window_bg);
    }

    vec![Layer { z: 5500, origin, buf, opacity: 1.0, scissor: None }]
}
