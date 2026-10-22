use crate::cell::Rgba;
use crate::compositor::CellChange;
use crate::display::{ClientCaps, ColorDepth, VGA16};
use std::io::{Write, Stdout};

/// Terminal capability flags detected from environment variables.
///
/// Used by [`frame_to_ansi`] to choose the right SGR color encoding,
/// and by [`Terminal`] to enable pixel-accurate mouse if the terminal supports it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Caps {
    /// How many colours to emit: mono (attributes only), 16, 256 or truecolor.
    pub depth: ColorDepth,
    /// A bare kernel VT (`TERM=linux`): 8 background colours, PSF fonts.
    pub vt: bool,
    /// Whether the terminal supports SGR 1016 pixel-position mouse events (Kitty / WezTerm / Ghostty).
    pub pixel_mouse: bool,
    /// Whether the terminal supports the Kitty graphics protocol (for images).
    pub kitty_graphics: bool,
}

impl Caps {
    /// Detect capabilities from the environment. Uses `TERM_PROGRAM` (set locally)
    /// **and** `TERM` — the latter is forwarded over SSH (e.g. `xterm-ghostty`,
    /// `xterm-kitty`, `wezterm`), so graphics still work in a remote session where
    /// `TERM_PROGRAM`/`COLORTERM` are stripped.
    pub fn detect() -> Caps {
        let ct = std::env::var("COLORTERM").unwrap_or_default();
        let term_program = std::env::var("TERM_PROGRAM").unwrap_or_default();
        let term = std::env::var("TERM").unwrap_or_default();
        let known = matches!(term_program.as_str(), "kitty" | "WezTerm" | "ghostty")
            || term.contains("ghostty")
            || term.contains("kitty")
            || term.contains("wezterm");
        // Manual override: `TETRON_WM_GRAPHICS=1` forces graphics on when detection
        // can't see the terminal (e.g. SSH with TERM stripped to xterm-256color).
        let force = std::env::var("TETRON_WM_GRAPHICS").map(|v| v != "0").unwrap_or(false);
        let graphics = known || force;
        // Known terminals are all truecolor; otherwise trust COLORTERM, or detect
        // kmscon (truecolor on a bare console, but TERM=xterm-256color + no
        // COLORTERM, so env detection alone misses it).
        let auto_truecolor =
            graphics || ct.contains("truecolor") || ct.contains("24bit") || under_kmscon();
        let depth = detect_depth(
            std::env::var("TETRON_WM_COLORS").ok().as_deref(),
            std::env::var_os("NO_COLOR").is_some_and(|v| !v.is_empty()),
            &term,
            truecolor_override().unwrap_or(auto_truecolor),
        );
        Caps { depth, vt: term == "linux", pixel_mouse: graphics, kitty_graphics: graphics }
    }

    /// Whether 24-bit SGR is emitted (kept for call sites that only care about that).
    pub fn truecolor(&self) -> bool {
        self.depth == ColorDepth::TrueColor
    }

    /// What the daemon needs to know about this display (sent on attach).
    pub fn client_caps(&self) -> ClientCaps {
        ClientCaps { colors: self.depth, vt: self.vt, kitty_graphics: self.kitty_graphics }
    }
}

/// Resolve the colour depth from its signals, strongest first:
///
/// 1. `TETRON_WM_COLORS` (`mono|16|256|truecolor`; `auto` or junk falls through).
/// 2. `NO_COLOR` (non-empty, per no-color.org) means mono.
/// 3. `TERM`: `dumb`/`vt100`-class terminals are mono; the kernel VT (`linux`) is 16.
/// 4. Otherwise truecolor if detected or forced (`TETRON_WM_TRUECOLOR`/config), else 256.
///
/// Pure (the caller reads the environment) so every branch is unit-tested.
pub fn detect_depth(colors: Option<&str>, no_color: bool, term: &str, truecolor: bool) -> ColorDepth {
    if let Some(d) = colors.and_then(ColorDepth::parse) {
        return d;
    }
    if no_color {
        return ColorDepth::Mono;
    }
    if matches!(term, "dumb" | "vt52" | "vt100" | "vt102" | "vt220" | "vt320" | "xterm-mono") || term.ends_with("-m") {
        return ColorDepth::Mono;
    }
    if term == "linux" {
        return ColorDepth::Ansi16;
    }
    if truecolor { ColorDepth::TrueColor } else { ColorDepth::Ansi256 }
}

