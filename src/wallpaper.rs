//! PROTOTYPE image wallpaper via chafa (cell-art). Experimental / throwaway:
//! gated behind the `TETRON_WM_WALLPAPER=<image>` env var so it changes nothing
//! unless asked. Converts an image to a w×h grid of colored cells (works on any
//! terminal incl. a kmscon console, since the output is cells, not graphics).

use crate::buffer::CellBuffer;
use crate::cell::{Cell, Rgba};
use std::path::PathBuf;

/// The built-in default wallpaper (426x240 JPEG, 70 KB). It is drawn when the
/// config says `default` (or nothing), and stands in for a configured file
/// that is missing, so the desktop never goes blank by accident. To ship a new
/// default, replace `assets/wallpaper.jpg` (keep it under 100 KB, a test checks).
const DEFAULT_JPG: &[u8] = include_bytes!("../assets/wallpaper.jpg");

/// Config value that means "the built-in wallpaper". An empty value means the
/// same; Settings writes `default` back so the name is always visible.
pub const DEFAULT_NAME: &str = "default";

/// The config default before `default` existed. Configs still carrying it get
/// the built-in image without a "not found" note when no such file exists.
const LEGACY_NAME: &str = "wallpaper.jpg";

/// Does config value `spec` ask for the built-in wallpaper (unset, empty, or
/// the `default` keyword)?
pub fn is_default(spec: Option<&str>) -> bool {
    spec.map(clean_spec).is_none_or(|s| s.is_empty() || s == DEFAULT_NAME)
}

/// What a wallpaper value asks for, before any file is looked at.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    /// An image: the built-in, a bundled one, or a file.
    Image,
    /// `none`: no wallpaper, the plain theme `desktop_bg`.
    Plain,
    /// `black` or `#rrggbb`: a flat colour, drawn without chafa.
    Solid(Rgba),
}

/// Classify wallpaper value `spec` (see [`Source`]). Only `none`, `black` and
/// a `#rrggbb` colour are special; anything else, including an unset value, is
/// an image (and a malformed `#…` stays one, so the "not found" note shows).
pub fn source(spec: Option<&str>) -> Source {
    let Some(spec) = spec else { return Source::Image };
    let s = clean_spec(spec);
    if s.eq_ignore_ascii_case("none") {
        return Source::Plain;
    }
    if s.eq_ignore_ascii_case("black") {
        return Source::Solid(Rgba::rgb(0, 0, 0));
    }
    match s.strip_prefix('#') {
        Some(hex) if hex.len() == 6 => crate::badge::parse_color(&s).map_or(Source::Image, |c| Source::Solid(Rgba { a: 255, ..c })),
        _ => Source::Image,
    }
}

/// What to put behind the icons for `spec` at colour `depth`, before any file
/// is read. Below 256 colours the built-in image turns to noise, so `default`
/// is the theme's own desktop colour there; a file the user chose is drawn at
/// any depth.
pub fn draw_for(spec: Option<&str>, depth: crate::display::ColorDepth) -> Option<Source> {
    match source(spec) {
        Source::Plain => None,
        s @ Source::Solid(_) => Some(s),
        Source::Image if is_default(spec) && depth < crate::display::ColorDepth::Ansi256 => None,
        Source::Image => Some(Source::Image),
    }
}

/// A `w`×`h` buffer of one background colour.
pub fn solid(color: Rgba, w: i32, h: i32) -> CellBuffer {
    let mut buf = CellBuffer::new(w.max(0), h.max(0));
    buf.fill(Cell { ch: ' ', fg: color, bg: color, attrs: Default::default() });
    buf
}

