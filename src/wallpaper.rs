//! PROTOTYPE image wallpaper via chafa (cell-art). Experimental / throwaway:
//! gated behind the `TETRON_WM_WALLPAPER=<image>` env var so it changes nothing
//! unless asked. Converts an image to a w×h grid of colored cells (works on any
//! terminal incl. a kmscon console, since the output is cells, not graphics).
//!
//! The design notes (IDEAS_tetron-wm_configurable_wallpaper.md) moved out of
//! the repo into DO-NOT-COMMIT/ with the other internal plans.

use crate::buffer::CellBuffer;
use crate::cell::{Cell, Rgba};
use std::path::PathBuf;

/// Resolve a configured wallpaper path to a file on disk.
///
/// - An absolute path (`/…`) is used as-is.
/// - A `~`-path is expanded against the home directory.
/// - A relative path is resolved against the config dir
///   (`~/.config/tetron-wm/`), trying a `wallpapers/` subdir first, then the
///   config-dir root. It is **never** resolved against the process cwd (the
///   daemon's cwd is unpredictable — a service, a reload, launched from
///   anywhere), so a bare filename always means "next to your config".
///
/// Returns the first candidate that exists, or the best-effort candidate when
/// none exist (so the caller's own "missing file" handling / log still fires).
/// The built-in default wallpaper (426x240 JPEG, 70 KB), used when the default
/// `wallpaper.jpg` is not on disk so a fresh install has a wallpaper with no setup.
const DEFAULT_JPG: &[u8] = include_bytes!("../assets/default-wallpaper.jpg");

/// Config value that means "the default wallpaper".
pub const DEFAULT_NAME: &str = "wallpaper.jpg";

/// Write the embedded default wallpaper to `<data>/tetron-wm/default-wallpaper.jpg`
/// (`$XDG_DATA_HOME` aware) if it is missing or a different size, and return its
/// path. `chafa` needs a real file, hence the stamp.
pub fn ensure_default() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| dirs::home_dir().map(|h| h.join(".local/share")))?;
    let dir = base.join("tetron-wm");
    let path = dir.join("default-wallpaper.jpg");
    let current = std::fs::metadata(&path).map(|m| m.len() as usize == DEFAULT_JPG.len()).unwrap_or(false);
    if !current {
        std::fs::create_dir_all(&dir).ok()?;
        std::fs::write(&path, DEFAULT_JPG).ok()?;
        crate::dbg_log(&format!("wallpaper: stamped built-in default to {}", path.display()));
    }
    Some(path)
}

pub fn resolve_path(path: &str) -> PathBuf {
    let path = path.trim();
    if let Some(rest) = path.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(rest);
        }
    }
    let p = PathBuf::from(path);
    if p.is_absolute() {
        return p;
    }
    if let Some(cfg) = crate::config::config_dir() {
        let sub = cfg.join("wallpapers").join(path);
        if sub.exists() {
            return sub;
        }
        let root = cfg.join(path);
        if root.exists() {
            return root;
        }
        return sub; // best-effort (lets the caller log the expected location)
    }
    p
}

/// chafa symbol classes the console font can draw. `ascii` (the default style)
/// must never emit a glyph a console font lacks: kernel VT fonts have only the
/// space, full block and the ░ ▒ shades (no half blocks at all), while DejaVu
/// (kmscon) has every half block and quadrant.
/// Sextants and braille are absent from those fonts, and as kmscon fallbacks they
/// are wider than a cell, so only the GUI styles (`nerd`/`emoji`) use `all-wide`.
/// Whether the client is a bare VT comes from its reported caps (`display`),
/// not the daemon's own `TERM`, which may belong to a different terminal.
fn symbol_set() -> &'static str {
    use crate::iconset::IconStyle;
    match crate::iconset::style() {
        IconStyle::Ascii if crate::display::get().vt => "space+solid+0x2591..0x2592",
        // DejaVu Sans Mono (kmscon) has every half block and quadrant.
        IconStyle::Ascii | IconStyle::Unicode | IconStyle::Auto => "half+quad+space+solid",
        _ => "all-wide",
    }
}

/// chafa's colour mode for the client's display. With a small palette chafa
/// picks symbols and colours together (far better art than quantizing a
/// truecolor render afterwards); a bare VT has only 8 background colours.
/// Mono never gets here (`SessionCore::build_wallpaper` skips the wallpaper).
fn color_mode(caps: crate::display::ClientCaps) -> &'static str {
    use crate::display::ColorDepth;
    match caps.colors {
        ColorDepth::Mono => "2",
        ColorDepth::Ansi16 if caps.vt => "8",
        ColorDepth::Ansi16 => "16",
        ColorDepth::Ansi256 => "240",
        ColorDepth::TrueColor => "full",
    }
}

/// Build a `w`×`h` cell wallpaper from `path` using chafa's symbol output,
/// stretched to fill exactly. Returns `None` if chafa is missing/fails.
///
/// The chafa call is **bounded** (stdin closed, hard timeout). This runs on the
/// daemon's input/render loop (via `SessionCore::build_wallpaper`), so a chafa
/// that ever blocked — an inherited stdin, a slow or malformed file, a wedged
/// process — would otherwise freeze the whole UI until restart.
pub fn from_image(path: &str, w: i32, h: i32) -> Option<CellBuffer> {
    if w <= 0 || h <= 0 {
        return None;
    }
    let size = format!("{w}x{h}");
    let args = [
        "-f", "symbols",
        "-c", color_mode(crate::display::get()),
        // `all-wide`: every symbol EXCEPT the 2-column-wide ones. A wide glyph
        // (e.g. a CJK char chafa picks) would occupy two cells but our grid is
        // strictly one glyph per column, so it shifts the row and leaves black
        // gaps. Excluding wide keeps exactly w×h single-column cells.
        "--symbols", symbol_set(),
        "--stretch",
        "-s", &size,
        path,
    ];
    let out = run_chafa_capped(&args, std::time::Duration::from_secs(8))?;
    Some(parse_ansi(&String::from_utf8_lossy(&out), w, h))
}