/// Whether an ancestor process is `kmscon`. kmscon renders truecolor (and, on
/// the maintained line, mouse) on a bare console, but sets `TERM=xterm-256color`
/// with no `COLORTERM`, so env-based detection misses it. The session's shell
/// and this client run under the kmscon process, so walk `/proc` up the parent
/// chain and look for it. Linux-only and cheap (the chain is a few hops:
/// client ← login ← kmscon); bounded so a cycle can never spin.
#[cfg(target_os = "linux")]
fn under_kmscon() -> bool {
    let mut pid = std::process::id();
    for _ in 0..16 {
        if pid <= 1 {
            break;
        }
        if std::fs::read_to_string(format!("/proc/{pid}/comm"))
            .map(|c| c.trim() == "kmscon")
            .unwrap_or(false)
        {
            return true;
        }
        let ppid = std::fs::read_to_string(format!("/proc/{pid}/status"))
            .ok()
            .and_then(|s| {
                s.lines()
                    .find_map(|l| l.strip_prefix("PPid:"))
                    .and_then(|v| v.trim().parse::<u32>().ok())
            });
        match ppid {
            Some(p) => pid = p,
            None => break,
        }
    }
    false
}
#[cfg(not(target_os = "linux"))]
fn under_kmscon() -> bool {
    false
}

/// Explicit truecolor choice: the `TETRON_WM_TRUECOLOR` env var (`1/true/yes/on`
/// vs `0/false/no/off`) takes priority over the `truecolor` config key. `None`
/// means "auto-detect".
pub(crate) fn truecolor_override() -> Option<bool> {
    if let Some(v) = std::env::var_os("TETRON_WM_TRUECOLOR") {
        let v = v.to_string_lossy().trim().to_ascii_lowercase();
        if !v.is_empty() {
            return Some(matches!(v.as_str(), "1" | "true" | "yes" | "on"));
        }
    }
    crate::config::Config::load().truecolor
}

// ── Pure color helpers ────────────────────────────────────────────────────────

/// Below this WCAG contrast ratio a 16-colour fg/bg pair is unreadable (the
/// quantizer can map distinct theme colours onto the same or adjacent VGA
/// entries), so the fg is replaced by black or bright white. 2.0 keeps the
/// usual dark-grey-on-black "dim" text (2.8) and catches grey-on-grey (1.0).
const MIN_CONTRAST_16: f64 = 2.0;

/// A background at least this bright reads as "highlighted" in mono mode and is
/// drawn as reverse video (selection, focused title, an app's status bar).
const MONO_REVERSE_LUM: f64 = 0.2;

