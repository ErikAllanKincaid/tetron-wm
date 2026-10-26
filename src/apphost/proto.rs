//! Wire protocol between the frontend (`RemoteAppHost`) and the apphost server.
//! Newline-delimited JSON, mirroring `crate::protocol`. Image bytes ride as
//! base64 strings; grids ride as serialized `CellBuffer`s.

use crate::buffer::CellBuffer;
use crate::kittygfx::Placement;
use serde::{Deserialize, Serialize};
use std::io::{BufRead, Write};

/// frontend → apphost.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum HostReq {
    Spawn { req_id: u64, cmd: String, args: Vec<String>, cwd: Option<String>, cols: i32, rows: i32 },
    Input { app: u64, bytes: Vec<u8> },
    Resize { app: u64, cols: i32, rows: i32 },
    /// Scroll the app's scrollback view by `lines` (+ = back into history).
    Scroll { app: u64, lines: i32 },
    SetMeta { app: u64, meta: Vec<u8> },
    Kill { app: u64 },
    /// Ask the apphost to enumerate its hosted apps. The apphost replies with
    /// `HostEvt::AppList`. Used by `tetron-wm ps` / `tetron-wm kill-app`.
    ListApps,
    /// Stop the apphost process entirely (full shutdown / `tetron-wm kill`).
    Shutdown,
    /// Re-apply the desktop theme and the `terminal_bg`/`terminal_fg` overrides
    /// in the running apphost (protocol v4+). Running apps repaint with the new
    /// default colors on the next frame. `None` means "follow the theme"; the
    /// daemon always sends resolved colors, since the apphost may not know a
    /// theme file that was added after it started.
    ///
    /// `ansi` (v6+) is the theme's 16 ANSI colours as hex, in palette order; an
    /// older frontend omits it and an older apphost ignores it.
    SetColors {
        theme: String,
        terminal_bg: Option<String>,
        terminal_fg: Option<String>,
        #[serde(default)]
        ansi: Option<Vec<String>>,
    },
    /// The frontend's [`PROTO_VERSION`], sent right after the `Roster` (protocol
    /// v5+). It tells the apphost which `Frame` encodings this frontend reads:
    /// from v5 the apphost sends [`PackedGrid`]s instead of JSON cell grids.
    /// Sent only to apphosts reporting proto >= 5 (an older one would log and
    /// skip the unknown variant); a frontend that never says gets JSON grids.
    Hello { proto: u32 },
}

/// An app asked to READ a clipboard through OSC 52 (`ESC ] 52 ; c ; ? ST`).
/// The frontend answers it by writing an OSC 52 reply to the app's input.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClipReq {
    /// `c` (clipboard) or `p` (primary selection).
    pub kind: char,
    /// The app terminated its query with BEL (else ST); the reply must match.
    pub bel: bool,
}

/// A PNG the app transmitted, base64-encoded, sent once per (frontend, app, id).
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ImgBlob {
    pub image_id: u32,
    pub png_b64: String,
}

/// The apphost wire-protocol version this binary speaks. Bump on ANY change
/// to `HostReq`/`HostEvt`.
///
/// v2 — added an optional `pid` field to `HostEvt::Spawned` so the daemon
/// (via `RemoteAppHost`) can populate the activity monitor's `pid` column.
/// `#[serde(default)]` keeps it backward-compatible: a v1 apphost omits the
/// field and the v2 frontend fills `None`; a v1 frontend ignores the unknown
/// field on a v2 wire payload (serde default is permissive). Do NOT bump
/// `MIN_COMPAT`.
///
/// v4 — added `HostReq::SetColors` so the daemon can recolor running apps live
/// (theme switch, terminal_bg/terminal_fg) instead of needing an apphost
/// restart. A new request variant is unknown to older apphosts, so the frontend
/// sends it only when the apphost reports proto >= 4 (`RemoteAppHost`). Do NOT
/// bump `MIN_COMPAT`: an older apphost just keeps its start-up colors.
///
/// v5 — added `HostReq::Hello` and `HostEvt::Frame::packed`: once a frontend
/// says it speaks v5, the apphost sends each changed grid as a [`PackedGrid`]
/// (run-length binary, changed rows only) and leaves `grid` empty. JSON cell
/// grids cost the daemon about half its CPU under heavy output. Both sides
/// stay on JSON grids unless both are v5+, so no `MIN_COMPAT` bump.
///
/// v6 — `HostReq::SetColors` gained an optional `ansi` list (the theme's 16 ANSI
/// colours). `#[serde(default)]` both ways: an older apphost ignores the field
/// and keeps its palette, an older frontend omits it. Do NOT bump `MIN_COMPAT`.
pub const PROTO_VERSION: u32 = 6;