/// Normalise a wallpaper value the way people paste it: surrounding quotes
/// (`"/a b.jpg"`, `'/a b.jpg'`, from a shell or a file manager's "copy
/// path"), a `file://` URL (percent-decoded, so `%20` becomes a space), and a
/// leading `$HOME` / `${HOME}` (becomes `~`, which [`resolve_path`] expands).
pub fn clean_spec(spec: &str) -> String {
    let mut s = spec.trim();
    for q in ['"', '\''] {
        if s.len() >= 2 && s.starts_with(q) && s.ends_with(q) {
            s = s[1..s.len() - 1].trim();
        }
    }
    if let Some(rest) = s.strip_prefix("file://") {
        // `file://localhost/x` and `file:///x` both name `/x`.
        let rest = rest.strip_prefix("localhost").unwrap_or(rest);
        return percent_decode(rest);
    }
    for var in ["$HOME", "${HOME}"] {
        if let Some(rest) = s.strip_prefix(var) {
            if rest.is_empty() || rest.starts_with('/') {
                return format!("~{rest}");
            }
        }
    }
    s.to_string()
}

/// Decode `%XX` escapes (as bytes, so multi-byte UTF-8 survives); a malformed
/// escape is kept as typed.
fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let hex = |c: u8| (c as char).to_digit(16);
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            if let (Some(h), Some(l)) = (hex(b[i + 1]), hex(b[i + 2])) {
                out.push((h * 16 + l) as u8);
                i += 3;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// What to draw for a config value: the image file, plus the configured file
/// when it was missing (the built-in image stands in; the caller logs it).
pub struct Choice {
    /// File to hand to chafa. `None` only if the built-in could not be written.
    pub path: Option<PathBuf>,
    /// The configured file that does not exist, when that is why `path` is
    /// the built-in image.
    pub missing: Option<PathBuf>,
}

/// Pick the file to draw for config value `spec` (see [`Choice`]).
pub fn choose(spec: Option<&str>) -> Choice {
    if is_default(spec) {
        return Choice { path: ensure_default(), missing: None };
    }
    let spec = spec.unwrap_or_default();
    let p = resolve_path(spec);
    if p.exists() {
        return Choice { path: Some(p), missing: None };
    }
    let legacy = clean_spec(spec) == LEGACY_NAME;
    Choice { path: ensure_default(), missing: (!legacy).then_some(p) }
}

/// A one-line problem with config value `spec` for Settings to show under the
/// wallpaper rows, or `None` when it will draw as asked. Only looks at the
/// disk and `$PATH`; never writes.
pub fn note(spec: Option<&str>) -> Option<String> {
    if source(spec) != Source::Image {
        return None; // a flat colour or no wallpaper needs neither chafa nor a file
    }
    if !chafa_installed() {
        return Some("chafa is not installed, so the desktop is plain.".into());
    }
    if is_default(spec) {
        return None;
    }
    let spec = spec.unwrap_or_default();
    let p = resolve_path(spec);
    if p.exists() || clean_spec(spec) == LEGACY_NAME {
        return None;
    }
    Some(format!("Not found, showing the built-in: {}", p.display()))
}

/// Is `chafa` (the wallpaper renderer) on `$PATH`?
fn chafa_installed() -> bool {
    std::env::var_os("PATH")
        .map(|p| std::env::split_paths(&p).any(|d| d.join("chafa").is_file()))
        .unwrap_or(false)
}

/// Write the embedded default wallpaper to `<data>/tetron-wm/wallpaper.jpg`
/// (`$XDG_DATA_HOME` aware) unless the file there already holds exactly these
/// bytes, and return its path. `chafa` needs a real file, hence the stamp.
/// Comparing contents (not size) means a new default always replaces the old
/// copy. The copy older versions wrote (`default-wallpaper.jpg`) is removed.
pub fn ensure_default() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| dirs::home_dir().map(|h| h.join(".local/share")))?;
    let dir = base.join("tetron-wm");
    let path = dir.join("wallpaper.jpg");
    let current = std::fs::read(&path).map(|b| b == DEFAULT_JPG).unwrap_or(false);
    if !current {
        std::fs::create_dir_all(&dir).ok()?;
        std::fs::write(&path, DEFAULT_JPG).ok()?;
        let _ = std::fs::remove_file(dir.join("default-wallpaper.jpg"));
        crate::dbg_log(&format!("wallpaper: stamped built-in default to {}", path.display()));
    }
    Some(path)
}