/// Push the colour SGR parameters (and, for mono, the attributes that stand in
/// for colour) for one cell onto `sgr`.
fn push_colors(fg: Rgba, bg: Rgba, caps: &Caps, sgr: &mut Vec<String>) {
    match caps.depth {
        ColorDepth::TrueColor => {
            // On a bare VT the kernel reduces this to 16 colours itself.
            let fg = if caps.vt {
                kernel_readable_fg(fg, fg, bg)
            } else {
                fg
            };
            sgr.push(format!("38;2;{};{};{}", fg.r, fg.g, fg.b));
            sgr.push(format!("48;2;{};{};{}", bg.r, bg.g, bg.b));
        }
        ColorDepth::Ansi256 => {
            let (mut fi, mut bi) = (ansi256(fg), ansi256(bg));
            if caps.vt {
                // The kernel decodes cube indices at about half brightness, so
                // mid colours lose their hue (teal #008080 -> index 30 -> black
                // background). When the index would show differently from the
                // colour itself, send the basic index (0-15) the kernel reads
                // as exactly that colour: still a valid 256-colour code.
                if kernel_bg(kernel_rgb_256(bi)) != kernel_bg(bg) {
                    bi = kernel_bg(bg);
                }
                if kernel_fg(kernel_rgb_256(fi)) != kernel_fg(fg) {
                    fi = kernel_fg(fg);
                }
                // Then guard on the colours the kernel will derive from them.
                let g = kernel_readable_fg(fg, kernel_rgb_256(fi), kernel_rgb_256(bi));
                if g != fg {
                    fi = ansi256(g);
                }
            }
            sgr.push(format!("38;5;{fi}"));
            sgr.push(format!("48;5;{bi}"));
        }
        ColorDepth::Ansi16 => {
            // A bare VT has only the dark half for backgrounds (SGR 40-47);
            // terminals forced to 16 colours also take 100-107.
            let bi = ansi16(bg, if caps.vt { 8 } else { 16 });
            let fi = readable_fg16(ansi16(fg, 16), bi);
            sgr.push(format!("{}", if fi < 8 { 30 + fi } else { 90 + fi - 8 }));
            sgr.push(format!("{}", if bi < 8 { 40 + bi } else { 100 + bi - 8 }));
        }
        ColorDepth::Mono => {
            // No colour at all: brightness becomes attributes. A light
            // background is a highlight (reverse video); on normal cells a dark
            // fg is secondary text (dim) and a near-white fg is emphasis (bold).
            if crate::theme::luminance(bg) >= MONO_REVERSE_LUM {
                sgr.push("7".into());
            } else {
                let l = crate::theme::luminance(fg);
                if l < 0.3 {
                    sgr.push("2".into());
                } else if l > 0.9 {
                    sgr.push("1".into());
                }
            }
        }
    }
}

// ── The kernel VT's own reduction ─────────────────────────────────────────────
//
// A bare Linux console accepts 256-colour and truecolor SGR (honoured when the
// user forces `TETRON_WM_COLORS=256|truecolor`) but shows only 16 colours, 8 as
// backgrounds. It reduces them with a crude rule (`drivers/tty/vt/vt.c`:
// `rgb_from_256`, `rgb_foreground`, `rgb_background`), which can land text and
// background on the same colour (the `windows` theme's grey dim text on white
// became grey on grey). These mirror that rule so the contrast guard can run on
// what the kernel will actually show.

/// The RGB the kernel derives from xterm-256 index `i`.
fn kernel_rgb_256(i: u8) -> Rgba {
    let i = i as u32;
    let v = |on: bool, hi: u32, lo: u32| if on { hi } else { lo } as u8;
    match i {
        0..=7 => Rgba::rgb(v(i & 1 != 0, 0xaa, 0), v(i & 2 != 0, 0xaa, 0), v(i & 4 != 0, 0xaa, 0)),
        8..=15 => Rgba::rgb(v(i & 1 != 0, 0xff, 0x55), v(i & 2 != 0, 0xff, 0x55), v(i & 4 != 0, 0xff, 0x55)),
        16..=231 => {
            let n = i - 16;
            Rgba::rgb((n / 36 * 85 / 2) as u8, (n / 6 % 6 * 85 / 2) as u8, (n % 6 * 85 / 2) as u8)
        }
        _ => {
            let g = (i * 10 - 2312) as u8;
            Rgba::rgb(g, g, g)
        }
    }
}

/// The 16-colour index (ANSI order) the kernel shows for foreground `c`: a
/// channel is on above half the brightest channel; bright above 0xaa; a
/// dim grey becomes bright black.
fn kernel_fg(c: Rgba) -> u8 {
    let max = c.r.max(c.g).max(c.b);
    let hue = (c.r > max / 2) as u8 | ((c.g > max / 2) as u8) << 1 | ((c.b > max / 2) as u8) << 2;
    if hue == 7 && max <= 0x55 {
        8
    } else if max > 0xaa {
        hue + 8
    } else {
        hue
    }
}