/// v3 — added `clip_reqs` to `HostEvt::Frame`: OSC 52 clipboard READ queries
/// from the app, so the frontend can answer them. `#[serde(default)]`; an old
/// apphost simply never reports a query. Do NOT bump `MIN_COMPAT`.
///
/// The OLDEST apphost protocol this frontend can safely talk to. Apphosts
/// predating the `proto` roster field report 0. Bump this to `PROTO_VERSION`
/// ONLY when a change genuinely breaks older apphosts — doing so arms the
/// post-update safety dialog ("restart the app server, closes your apps"),
/// giving users a chance to save work instead of silent breakage.
pub const MIN_COMPAT: u32 = 0;

/// One app's metadata in the on-connect roster.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RosterEntry {
    pub app: u64,
    pub meta: Vec<u8>,
}

/// One row in `HostEvt::AppList` — a snapshot of a hosted app's state. Used by
/// `tetron-wm ps`. `age_secs` is seconds since the app was spawned (so the client
/// doesn't need a synchronized clock with the apphost).
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AppListEntry {
    pub app: u64,
    pub cmd: String,
    pub args: Vec<String>,
    pub pid: Option<u32>,
    pub cols: i32,
    pub rows: i32,
    pub age_secs: u64,
    pub alive: bool,
}

/// apphost → frontend.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum HostEvt {
    /// The app's current grid + placements, plus any not-yet-sent image blobs.
    Frame {
        app: u64,
        grid: CellBuffer,
        placements: Vec<Placement>,
        images: Vec<ImgBlob>,
        alive: bool,
        mouse: crate::mouse::AppMouse,
        /// The app's text cursor `(col, row)` in grid coords, or `None` when
        /// hidden/scrolled away (default `None` across version skew).
        #[serde(default)]
        cursor: Option<(u16, u16)>,
        /// Bell rings since the previous frame (default 0 across version skew).
        #[serde(default)]
        bells: u32,
        /// The app's latest OSC-52 clipboard store, forwarded to the host.
        #[serde(default)]
        clip: Option<String>,
        /// OSC 52 clipboard read queries from the app (default none).
        #[serde(default)]
        clip_reqs: Vec<ClipReq>,
        /// v5+: the grid in compact form, replacing `grid` (sent empty). Only
        /// sent to a frontend that announced v5 with `HostReq::Hello`.
        #[serde(default)]
        packed: Option<PackedGrid>,
    },
    /// The app's child exited.
    Gone { app: u64 },
    /// Sent right after a frontend connects so it can rebuild its window list.
    Roster {
        apps: Vec<RosterEntry>,
        /// The apphost's [`PROTO_VERSION`] (0 = an apphost too old to say).
        #[serde(default)]
        proto: u32,
    },
    SpawnFailed { req_id: u64, error: String },
    /// Reply to `HostReq::Spawn`. The apphost now also reports the child's
    /// OS pid (v2+; v1 apphosts omit the field, the frontend fills `None`).
    /// The daemon's activity monitor uses this to fill the panel's `pid`
    /// column in normal daemon mode.
    Spawned {
        req_id: u64,
        app: u64,
        #[serde(default)]
        pid: Option<u32>,
    },
    /// Reply to `HostReq::ListApps`.
    AppList { apps: Vec<AppListEntry> },
}