/// Run chafa with stdin closed and a hard timeout, draining stdout on a thread
/// so a full pipe can never deadlock the child. Returns its stdout on success,
/// or `None` if chafa is missing, fails, or outruns `timeout` (then killed).
fn run_chafa_capped(args: &[&str], timeout: std::time::Duration) -> Option<Vec<u8>> {
    use std::io::Read;
    use std::process::{Command, Stdio};
    use std::time::Instant;
    let mut child = Command::new("chafa")
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    // Drain stdout concurrently: chafa's full-screen output is far larger than
    // the OS pipe buffer, so reading only after exit could deadlock.
    let mut stdout = child.stdout.take()?;
    let reader = std::thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = stdout.read_to_end(&mut buf);
        buf
    });
    let deadline = Instant::now() + timeout;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let buf = reader.join().unwrap_or_default();
                return if status.success() { Some(buf) } else { None };
            }
            Ok(None) => {
                if Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    let _ = reader.join();
                    crate::dbg_log("wallpaper: chafa timed out — killed (solid fallback)");
                    return None;
                }
                std::thread::sleep(std::time::Duration::from_millis(15));
            }
            Err(_) => {
                let _ = reader.join();
                return None;
            }
        }
    }
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
/// `48;2;r;g;b`, 256-colour `38;5;n`, the 16 ANSI colours `30-37/90-97` and
/// `40-47/100-107`, plus reset/default). Non-color attributes are ignored.
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
            "38" if toks.get(i + 1) == Some(&"5") => {
                *fg = xterm256(num(toks.get(i + 2)));
                i += 3;
            }
            "48" if toks.get(i + 1) == Some(&"5") => {
                *bg = xterm256(num(toks.get(i + 2)));
                i += 3;
            }
            t => {
                // The 16 ANSI colours, from chafa's `-c 8` / `-c 16` modes.
                match t.parse::<u8>() {
                    Ok(n @ 30..=37) => *fg = crate::display::VGA16[(n - 30) as usize],
                    Ok(n @ 90..=97) => *fg = crate::display::VGA16[(n - 90 + 8) as usize],
                    Ok(n @ 40..=47) => *bg = crate::display::VGA16[(n - 40) as usize],
                    Ok(n @ 100..=107) => *bg = crate::display::VGA16[(n - 100 + 8) as usize],
                    _ => {}
                }
                i += 1;
            }
        }
    }
}

/// The RGB of an xterm 256-colour index (16 system colours, 6x6x6 cube, grey ramp).
fn xterm256(n: u8) -> Rgba {
    match n {
        0..=15 => crate::display::VGA16[n as usize],
        16..=231 => {
            let i = n - 16;
            let v = |q: u8| if q == 0 { 0 } else { 55 + 40 * q };
            Rgba::rgb(v(i / 36), v((i / 6) % 6), v(i % 6))
        }
        _ => {
            let g = 8 + 10 * (n - 232);
            Rgba::rgb(g, g, g)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sgr_parses_16_and_256_colour_forms() {
        let (mut fg, mut bg) = (Rgba::rgb(1, 1, 1), Rgba::rgb(1, 1, 1));
        let def = Rgba::rgb(0, 0, 0);
        apply_sgr("0;94;41", &mut fg, &mut bg, def);
        assert_eq!((fg, bg), (crate::display::VGA16[12], crate::display::VGA16[1]));
        apply_sgr("38;5;196;48;5;232", &mut fg, &mut bg, def);
        assert_eq!((fg, bg), (Rgba::rgb(255, 0, 0), Rgba::rgb(8, 8, 8)));
    }

    #[test]
    fn embedded_default_is_a_small_jpeg() {
        assert_eq!(&DEFAULT_JPG[..2], &[0xFF, 0xD8], "JPEG magic");
        assert!(DEFAULT_JPG.len() < 100 * 1024, "keep the embedded wallpaper small");
    }

    #[test]
    fn absolute_path_passes_through() {
        assert_eq!(resolve_path("/tmp/wall.png"), PathBuf::from("/tmp/wall.png"));
        // whitespace is trimmed
        assert_eq!(resolve_path("  /tmp/wall.png  "), PathBuf::from("/tmp/wall.png"));
    }

    #[test]
    fn tilde_expands_to_home() {
        if let Some(home) = dirs::home_dir() {
            assert_eq!(resolve_path("~/pic.png"), home.join("pic.png"));
        }
    }

    #[test]
    fn relative_resolves_under_config_dir() {
        // A bare filename resolves beneath the config dir, never the cwd.
        if let Some(cfg) = crate::config::config_dir() {
            let got = resolve_path("pic.png");
            assert!(got.starts_with(&cfg), "{got:?} should be under {cfg:?}");
            assert!(got.ends_with("pic.png"));
        }
    }
}