/// The background index (0-7, ANSI order) the kernel shows for `c`: each
/// channel on at 128 or above ("errs on the dark side").
fn kernel_bg(c: Rgba) -> u8 {
    (c.r >= 0x80) as u8 | ((c.g >= 0x80) as u8) << 1 | ((c.b >= 0x80) as u8) << 2
}

/// `fg` unless the kernel's reduction of (`kfg`, `kbg`), the RGB it derives
/// from what we send, is unreadable; then black or white.
fn kernel_readable_fg(fg: Rgba, kfg: Rgba, kbg: Rgba) -> Rgba {
    let (fi, bi) = (kernel_fg(kfg), kernel_bg(kbg));
    match readable_fg16(fi, bi) {
        f if f == fi => fg,
        0 => Rgba::rgb(0, 0, 0),
        _ => Rgba::rgb(255, 255, 255),
    }
}

/// The VGA colour of palette index `i`.
fn vga(i: u8) -> Rgba {
    VGA16[i as usize]
}

/// Contrast guard for 16 colours: keep `fi` if it reads on `bi`, else switch to
/// whichever of black (0) or bright white (15) contrasts more.
fn readable_fg16(fi: u8, bi: u8) -> u8 {
    let bg = vga(bi);
    if crate::theme::contrast(vga(fi), bg) >= MIN_CONTRAST_16 {
        return fi;
    }
    if crate::theme::contrast(vga(0), bg) >= crate::theme::contrast(vga(15), bg) { 0 } else { 15 }
}

/// Nearest VGA palette index among the first `n` entries (8 for backgrounds, which
/// can not use the bright half on a VT; 16 for foregrounds).
fn ansi16(c: Rgba, n: usize) -> u8 {
    let (r, g, b) = (c.r as i32, c.g as i32, c.b as i32);
    (0..n)
        .min_by_key(|&i| {
            let p = VGA16[i];
            let (pr, pg, pb) = (p.r as i32, p.g as i32, p.b as i32);
            (r - pr).pow(2) + (g - pg).pow(2) + (b - pb).pow(2)
        })
        .unwrap_or(0) as u8
}

/// Map an [`Rgba`] color to the nearest xterm 256-color index.
///
/// Near-grey colors use the 24-step grayscale ramp (232–255), which has far
/// finer dark-grey resolution than the 6×6×6 color cube — without this, dark
/// greys collapse to black. Other colors use the cube (indices 16–231).
fn ansi256(c: Rgba) -> u8 {
    let (r, g, b) = (c.r as i32, c.g as i32, c.b as i32);
    if (r - g).abs() < 12 && (g - b).abs() < 12 && (r - b).abs() < 12 {
        let grey = (r + g + b) / 3;
        if grey < 4 {
            return 16; // pure black
        }
        if grey > 246 {
            return 231; // pure white
        }
        // Grayscale ramp index n (0..=23) has value 8 + 10*n.
        return (232 + ((grey - 8).clamp(0, 238) / 10)) as u8;
    }
    let q = |v: u8| -> u32 { v as u32 * 5 / 255 };
    (16 + 36 * q(c.r) + 6 * q(c.g) + q(c.b)) as u8
}

// ── Pure ANSI frame writer ────────────────────────────────────────────────────