/// A `w` x `h` grid as base64 of binary rows, each `y` (varint) followed by
/// runs of `count` (varint) + one cell (UTF-8 char, fg RGBA, bg RGBA, attrs
/// bits) covering the row. Only rows that differ from the previous grid sent
/// to this frontend are present; the frontend applies them over its cached
/// copy, or over a blank grid when it has none of this size.
#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
pub struct PackedGrid {
    pub w: i32,
    pub h: i32,
    pub rows: String,
}

fn put_varint(out: &mut Vec<u8>, mut v: u32) {
    while v >= 0x80 {
        out.push(v as u8 | 0x80);
        v >>= 7;
    }
    out.push(v as u8);
}

fn get_varint(b: &[u8], i: &mut usize) -> Option<u32> {
    let mut v = 0u32;
    for shift in (0..35).step_by(7) {
        let x = *b.get(*i)?;
        *i += 1;
        v |= ((x & 0x7f) as u32) << shift;
        if x & 0x80 == 0 {
            return Some(v);
        }
    }
    None
}

fn put_cell(out: &mut Vec<u8>, c: &crate::cell::Cell) {
    let mut u = [0u8; 4];
    out.extend_from_slice(c.ch.encode_utf8(&mut u).as_bytes());
    out.extend_from_slice(&[c.fg.r, c.fg.g, c.fg.b, c.fg.a, c.bg.r, c.bg.g, c.bg.b, c.bg.a]);
    let a = c.attrs;
    out.push(a.bold as u8 | (a.italic as u8) << 1 | (a.underline as u8) << 2 | (a.inverse as u8) << 3);
}

fn get_cell(b: &[u8], i: &mut usize) -> Option<crate::cell::Cell> {
    use crate::cell::{Cell, CellAttrs, Rgba};
    let n = match *b.get(*i)? {
        x if x < 0x80 => 1,
        x if x >= 0xf0 => 4,
        x if x >= 0xe0 => 3,
        _ => 2,
    };
    let ch = std::str::from_utf8(b.get(*i..*i + n)?).ok()?.chars().next()?;
    let r = b.get(*i + n..*i + n + 9)?;
    *i += n + 9;
    let attrs = CellAttrs { bold: r[8] & 1 != 0, italic: r[8] & 2 != 0, underline: r[8] & 4 != 0, inverse: r[8] & 8 != 0 };
    Some(Cell { ch, fg: Rgba::new(r[0], r[1], r[2], r[3]), bg: Rgba::new(r[4], r[5], r[6], r[7]), attrs })
}

/// Pack the rows of `grid` that differ from `prev` (all rows when `prev` is
/// `None` or another size). Returns `None` when nothing changed.
pub fn pack_grid(grid: &CellBuffer, prev: Option<&CellBuffer>) -> Option<PackedGrid> {
    let (w, h) = (grid.width(), grid.height());
    let prev = prev.filter(|p| p.width() == w && p.height() == h);
    let mut out = Vec::new();
    let mut any = prev.is_none();
    for y in 0..h {
        if let Some(p) = prev {
            if (0..w).all(|x| grid.get(x, y) == p.get(x, y)) {
                continue;
            }
        }
        any = true;
        put_varint(&mut out, y as u32);
        let mut x = 0;
        while x < w {
            let c = grid.get(x, y).unwrap();
            let mut n = 1;
            while x + n < w && grid.get(x + n, y) == Some(c) {
                n += 1;
            }
            put_varint(&mut out, n as u32);
            put_cell(&mut out, c);
            x += n;
        }
    }
    use base64::Engine;
    any.then(|| PackedGrid { w, h, rows: base64::engine::general_purpose::STANDARD.encode(&out) })
}

