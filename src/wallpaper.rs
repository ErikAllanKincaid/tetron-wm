//! PROTOTYPE image wallpaper via chafa (cell-art). Experimental / throwaway:
//! gated behind the `TETRON_WM_WALLPAPER=<image>` env var so it changes nothing
//! unless asked. Converts an image to a w×h grid of colored cells (works on any
//! terminal incl. a kmscon console, since the output is cells, not graphics).
//!
//! See docs/IDEAS_tetron-wm_configurable_wallpaper.md for the real design.

use crate::buffer::CellBuffer;
use crate::cell::{Cell, Rgba};

/// Build a `w`×`h` cell wallpaper from `path` using chafa's symbol output,
/// stretched to fill exactly. Returns `None` if chafa is missing/fails.
pub fn from_image(path: &str, w: i32, h: i32) -> Option<CellBuffer> {
    if w <= 0 || h <= 0 {
        return None;
    }
    let out = std::process::Command::new("chafa")
        .args([
            "-f", "symbols",
            "-c", "full",
            // `all-wide`: every symbol EXCEPT the 2-column-wide ones. A wide glyph
            // (e.g. a CJK char chafa picks) would occupy two cells but our grid is
            // strictly one glyph per column, so it shifts the row and leaves black
            // gaps. Excluding wide keeps exactly w×h single-column cells.
            "--symbols", "all-wide",
            "--dither", "ordered", // break up color banding in smooth gradients
            "--stretch",
            "-s", &format!("{w}x{h}"),
            path,
        ])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    Some(parse_ansi(&String::from_utf8_lossy(&out.stdout), w, h))
}

/// Parse chafa's truecolor-SGR symbol output into a `w`×`h` [`CellBuffer`].
fn parse_ansi(s: &str, w: i32, h: i32) -> CellBuffer {
    let def = Rgba::rgb(0, 0, 0);
    let mut buf = CellBuffer::new(w, h);
    buf.fill(Cell { ch: ' ', fg: def, bg: def, attrs: Default::default() });
    let (mut fg, mut bg) = (def, def);
    let (mut x, mut y) = (0i32, 0i32);
    let mut it = s.chars().peekable();
    while let Some(c) = it.next() {
        match c {
            '\x1b' => {
                if it.peek() == Some(&'[') {
                    it.next();
                    let mut params = String::new();
                    let mut final_byte = ' ';
                    while let Some(&p) = it.peek() {
                        it.next();
                        if p.is_ascii_alphabetic() {
                            final_byte = p;
                            break;
                        }
                        params.push(p);
                    }
                    if final_byte == 'm' {
                        apply_sgr(&params, &mut fg, &mut bg, def);
                    }
                }
            }
            '\n' => {
                y += 1;
                x = 0;
            }
            '\r' => x = 0,
            _ => {
                if x < w && y < h {
                    buf.set(x, y, Cell { ch: c, fg, bg, attrs: Default::default() });
                }
                x += 1;
            }
        }
        if y >= h {
            break;
        }
    }
    buf
}

/// Apply one SGR parameter string, updating `fg`/`bg` (truecolor `38;2;r;g;b` /
/// `48;2;r;g;b`, plus reset/default). Non-color attributes are ignored.
fn apply_sgr(params: &str, fg: &mut Rgba, bg: &mut Rgba, def: Rgba) {
    let toks: Vec<&str> = params.split(';').collect();
    let num = |t: Option<&&str>| t.and_then(|s| s.parse::<u8>().ok()).unwrap_or(0);
    let mut i = 0;
    while i < toks.len() {
        match toks[i] {
            "" | "0" => {
                *fg = def;
                *bg = def;
                i += 1;
            }
            "39" => {
                *fg = def;
                i += 1;
            }
            "49" => {
                *bg = def;
                i += 1;
            }
            "38" if toks.get(i + 1) == Some(&"2") => {
                *fg = Rgba::rgb(num(toks.get(i + 2)), num(toks.get(i + 3)), num(toks.get(i + 4)));
                i += 5;
            }
            "48" if toks.get(i + 1) == Some(&"2") => {
                *bg = Rgba::rgb(num(toks.get(i + 2)), num(toks.get(i + 3)), num(toks.get(i + 4)));
                i += 5;
            }
            _ => i += 1,
        }
    }
}