/// Convert a list of [`CellChange`]s into an ANSI byte string ready for `stdout`.
///
/// Emits a CUP (`\x1b[row;colH`) + SGR for each changed cell, followed by a final
/// SGR reset. Row/column positions are 1-based as required by the VT100 standard.
///
/// This function is **pure** — it has no I/O side effects and can be unit-tested
/// without a real terminal.
///
/// Returns an empty string when `changes` is empty.
pub fn frame_to_ansi(changes: &[CellChange], caps: &Caps) -> String {
    if changes.is_empty() {
        return String::new();
    }
    let mut s = String::new();
    for ch in changes {
        // Continuation cell (right half of a double-width glyph): the glyph to its
        // left already covers this column, so skip it — painting it would erase the
        // glyph's right half.
        if ch.cell.ch == '\0' {
            continue;
        }
        // CUP: move cursor to 1-based (row, col).
        s.push_str(&format!("\x1b[{};{}H", ch.y + 1, ch.x + 1));
        let a = &ch.cell.attrs;
        let mut sgr = Vec::with_capacity(4);
        // Reverse video is applied here, by swapping the colours before they
        // are encoded, rather than sent as SGR 7: a kernel VT can not show a
        // bright background, so reversing bright text there gives a black
        // background (an invisible cursor or selection), and in mono SGR 7
        // would cancel the reverse that stands in for a light background.
        let (fg, bg) = if a.inverse { (ch.cell.bg, ch.cell.fg) } else { (ch.cell.fg, ch.cell.bg) };
        push_colors(fg, bg, caps, &mut sgr);
        if a.bold      { sgr.push("1".into()); }
        if a.italic    { sgr.push("3".into()); }
        if a.underline { sgr.push("4".into()); }
        if sgr.is_empty() {
            s.push_str("\x1b[0m");
        } else {
            s.push_str(&format!("\x1b[0;{}m", sgr.join(";")));
        }
        s.push(ch.cell.ch);
    }
    s.push_str("\x1b[0m");
    s
}

/// Ask the terminal whether it supports the Kitty graphics protocol, by sending a
/// graphics query immediately followed by a Device-Attributes query as a sentinel.
/// Graphics-capable terminals reply `ESC _ G … ;OK ESC \` before the DA reply
/// (`ESC [ … c`); others just answer DA. Reads the reply with a short timeout, so
/// it never hangs. Must run in raw mode, before any other stdin reader starts.
fn probe_graphics(out: &mut Stdout) -> bool {
    use std::io::{Read, Write};
    // a=q queries support without displaying; t=d/f=24 = a 1×1 RGB payload.
    let _ = out.write_all(b"\x1b_Gi=4294967290,s=1,v=1,a=q,t=d,f=24;AAAA\x1b\\\x1b[c");
    let _ = out.flush();

    let start = std::time::Instant::now();
    let budget = std::time::Duration::from_millis(300);
    let mut buf: Vec<u8> = Vec::new();
    let mut tmp = [0u8; 256];
    let has = |hay: &[u8], n: &[u8]| hay.windows(n.len()).any(|w| w == n);
    loop {
        let elapsed = start.elapsed();
        if elapsed >= budget {
            break;
        }
        let remaining_ms = (budget - elapsed).as_millis() as libc::c_int;
        let mut pfd = libc::pollfd { fd: libc::STDIN_FILENO, events: libc::POLLIN, revents: 0 };
        // SAFETY: a single valid pollfd on stdin with a finite timeout.
        let ready = unsafe { libc::poll(&mut pfd, 1, remaining_ms) };
        if ready <= 0 {
            break; // timeout or error
        }
        match std::io::stdin().lock().read(&mut tmp) {
            Ok(0) | Err(_) => break,
            Ok(n) => buf.extend_from_slice(&tmp[..n]),
        }
        // Stop once the DA reply (ends in 'c') has arrived.
        if has(&buf, b"\x1b[") && buf.contains(&b'c') {
            break;
        }
    }
    has(&buf, b"\x1b_G") && has(&buf, b"OK")
}

// ── Terminal lifecycle ────────────────────────────────────────────────────────

/// Owns the raw-mode / alternate-screen / mouse-capture lifecycle.
///
/// Call [`Terminal::enter`] to initialise the terminal; the [`Drop`] implementation
/// tears it down automatically when the value goes out of scope.
///
/// # Note
/// The lifecycle code is not unit-tested (it requires a real PTY); it is exercised
/// through the main render loop. The pure rendering path ([`frame_to_ansi`]) is
/// fully unit-tested separately.
pub struct Terminal {
    out: Stdout,
    /// Detected terminal capabilities (truecolor, pixel mouse).
    pub caps: Caps,
}