/// Resolve a configured wallpaper path to a file on disk (after [`clean_spec`]).
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
    let cleaned = clean_spec(path);
    let path = cleaned.as_str();
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

/// Render `path` into exactly `w`×`h` cells for the image viewer, with the same
/// symbols and palette as the wallpaper. The caller already fitted the aspect
/// ratio, hence `--stretch`. Small palettes are dithered, which chafa only
/// does when asked and which keeps photos readable on a VT. `None` when chafa
/// is missing or fails.
pub fn viewer_cells(path: &std::path::Path, w: i32, h: i32) -> Option<CellBuffer> {
    if w <= 0 || h <= 0 {
        return None;
    }
    let size = format!("{w}x{h}");
    let colors = color_mode(crate::display::get());
    let path = path.to_string_lossy();
    let mut args = vec!["-f", "symbols", "-c", colors, "--symbols", symbol_set(), "--stretch", "-s", &size];
    if matches!(colors, "2" | "8" | "16") {
        args.extend(["--dither", "diffusion"]);
    }
    args.push(&path);
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
    // chafa writes a cell of solid foreground as inverse video plus a space,
    // typically at the start of a row; ignoring it left black notches there.
    let mut inverse = false;
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
                        apply_sgr(&params, &mut fg, &mut bg, &mut inverse, def);
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
                    let (fg, bg) = if inverse { (bg, fg) } else { (fg, bg) };
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
/// `40-47/100-107`, plus reset/default) and inverse (`7` / `27`). Other
/// attributes are ignored.
fn apply_sgr(params: &str, fg: &mut Rgba, bg: &mut Rgba, inverse: &mut bool, def: Rgba) {
    let toks: Vec<&str> = params.split(';').collect();
    let num = |t: Option<&&str>| t.and_then(|s| s.parse::<u8>().ok()).unwrap_or(0);
    let mut i = 0;
    while i < toks.len() {
        match toks[i] {
            "" | "0" => {
                *fg = def;
                *bg = def;
                *inverse = false;
                i += 1;
            }
            "7" => {
                *inverse = true;
                i += 1;
            }
            "27" => {
                *inverse = false;
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
    fn source_recognises_none_black_and_hex() {
        assert_eq!(source(None), Source::Image);
        assert_eq!(source(Some("default")), Source::Image);
        assert_eq!(source(Some("forest.jpg")), Source::Image);
        assert_eq!(source(Some("none")), Source::Plain);
        assert_eq!(source(Some(" NONE ")), Source::Plain);
        assert_eq!(source(Some("black")), Source::Solid(Rgba::rgb(0, 0, 0)));
        assert_eq!(source(Some("\"#004040\"")), Source::Solid(Rgba::rgb(0, 0x40, 0x40)));
        assert_eq!(source(Some("#0A0b0C")), Source::Solid(Rgba::rgb(10, 11, 12)));
        // Malformed colours stay images, so the "not found" note shows them.
        assert_eq!(source(Some("#12345")), Source::Image);
        assert_eq!(source(Some("#12345678")), Source::Image);
        assert_eq!(source(Some("#gggggg")), Source::Image);
    }

    #[test]
    fn default_is_the_theme_colour_below_256_colours_but_a_chosen_file_is_drawn() {
        use crate::display::ColorDepth::*;
        for d in [Mono, Ansi16] {
            assert_eq!(draw_for(Some("default"), d), None, "{d:?}");
            assert_eq!(draw_for(None, d), None, "{d:?}");
            assert_eq!(draw_for(Some("forest.jpg"), d), Some(Source::Image), "{d:?}");
            assert_eq!(draw_for(Some("black"), d), Some(Source::Solid(Rgba::rgb(0, 0, 0))), "{d:?}");
        }
        for d in [Ansi256, TrueColor] {
            assert_eq!(draw_for(Some("default"), d), Some(Source::Image), "{d:?}");
        }
        assert_eq!(draw_for(Some("none"), TrueColor), None);
    }

    #[test]
    fn solid_fills_every_cell_and_needs_no_chafa() {
        let c = Rgba::rgb(1, 2, 3);
        let buf = solid(c, 4, 2);
        assert_eq!((buf.get(0, 0).unwrap().bg, buf.get(3, 1).unwrap().bg), (c, c));
        assert_eq!(note(Some("black")), None);
        assert_eq!(note(Some("#102030")), None);
        assert_eq!(note(Some("none")), None);
    }

    #[test]
    fn sgr_parses_16_and_256_colour_forms() {
        let (mut fg, mut bg) = (Rgba::rgb(1, 1, 1), Rgba::rgb(1, 1, 1));
        let def = Rgba::rgb(0, 0, 0);
        let mut inv = false;
        apply_sgr("0;94;41", &mut fg, &mut bg, &mut inv, def);
        assert_eq!((fg, bg), (crate::display::VGA16[12], crate::display::VGA16[1]));
        apply_sgr("38;5;196;48;5;232", &mut fg, &mut bg, &mut inv, def);
        assert_eq!((fg, bg), (Rgba::rgb(255, 0, 0), Rgba::rgb(8, 8, 8)));
    }

    /// chafa's solid cell: inverse video plus a space, as it writes at a row
    /// start. It must come out in the foreground colour, not black.
    #[test]
    fn inverse_space_is_a_solid_foreground_cell() {
        let out = "\x1b[7m\x1b[38;2;6;9;50m \x1b[0m\x1b[38;2;1;2;3;48;2;4;5;6m\u{2580}";
        let buf = parse_ansi(out, 2, 1);
        assert_eq!(buf.get(0, 0).unwrap().bg, Rgba::rgb(6, 9, 50), "inverse space shows the fg colour");
        let c = buf.get(1, 0).unwrap();
        assert_eq!((c.fg, c.bg), (Rgba::rgb(1, 2, 3), Rgba::rgb(4, 5, 6)), "reset ends inverse");
    }

    #[test]
    fn embedded_default_is_a_small_jpeg() {
        assert_eq!(&DEFAULT_JPG[..2], &[0xFF, 0xD8], "JPEG magic");
        assert!(DEFAULT_JPG.len() < 100 * 1024, "keep the embedded wallpaper small");
    }

    #[test]
    fn clean_spec_handles_pasted_forms() {
        assert_eq!(clean_spec("  \"/a b.jpg\" "), "/a b.jpg");
        assert_eq!(clean_spec("'/a b.jpg'"), "/a b.jpg");
        assert_eq!(clean_spec("file:///home/u/My%20Pics/%C3%A9.png"), "/home/u/My Pics/\u{e9}.png");
        assert_eq!(clean_spec("file://localhost/x.png"), "/x.png");
        assert_eq!(clean_spec("$HOME/p.png"), "~/p.png");
        assert_eq!(clean_spec("${HOME}/p.png"), "~/p.png");
        assert_eq!(clean_spec("$HOMEDIR/p.png"), "$HOMEDIR/p.png", "only the whole variable");
        assert_eq!(clean_spec("100%.png"), "100%.png");
    }

    #[test]
    fn default_keyword_empty_and_unset_mean_builtin() {
        for v in [None, Some(""), Some("  "), Some("default"), Some("\"default\"")] {
            assert!(is_default(v), "{v:?}");
        }
        assert!(!is_default(Some("wallpaper.jpg")));
        assert!(!is_default(Some("/x/default")));
    }

    #[test]
    fn missing_file_reports_missing_but_legacy_name_does_not() {
        let c = choose(Some("/no/such/dir/pic.png"));
        assert_eq!(c.missing, Some(PathBuf::from("/no/such/dir/pic.png")));
        // Old configs naming the former default never report it missing.
        assert!(choose(Some("wallpaper.jpg")).missing.is_none());
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
