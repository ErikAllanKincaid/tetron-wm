//! PROTOTYPE image wallpaper via chafa (cell-art). Experimental / throwaway:
//! gated behind the `TETRON_WM_WALLPAPER=<image>` env var so it changes nothing
//! unless asked. Converts an image to a w×h grid of colored cells (works on any
//! terminal incl. a kmscon console, since the output is cells, not graphics).
//!
//! See docs/IDEAS_tetron-wm_configurable_wallpaper.md for the real design.

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
        "-c", "full",
        // `all-wide`: every symbol EXCEPT the 2-column-wide ones. A wide glyph
        // (e.g. a CJK char chafa picks) would occupy two cells but our grid is
        // strictly one glyph per column, so it shifts the row and leaves black
        // gaps. Excluding wide keeps exactly w×h single-column cells.
        "--symbols", "all-wide",
        "--dither", "ordered", // break up color banding in smooth gradients
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

#[cfg(test)]
mod tests {
    use super::*;

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