/// Apply `p` over `base` (used only when it has the same size, else a blank
/// grid). `None` if the payload is malformed.
pub fn unpack_grid(p: &PackedGrid, base: Option<CellBuffer>) -> Option<CellBuffer> {
    use base64::Engine;
    let b = base64::engine::general_purpose::STANDARD.decode(&p.rows).ok()?;
    let mut g = base.filter(|g| g.width() == p.w && g.height() == p.h).unwrap_or_else(|| CellBuffer::new(p.w, p.h));
    let mut i = 0;
    while i < b.len() {
        let y = get_varint(&b, &mut i)? as i32;
        if y >= p.h {
            return None;
        }
        let mut x = 0;
        while x < p.w {
            let n = get_varint(&b, &mut i)? as i32;
            let c = get_cell(&b, &mut i)?;
            if n == 0 || x + n > p.w {
                return None;
            }
            for dx in 0..n {
                g.set(x + dx, y, c);
            }
            x += n;
        }
    }
    Some(g)
}

/// Write a newline-JSON message. Returns `Err` if the peer is gone.
pub fn send<T: Serialize, W: Write>(w: &mut W, msg: &T) -> std::io::Result<()> {
    let mut buf = serde_json::to_vec(msg).map_err(std::io::Error::other)?;
    buf.push(b'\n');
    w.write_all(&buf)
}