impl Terminal {
    /// Enter raw mode, switch to the alternate screen, enable mouse capture, and hide
    /// the cursor. Returns an initialised [`Terminal`] on success.
    pub fn enter() -> std::io::Result<Terminal> {
        use crossterm::{
            terminal,
            execute,
            event::EnableMouseCapture,
            cursor,
        };
        terminal::enable_raw_mode()?;
        let mut out = std::io::stdout();
        execute!(out, terminal::EnterAlternateScreen, EnableMouseCapture, cursor::Hide)?;
        use std::io::Write;
        write!(out, "\x1b[?1003h")?; // all-motion mouse tracking (for launcher hover)
        write!(out, "\x1b_Ga=d,d=A\x1b\\")?; // clear any orphaned graphics from a prior session
        out.flush()?;
        // Resolve graphics support: env detection first (cheap), else ask the
        // terminal directly — this is the only reliable signal over SSH, where
        // TERM_PROGRAM/COLORTERM are stripped.
        let mut caps = Caps::detect();
        if !caps.kitty_graphics && probe_graphics(&mut out) {
            caps.kitty_graphics = true;
            // A graphics terminal is truecolor, unless the depth was chosen
            // explicitly (TETRON_WM_COLORS / NO_COLOR / TETRON_WM_TRUECOLOR).
            let explicit = std::env::var("TETRON_WM_COLORS").ok().and_then(|v| ColorDepth::parse(&v)).is_some();
            if caps.depth == ColorDepth::Ansi256 && !explicit && truecolor_override().is_none() {
                caps.depth = ColorDepth::TrueColor;
            }
        }
        crate::dbg_log(&format!(
            "caps: kitty_graphics={} colors={} vt={} term={} term_program={}",
            caps.kitty_graphics,
            caps.depth.name(),
            caps.vt,
            std::env::var("TERM").unwrap_or_default(),
            std::env::var("TERM_PROGRAM").unwrap_or_default(),
        ));
        Ok(Terminal { out, caps })
    }

    /// Return the current terminal size as `(columns, rows)`.
    pub fn size() -> std::io::Result<(i32, i32)> {
        let (c, r) = crossterm::terminal::size()?;
        Ok((c as i32, r as i32))
    }

    /// Encode `changes` as ANSI and flush them to stdout.
    pub fn write_frame(&mut self, changes: &[CellChange]) -> std::io::Result<()> {
        let s = frame_to_ansi(changes, &self.caps);
        self.out.write_all(s.as_bytes())?;
        self.out.flush()
    }
}

impl Drop for Terminal {
    /// Restore the terminal: disable mouse capture, leave the alternate screen, show
    /// the cursor, and disable raw mode. Errors are silently discarded (best-effort
    /// teardown at shutdown).
    fn drop(&mut self) {
        use crossterm::{
            terminal,
            execute,
            event::DisableMouseCapture,
            cursor,
        };
        use std::io::Write;
        let _ = write!(self.out, "\x1b[?1003l"); // disable all-motion mouse tracking
        let _ = execute!(
            self.out,
            DisableMouseCapture,
            terminal::LeaveAlternateScreen,
            cursor::Show
        );
        let _ = terminal::disable_raw_mode();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truecolor_env_override_wins_and_parses() {
        // Serialized within one test so the process-wide env var is deterministic.
        std::env::set_var("TETRON_WM_TRUECOLOR", "1");
        assert_eq!(truecolor_override(), Some(true));
        std::env::set_var("TETRON_WM_TRUECOLOR", "ON");
        assert_eq!(truecolor_override(), Some(true));
        std::env::set_var("TETRON_WM_TRUECOLOR", "0");
        assert_eq!(truecolor_override(), Some(false));
        std::env::set_var("TETRON_WM_TRUECOLOR", "false");
        assert_eq!(truecolor_override(), Some(false));
        std::env::remove_var("TETRON_WM_TRUECOLOR");
    }
}