/// Read one newline-JSON message into `T`. `Ok(None)` on EOF.
///
/// A single unparseable (or blank) line is logged and SKIPPED rather than
/// returned as an error: the reader loops break on `Err`, so without this a
/// lone corrupted/truncated frame would tear down the whole apphost connection
/// — dropping the frame stream for every window at once. Genuine IO errors
/// from the socket still propagate (via `?`), so a dead peer breaks the loop
/// instead of busy-spinning.
pub fn recv<T: for<'de> Deserialize<'de>, R: BufRead>(r: &mut R) -> std::io::Result<Option<T>> {
    loop {
        let mut line = String::new();
        if r.read_line(&mut line)? == 0 {
            return Ok(None); // EOF
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue; // tolerate blank/keepalive lines
        }
        match serde_json::from_str(trimmed) {
            Ok(msg) => return Ok(Some(msg)),
            Err(e) => crate::dbg_log(&format!("proto: skipping unparseable line ({e})")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recv_skips_a_bad_line_and_keeps_the_stream_alive() {
        // A corrupted/blank line between two valid frames must NOT end the
        // stream (which would drop every window's frames at once).
        let stream = b"not json at all\n\n{\"Gone\":{\"app\":7}}\n";
        let mut r = std::io::BufReader::new(&stream[..]);
        match recv::<HostEvt, _>(&mut r).unwrap().unwrap() {
            HostEvt::Gone { app } => assert_eq!(app, 7, "recovered the next valid frame"),
            other => panic!("expected Gone after skipping junk, got {other:?}"),
        }
        // Then clean EOF.
        assert!(recv::<HostEvt, _>(&mut r).unwrap().is_none());
    }

    #[test]
    fn roster_proto_round_trips_and_legacy_defaults_to_zero() {
        let evt = HostEvt::Roster { apps: vec![], proto: PROTO_VERSION };
        let mut buf: Vec<u8> = Vec::new();
        send(&mut buf, &evt).unwrap();
        let mut r = std::io::BufReader::new(&buf[..]);
        match recv::<HostEvt, _>(&mut r).unwrap().unwrap() {
            HostEvt::Roster { proto, .. } => assert_eq!(proto, PROTO_VERSION),
            _ => panic!("wrong variant"),
        }
        // A roster from an apphost that predates the field parses with proto 0.
        let legacy = br#"{"Roster":{"apps":[]}}
"#;
        let mut r = std::io::BufReader::new(&legacy[..]);
        match recv::<HostEvt, _>(&mut r).unwrap().unwrap() {
            HostEvt::Roster { proto, .. } => assert_eq!(proto, 0),
            _ => panic!("wrong variant"),
        }
    }
    #[test]
    fn spawned_with_pid_round_trips_and_legacy_omits_pid() {
        // Modern v2 wire: Spawned carries the child pid.
        let evt = HostEvt::Spawned { req_id: 42, app: 7, pid: Some(12345) };
        let mut buf: Vec<u8> = Vec::new();
        send(&mut buf, &evt).unwrap();
        let mut r = std::io::BufReader::new(&buf[..]);
        match recv::<HostEvt, _>(&mut r).unwrap().unwrap() {
            HostEvt::Spawned { req_id, app, pid } => {
                assert_eq!(req_id, 42);
                assert_eq!(app, 7);
                assert_eq!(pid, Some(12345));
            }
            _ => panic!("wrong variant"),
        }
        // A v1 apphost that predates the pid field still parses fine on a
        // v2 frontend (#[serde(default)] on `pid`).
        let legacy = br#"{"Spawned":{"req_id":1,"app":2}}
"#;
        let mut r = std::io::BufReader::new(&legacy[..]);
        match recv::<HostEvt, _>(&mut r).unwrap().unwrap() {
            HostEvt::Spawned { req_id, app, pid } => {
                assert_eq!(req_id, 1);
                assert_eq!(app, 2);
                assert_eq!(pid, None);
            }
            _ => panic!("wrong variant"),
        }
    }
    use crate::cell::Cell;

    #[test]
    fn req_round_trips() {
        let msgs = vec![
            HostReq::Spawn { req_id: 7, cmd: "sh".into(), args: vec!["-c".into()], cwd: None, cols: 80, rows: 24 },
            HostReq::Input { app: 3, bytes: vec![1, 2, 3] },
            HostReq::Resize { app: 3, cols: 100, rows: 40 },
            HostReq::SetMeta { app: 3, meta: vec![9, 9] },
            HostReq::Kill { app: 3 },
            HostReq::ListApps,
            HostReq::Shutdown,
        ];
        for m in msgs {
            let mut buf: Vec<u8> = Vec::new();
            send(&mut buf, &m).unwrap();
            let mut r = std::io::BufReader::new(&buf[..]);
            let back: HostReq = recv(&mut r).unwrap().unwrap();
            assert_eq!(format!("{m:?}"), format!("{back:?}"));
        }
    }

    #[test]
    fn frame_round_trips_with_grid() {
        let mut grid = CellBuffer::new(4, 2);
        grid.set(1, 0, Cell { ch: 'X', ..Default::default() });
        let evt = HostEvt::Frame {
            app: 5,
            grid: grid.clone(),
            placements: vec![Placement { image_id: 1, col: 0, row: 0, cols: 2, rows: 1 }],
            images: vec![ImgBlob { image_id: 1, png_b64: "QUJD".into() }],
            alive: true,
            mouse: Default::default(),
            cursor: Some((1, 0)),
            bells: 2,
            clip: Some("copied".into()),
            clip_reqs: vec![ClipReq { kind: 'c', bel: true }],
            packed: None,
        };
        let mut buf: Vec<u8> = Vec::new();
        send(&mut buf, &evt).unwrap();
        let mut r = std::io::BufReader::new(&buf[..]);
        let back: HostEvt = recv(&mut r).unwrap().unwrap();
        match back {
            HostEvt::Frame { app, grid: g, placements, images, alive, mouse, cursor, bells, clip, clip_reqs, .. } => {
                assert_eq!(app, 5);
                assert_eq!(g, grid);
                assert_eq!(placements.len(), 1);
                assert_eq!(images[0].png_b64, "QUJD");
                assert!(alive);
                assert_eq!(mouse, Default::default());
                assert_eq!(cursor, Some((1, 0)));
                assert_eq!(bells, 2);
                assert_eq!(clip.as_deref(), Some("copied"));
                assert_eq!(clip_reqs, vec![ClipReq { kind: 'c', bel: true }]);
            }
            _ => panic!("wrong variant"),
        }
    }

    /// A 200x50 grid like a busy terminal: coloured text, blank tails, a wide glyph.
    fn busy_grid(seed: u32) -> CellBuffer {
        use crate::cell::{CellAttrs, Rgba};
        let mut g = CellBuffer::new(200, 50);
        for y in 0..50 {
            for x in 0..(120 + (y * 7 + seed as i32) % 80) {
                let k = (x as u32).wrapping_mul(31).wrapping_add(y as u32 * 17 + seed);
                let ch = if x == 3 { '\u{1F4C1}' } else { (b'!' + (k % 90) as u8) as char };
                let fg = Rgba::rgb((k % 7 * 30) as u8, 200, (k % 5 * 50) as u8);
                g.set(x, y, Cell { ch, fg, bg: Rgba::TRANSPARENT, attrs: CellAttrs { bold: k.is_multiple_of(3), ..Default::default() } });
            }
        }
        g
    }

    #[test]
    fn packed_grid_round_trips_full_and_delta() {
        let a = busy_grid(1);
        let full = pack_grid(&a, None).unwrap();
        assert_eq!(unpack_grid(&full, None).unwrap(), a);
        // Unchanged: nothing to send.
        assert!(pack_grid(&a, Some(&a)).is_none());
        // One changed row: only that row rides, and applies over the cache.
        let mut b = a.clone();
        b.set(5, 7, Cell { ch: 'Z', ..Default::default() });
        let delta = pack_grid(&b, Some(&a)).unwrap();
        assert!(delta.rows.len() < full.rows.len() / 20, "delta {} vs full {}", delta.rows.len(), full.rows.len());
        assert_eq!(unpack_grid(&delta, Some(a.clone())).unwrap(), b);
        // A size change sends every row and ignores a mismatched cache.
        let c = CellBuffer::new(10, 3);
        assert_eq!(unpack_grid(&pack_grid(&c, Some(&a)).unwrap(), Some(a.clone())).unwrap(), c);
        // Malformed payloads are rejected, not panicked on.
        assert!(unpack_grid(&PackedGrid { w: 4, h: 1, rows: "AAAA".into() }, None).is_none());
        assert!(unpack_grid(&PackedGrid { w: 4, h: 1, rows: "!!".into() }, None).is_none());
    }

    #[test]
    fn frame_skew_parses_both_ways() {
        // A v4 apphost frame (no `packed`) parses on this frontend.
        let mut buf: Vec<u8> = Vec::new();
        let grid = busy_grid(2);
        send(&mut buf, &HostEvt::Frame { app: 1, grid: grid.clone(), placements: vec![], images: vec![], alive: true, mouse: Default::default(), cursor: None, bells: 0, clip: None, clip_reqs: vec![], packed: None }).unwrap();
        let line = String::from_utf8(buf).unwrap().replace(",\"packed\":null", "");
        assert!(!line.contains("packed"));
        match recv::<HostEvt, _>(&mut std::io::BufReader::new(line.as_bytes())).unwrap().unwrap() {
            HostEvt::Frame { grid: g, packed, .. } => { assert_eq!(g, grid); assert!(packed.is_none()); }
            _ => panic!("wrong variant"),
        }
        // Hello is a plain unit-ish variant an old apphost skips as unparseable.
        let mut buf: Vec<u8> = Vec::new();
        send(&mut buf, &HostReq::Hello { proto: PROTO_VERSION }).unwrap();
        assert_eq!(String::from_utf8(buf).unwrap(), format!("{{\"Hello\":{{\"proto\":{PROTO_VERSION}}}}}\n"));
    }

    /// Timing, old JSON grid vs packed (run with `--ignored --nocapture`, release).
    #[test]
    #[ignore]
    fn packed_grid_timing() {
        let frames: Vec<CellBuffer> = (0..50).map(busy_grid).collect();
        let t = std::time::Instant::now();
        let mut bytes = 0;
        for g in &frames {
            let s = serde_json::to_string(g).unwrap();
            bytes += s.len();
            let _: CellBuffer = serde_json::from_str(&s).unwrap();
        }
        let json = t.elapsed() / frames.len() as u32;
        let jbytes = bytes / frames.len();
        let t = std::time::Instant::now();
        let mut bytes = 0;
        for g in &frames {
            let s = serde_json::to_string(&pack_grid(g, None).unwrap()).unwrap();
            bytes += s.len();
            let p: PackedGrid = serde_json::from_str(&s).unwrap();
            unpack_grid(&p, None).unwrap();
        }
        let packed = t.elapsed() / frames.len() as u32;
        let pbytes = bytes / frames.len();
        println!("200x50 full grid encode+decode: json {json:?} {jbytes} B, packed {packed:?} {pbytes} B");
    }
}
