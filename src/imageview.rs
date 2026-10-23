//! The built-in image viewer window. It picks the best display the attached
//! terminal has: Kitty graphics when the client supports them, otherwise the
//! image drawn as cells by chafa (works everywhere, a bare VT included), or a
//! built-in half-block renderer when chafa is missing. Zoom, pan and
//! next/previous image in the folder are handled here; decoding and rendering
//! run on background threads so the desktop loop never waits for them.

use std::path::{Path, PathBuf};
use std::sync::{mpsc, Arc};
use std::time::{Duration, Instant};

use image::RgbaImage;

use crate::buffer::CellBuffer;
use crate::cell::{Cell, Rgba};
use crate::display::ClientCaps;
use crate::imagestore::{ImageId, ImageStore};

/// Longest side kept in memory (and sent for Kitty graphics) for a viewed image.
const MAX_SIDE: u32 = 2048;
/// Pixels per cell for the crop handed to chafa; only the 1:2 ratio matters.
const CELL_PX: (u32, u32) = (8, 16);
const ZOOM_MAX: f32 = 16.0;
const ZOOM_STEP: f32 = 1.25;
/// A resize re-renders only once the size has been stable this long.
const SETTLE: Duration = Duration::from_millis(120);

/// How the viewer draws the image (`image_viewer` in the config, `m` cycles).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Method {
    /// Kitty graphics when the terminal has them (never on a bare VT), else cells.
    #[default]
    Auto,
    /// Kitty graphics whenever the terminal reports them.
    Graphics,
    /// Always cells (chafa, or the built-in half blocks).
    Cells,
}

impl Method {
    pub fn parse(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "graphics" | "kitty" => Method::Graphics,
            "cells" | "chafa" | "text" => Method::Cells,
            _ => Method::Auto,
        }
    }

    fn next(self) -> Self {
        match self {
            Method::Auto => Method::Cells,
            Method::Cells => Method::Graphics,
            Method::Graphics => Method::Auto,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Method::Auto => "auto",
            Method::Graphics => "graphics",
            Method::Cells => "cells",
        }
    }

    /// Whether Kitty graphics are used for a client with `caps`.
    pub fn uses_graphics(self, caps: ClientCaps) -> bool {
        caps.kitty_graphics
            && match self {
                Method::Auto => !caps.vt,
                Method::Graphics => true,
                Method::Cells => false,
            }
    }
}

/// Viewer keys (the client sends them while a viewer has focus).
#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub enum ImageKey {
    ZoomIn,
    ZoomOut,
    Fit,
    Left,
    Right,
    Up,
    Down,
    Next,
    Prev,
    Mode,
    Info,
    Close,
    /// Hand the screen to fbi (plain console only).
    Fullscreen,
}

/// `path` first, then the rest of `all` after it in order, wrapping: the list
/// a full-screen viewer gets, so its next / previous still walk the folder.
pub fn starting_at(all: &[PathBuf], path: &Path) -> Vec<PathBuf> {
    match all.iter().position(|p| p == path) {
        Some(i) => all[i..].iter().chain(&all[..i]).cloned().collect(),
        None => vec![path.to_path_buf()],
    }
}

/// What part of the image to show where: the source crop in image pixels and
/// the destination cells relative to the image area.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Fit {
    pub sx: u32,
    pub sy: u32,
    pub sw: u32,
    pub sh: u32,
    pub dx: i32,
    pub dy: i32,
    pub dw: i32,
    pub dh: i32,
}

/// Fit an `iw`×`ih` image into `w`×`h` cells (a cell is twice as tall as it is
/// wide) at `zoom` (1 = the whole image), centred on (`cx`, `cy`) in 0..1 image
/// coordinates. The view is clamped to stay inside the image.
pub fn fit(iw: u32, ih: u32, w: i32, h: i32, zoom: f32, cx: f32, cy: f32) -> Option<Fit> {
    if iw == 0 || ih == 0 || w <= 0 || h <= 0 {
        return None;
    }
    let (iwf, ihf) = (iw as f32, ih as f32);
    // The area in cell-width units: w wide, 2h tall.
    let (bw, bh) = (w as f32, h as f32 * 2.0);
    let scale = (bw / iwf).min(bh / ihf) * zoom.max(1.0);
    let sw = (bw / scale).min(iwf);
    let sh = (bh / scale).min(ihf);
    let x0 = (cx * iwf - sw / 2.0).clamp(0.0, iwf - sw);
    let y0 = (cy * ihf - sh / 2.0).clamp(0.0, ihf - sh);
    let dw = ((sw * scale).round() as i32).clamp(1, w);
    let dh = ((sh * scale / 2.0).round() as i32).clamp(1, h);
    let sx = (x0.round() as u32).min(iw - 1);
    let sy = (y0.round() as u32).min(ih - 1);
    Some(Fit {
        sx,
        sy,
        sw: (sw.round() as u32).clamp(1, iw - sx),
        sh: (sh.round() as u32).clamp(1, ih - sy),
        dx: (w - dw) / 2,
        dy: (h - dh) / 2,
        dw,
        dh,
    })
}

/// Draw `img` (exactly `w` × `2h` pixels) as `w`×`h` cells, two pixels per cell
/// with `▀`: the fallback when chafa is missing. A bare VT's fonts have no half
/// blocks, so there each cell is one averaged pixel as a coloured space.
/// Transparency is flattened over `bg`.
pub fn halfblocks(img: &RgbaImage, w: i32, h: i32, vt: bool, bg: Rgba) -> CellBuffer {
    let mut buf = CellBuffer::new(w, h);
    let px = |x: i32, y: i32| -> Rgba {
        let p = img.get_pixel(x.min(img.width() as i32 - 1) as u32, y.min(img.height() as i32 - 1) as u32).0;
        let a = p[3] as u32;
        let mix = |c: u8, b: u8| ((c as u32 * a + b as u32 * (255 - a)) / 255) as u8;
        Rgba::rgb(mix(p[0], bg.r), mix(p[1], bg.g), mix(p[2], bg.b))
    };
    for y in 0..h {
        for x in 0..w {
            let (top, bot) = (px(x, 2 * y), px(x, 2 * y + 1));
            let cell = if vt {
                let avg = |a: u8, b: u8| ((a as u16 + b as u16) / 2) as u8;
                let c = Rgba::rgb(avg(top.r, bot.r), avg(top.g, bot.g), avg(top.b, bot.b));
                Cell { ch: ' ', fg: c, bg: c, attrs: Default::default() }
            } else {
                Cell { ch: crate::glyphs::HALF_BLOCK_UPPER, fg: top, bg: bot, attrs: Default::default() }
            };
            buf.set(x, y, cell);
        }
    }
    buf
}

/// Everything a cell render depends on; a finished render is used only while
/// its key still matches.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct Key {
    gen: u64,
    fit: Fit,
    caps: ClientCaps,
}

struct Loaded {
    img: Arc<RgbaImage>,
    dims: (u32, u32),
    png: Option<Vec<u8>>,
    siblings: Vec<PathBuf>,
    /// A PDF's page count (`None` for an image).
    pages: Option<u32>,
}

enum Done {
    Loaded { gen: u64, result: Result<Loaded, String> },
    Cells { key: Key, buf: CellBuffer, via: &'static str },
}

pub struct ImageView {
    path: PathBuf,
    siblings: Vec<PathBuf>,
    /// For a PDF: the shown page (1-based) and the page count (0 until known).
    /// n / p then turn pages instead of stepping through the folder.
    doc: Option<(u32, u32)>,
    /// Bumped on every (re)load so late results for an older image are dropped.
    gen: u64,
    img: Option<Arc<RgbaImage>>,
    /// The file's own pixel size (the kept copy may be downscaled).
    dims: (u32, u32),
    error: Option<String>,
    png_id: Option<ImageId>,
    zoom: f32,
    cx: f32,
    cy: f32,
    method: Method,
    info: bool,
    cells: Option<(Key, CellBuffer, &'static str)>,
    in_flight: Option<Key>,
    pending: Option<(Key, Instant)>,
    /// The current fit and whether Kitty graphics show it (set by `poll`).
    cur: Option<Key>,
    gfx_shown: bool,
    tx: mpsc::Sender<Done>,
    rx: mpsc::Receiver<Done>,
}

impl ImageView {
    /// Open `path`; decoding starts in the background at once.
    pub fn new(path: impl Into<PathBuf>, method: Method) -> Self {
        let (tx, rx) = mpsc::channel();
        let path: PathBuf = path.into();
        let doc = is_pdf(&path).then_some((1, 0));
        let mut v = ImageView {
            path,
            siblings: Vec::new(),
            doc,
            gen: 0,
            img: None,
            dims: (0, 0),
            error: None,
            png_id: None,
            zoom: 1.0,
            cx: 0.5,
            cy: 0.5,
            method,
            info: true,
            cells: None,
            in_flight: None,
            pending: None,
            cur: None,
            gfx_shown: false,
            tx,
            rx,
        };
        v.load();
        v
    }

    /// The file being shown.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Whether this view shows a PDF (pages instead of folder images).
    pub fn is_pdf(&self) -> bool {
        self.doc.is_some()
    }

    /// Window title: the file name, plus its position among the folder's
    /// images, or the page for a PDF.
    pub fn title(&self) -> String {
        let name = self.path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        if let Some((page, pages)) = self.doc {
            return match pages {
                0 => format!("pdf: {name}"),
                n => format!("pdf: {name} (page {page}/{n})"),
            };
        }
        match self.siblings.iter().position(|p| *p == self.path) {
            Some(i) if self.siblings.len() > 1 => format!("image: {name} ({}/{})", i + 1, self.siblings.len()),
            _ => format!("image: {name}"),
        }
    }

    fn load(&mut self) {
        self.gen += 1;
        self.img = None;
        self.error = None;
        self.png_id = None;
        self.cells = None;
        self.in_flight = None;
        self.pending = None;
        self.zoom = 1.0;
        self.cx = 0.5;
        self.cy = 0.5;
        let (gen, path, tx) = (self.gen, self.path.clone(), self.tx.clone());
        let page = self.doc.map(|(p, _)| p);
        crate::dbg_log(&format!("imageview: load {}{}", path.display(), page.map(|p| format!(" page {p}")).unwrap_or_default()));
        std::thread::spawn(move || {
            let result = match page {
                Some(p) => load_pdf_page(&path, p),
                None => load_file(&path),
            };
            if let Err(e) = &result {
                crate::dbg_log(&format!("imageview: {}: {e}", path.display()));
            }
            let _ = tx.send(Done::Loaded { gen, result });
            crate::daemon::wake();
        });
    }

    /// Apply finished background work and start what the current window size,
    /// zoom and client need. `w`×`h` is the content area, `unobstructed` whether
    /// a Kitty placement would be visible. Call every tick; cheap when idle.
    pub fn poll(&mut self, w: i32, h: i32, caps: ClientCaps, unobstructed: bool, store: &mut ImageStore) {
        while let Ok(done) = self.rx.try_recv() {
            match done {
                Done::Loaded { gen, result } if gen == self.gen => match result {
                    Ok(l) => {
                        self.dims = l.dims;
                        self.png_id = l.png.map(|png| store.store_png(png, l.img.width(), l.img.height()));
                        self.img = Some(l.img);
                        if let (Some(n), Some((_, pages))) = (l.pages, self.doc.as_mut()) {
                            *pages = n;
                        }
                        if !l.siblings.is_empty() {
                            self.siblings = l.siblings;
                        }
                    }
                    Err(e) => self.error = Some(e),
                },
                Done::Cells { key, buf, via } if Some(key) == self.in_flight => {
                    self.in_flight = None;
                    self.cells = Some((key, buf, via));
                }
                _ => {}
            }
        }
        let Some(img) = self.img.clone() else {
            self.cur = None;
            self.gfx_shown = false;
            return;
        };
        let ah = h - self.info_rows(h);
        let Some(f) = fit(img.width(), img.height(), w, ah, self.zoom, self.cx, self.cy) else {
            self.cur = None;
            return;
        };
        let key = Key { gen: self.gen, fit: f, caps };
        self.cur = Some(key);
        let graphics = self.method.uses_graphics(caps) && self.png_id.is_some();
        self.gfx_shown = graphics && unobstructed;
        // Cells are drawn whenever the image is not shown as graphics (and kept
        // ready under graphics, so an overlapping window shows the cells).
        let have = self.cells.as_ref().map(|c| c.0);
        if have == Some(key) || self.in_flight == Some(key) {
            self.pending = None;
            return;
        }
        let start = match self.pending {
            _ if have.is_none() => true,
            Some((k, t)) if k == key => t.elapsed() >= SETTLE,
            _ => {
                self.pending = Some((key, Instant::now()));
                false
            }
        };
        if start {
            self.pending = None;
            self.in_flight = Some(key);
            let tx = self.tx.clone();
            let bg = crate::theme::current().window_bg;
            std::thread::spawn(move || {
                let (buf, via) = render_cells(&img, &key.fit, key.caps.vt, bg);
                let _ = tx.send(Done::Cells { key, buf, via });
                crate::daemon::wake();
            });
        }
    }

    fn info_rows(&self, h: i32) -> i32 {
        i32::from(self.info && h > 2)
    }

    /// The Kitty placement for this view inside `content` (screen cells): the
    /// transmitted image, its fitted rect and the source crop. `None` when the
    /// view is drawn as cells.
    pub fn placement(&self, content: crate::geometry::Rect) -> Option<(ImageId, crate::geometry::Rect, [u32; 4])> {
        let (id, key) = (self.png_id?, self.cur?);
        if !self.method.uses_graphics(key.caps) {
            return None;
        }
        let img = self.img.as_ref()?;
        let f = key.fit;
        let rect = crate::geometry::Rect::new(content.x + f.dx, content.y + f.dy, f.dw, f.dh);
        // The transmitted PNG is the kept copy, the same pixels `fit` measured.
        debug_assert!(f.sx + f.sw <= img.width() && f.sy + f.sh <= img.height());
        Some((id, rect, [f.sx, f.sy, f.sw, f.sh]))
    }

    /// Handle a viewer key. Returns `true` when the view should close.
    pub fn key(&mut self, k: ImageKey) -> bool {
        let zoomed = self.zoom > 1.0;
        // A pan step is a fifth of what is visible.
        let step = 0.2 / self.zoom;
        match k {
            ImageKey::ZoomIn => self.zoom = (self.zoom * ZOOM_STEP).min(ZOOM_MAX),
            ImageKey::ZoomOut => self.zoom = (self.zoom / ZOOM_STEP).max(1.0),
            ImageKey::Fit => {
                self.zoom = 1.0;
                self.cx = 0.5;
                self.cy = 0.5;
            }
            ImageKey::Left if zoomed => self.cx -= step,
            ImageKey::Right if zoomed => self.cx += step,
            ImageKey::Up => self.cy -= step,
            ImageKey::Down => self.cy += step,
            ImageKey::Left | ImageKey::Prev if self.doc.is_some() => self.step_page(-1),
            ImageKey::Right | ImageKey::Next if self.doc.is_some() => self.step_page(1),
            ImageKey::Left | ImageKey::Prev => self.step_image(-1),
            ImageKey::Right | ImageKey::Next => self.step_image(1),
            ImageKey::Mode => self.method = self.method.next(),
            ImageKey::Info => self.info = !self.info,
            ImageKey::Close => return true,
            ImageKey::Fullscreen => {} // the session hands the screen over
        }
        // Keep the centre where the view can actually reach, so panning back
        // responds at once after hitting an edge.
        let half = 0.5 / self.zoom;
        self.cx = self.cx.clamp(half, 1.0 - half);
        self.cy = self.cy.clamp(half, 1.0 - half);
        false
    }

    /// Whether the view is zoomed in (so it can pan).
    pub fn zoomed(&self) -> bool {
        self.zoom > 1.0
    }

    /// Pan by a mouse drag of (`dx`, `dy`) cells: the image follows the pointer.
    pub fn drag_by(&mut self, dx: i32, dy: i32) {
        let (Some(key), Some(img)) = (self.cur, self.img.as_ref()) else { return };
        let f = key.fit;
        // Source pixels per destination cell, as a fraction of the image.
        self.cx -= dx as f32 * f.sw as f32 / f.dw.max(1) as f32 / img.width() as f32;
        self.cy -= dy as f32 * f.sh as f32 / f.dh.max(1) as f32 / img.height() as f32;
        let half = 0.5 / self.zoom;
        self.cx = self.cx.clamp(half, 1.0 - half);
        self.cy = self.cy.clamp(half, 1.0 - half);
    }

    /// The files a full-screen viewer should get: this image first.
    pub fn fullscreen_files(&self) -> Vec<PathBuf> {
        if self.siblings.is_empty() {
            vec![self.path.clone()]
        } else {
            starting_at(&self.siblings, &self.path)
        }
    }

    /// Turn to the previous / next page of a PDF (clamped, no wrap).
    fn step_page(&mut self, d: i32) {
        let Some((page, pages)) = self.doc else { return };
        let last = if pages == 0 { page } else { pages };
        let next = (page as i64 + d as i64).clamp(1, last.max(1) as i64) as u32;
        if next != page {
            self.doc = Some((next, pages));
            self.load();
        }
    }

    fn step_image(&mut self, d: i32) {
        let n = self.siblings.len() as i32;
        if n < 2 {
            return;
        }
        let i = self.siblings.iter().position(|p| *p == self.path).unwrap_or(0) as i32;
        self.path = self.siblings[(i + d).rem_euclid(n) as usize].clone();
        self.load();
    }

    /// The window contents: the cell render (or a status line), with the info
    /// line at the bottom. Under Kitty graphics the image area stays blank so a
    /// transparent image does not show the cell render through it.
    pub fn render(&self, w: i32, h: i32) -> CellBuffer {
        let t = crate::theme::current();
        let bg = t.window_bg;
        let mut buf = CellBuffer::new(w, h);
        buf.fill(Cell { ch: ' ', fg: t.dim, bg, attrs: Default::default() });
        let name = self.path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        let ah = h - self.info_rows(h);
        let centred = |buf: &mut CellBuffer, s: &str| {
            let x = ((w - s.chars().count() as i32) / 2).max(0);
            buf.write_str(x, (ah / 2).max(0), s, t.text, bg);
        };
        if let Some(e) = &self.error {
            centred(&mut buf, &format!("cannot display {name}: {e}"));
        } else if self.img.is_none() {
            centred(&mut buf, &format!("loading {name}\u{2026}"));
        } else if !self.gfx_shown {
            match &self.cells {
                Some((key, cells, _)) => {
                    let f = key.fit;
                    for y in 0..cells.height().min(ah - f.dy) {
                        for x in 0..cells.width().min(w - f.dx) {
                            if let Some(c) = cells.get(x, y) {
                                buf.set(f.dx + x, f.dy + y, *c);
                            }
                        }
                    }
                }
                None => centred(&mut buf, "rendering\u{2026}"),
            }
        }
        if self.info_rows(h) > 0 {
            buf.write_str(0, h - 1, &self.info_line(w), t.dim, bg);
        }
        buf
    }

    fn info_line(&self, w: i32) -> String {
        let via = match (self.gfx_shown, &self.cells) {
            (true, _) => "graphics",
            (false, Some((_, _, via))) => via,
            _ => "",
        };
        let what = match self.doc {
            Some((page, pages)) if pages > 0 => format!("page {page}/{pages}"),
            Some((page, _)) => format!("page {page}"),
            None => format!("{}x{}", self.dims.0, self.dims.1),
        };
        let left = format!(" {what}  {:.0}%  {} ({})", self.zoom * 100.0, via, self.method.label());
        let keys = if self.cur.is_some_and(|k| k.caps.fb_console) {
            "+/- zoom 0 fit n/p next m mode f full screen "
        } else {
            "+/- zoom 0 fit arrows pan n/p next m mode i info "
        };
        let pad = w as usize - left.chars().count().min(w as usize);
        if pad > keys.chars().count() {
            format!("{left}{}{keys}", " ".repeat(pad - keys.chars().count()))
        } else {
            left.chars().take(w.max(0) as usize).collect()
        }
    }
}

/// Read and decode `path`, keep at most `MAX_SIDE` pixels on the long side,
/// encode the Kitty copy, and list the folder's images for next/previous.
fn load_file(path: &Path) -> Result<Loaded, String> {
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    let mut l = decode(&bytes)?;
    l.siblings = siblings(path);
    Ok(l)
}

/// Decode an image, keep at most `MAX_SIDE` pixels on the long side, and
/// encode the Kitty copy.
fn decode(bytes: &[u8]) -> Result<Loaded, String> {
    let img = image::load_from_memory(bytes).map_err(|_| "unsupported or damaged image".to_string())?;
    let dims = (img.width(), img.height());
    let img = if dims.0 > MAX_SIDE || dims.1 > MAX_SIDE {
        img.resize(MAX_SIDE, MAX_SIDE, image::imageops::FilterType::Triangle)
    } else {
        img
    };
    let rgba = img.to_rgba8();
    let mut png = std::io::Cursor::new(Vec::new());
    let png = rgba.write_to(&mut png, image::ImageFormat::Png).ok().map(|_| png.into_inner());
    Ok(Loaded { img: Arc::new(rgba), dims, png, siblings: Vec::new(), pages: None })
}

/// Whether `path` is a PDF (by its extension).
pub fn is_pdf(path: &Path) -> bool {
    crate::openwith::classify(path, false) == crate::openwith::Role::Pdf
}

/// Render page `page` of a PDF with poppler's `pdftoppm` (150 dpi, bounded by
/// a timeout) and read the page count with `pdfinfo`.
fn load_pdf_page(path: &Path, page: u32) -> Result<Loaded, String> {
    static SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = crate::protocol::ensure_socket_dir().unwrap_or_else(|_| crate::protocol::socket_dir());
    let prefix = dir.join(format!("pdfpage-{}-{n}", std::process::id()));
    let (file, p, pre) = (path.to_string_lossy(), page.to_string(), prefix.to_string_lossy().into_owned());
    let args = ["-f", &p, "-l", &p, "-r", "150", "-png", "-singlefile", &file, &pre];
    match crate::system::run_capped_status("pdftoppm", &args, 20) {
        crate::system::Capped::Done => {}
        crate::system::Capped::Missing(_) => return Err("needs pdftoppm (poppler-utils)".into()),
        other => return Err(format!("pdftoppm failed: {other:?}")),
    }
    let png_path = prefix.with_extension("png");
    let bytes = std::fs::read(&png_path).map_err(|e| e.to_string());
    let _ = std::fs::remove_file(&png_path);
    let mut l = decode(&bytes?)?;
    l.pages = crate::system::run_capped("pdfinfo", &[&file], 5).as_deref().and_then(pdf_pages);
    Ok(l)
}

/// The page count from `pdfinfo` output ("Pages:          12").
fn pdf_pages(info: &str) -> Option<u32> {
    info.lines().find_map(|l| l.strip_prefix("Pages:")).and_then(|v| v.trim().parse().ok())
}

/// Image files in `path`'s folder, sorted by name (always including `path`).
pub fn siblings(path: &Path) -> Vec<PathBuf> {
    let Some(dir) = path.parent() else { return Vec::new() };
    let mut v: Vec<PathBuf> = std::fs::read_dir(dir)
        .map(|rd| {
            rd.filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| crate::openwith::classify(p, false) == crate::openwith::Role::Image)
                .collect()
        })
        .unwrap_or_default();
    if !v.iter().any(|p| p == path) {
        v.push(path.to_path_buf());
    }
    v.sort();
    v
}

/// The cells for `f` of `img`: chafa on the cropped, pre-scaled region, or the
/// built-in half blocks when chafa is missing or fails.
fn render_cells(img: &RgbaImage, f: &Fit, vt: bool, bg: Rgba) -> (CellBuffer, &'static str) {
    let crop = image::imageops::crop_imm(img, f.sx, f.sy, f.sw, f.sh).to_image();
    let (pw, ph) = (f.dw as u32 * CELL_PX.0, f.dh as u32 * CELL_PX.1);
    let scaled = image::imageops::resize(&crop, pw.max(1), ph.max(1), image::imageops::FilterType::Triangle);
    static SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = crate::protocol::ensure_socket_dir().unwrap_or_else(|_| crate::protocol::socket_dir());
    let tmp = dir.join(format!("imageview-{}-{n}.png", std::process::id()));
    let cells = scaled
        .save_with_format(&tmp, image::ImageFormat::Png)
        .ok()
        .and_then(|_| crate::wallpaper::viewer_cells(&tmp, f.dw, f.dh));
    let _ = std::fs::remove_file(&tmp);
    match cells {
        Some(c) => (c, "chafa"),
        None => {
            let half = image::imageops::resize(&crop, f.dw as u32, f.dh as u32 * 2, image::imageops::FilterType::Triangle);
            (halfblocks(&half, f.dw, f.dh, vt, bg), "half blocks")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn caps(kitty: bool, vt: bool) -> ClientCaps {
        ClientCaps { kitty_graphics: kitty, vt, ..Default::default() }
    }

    #[test]
    fn fit_keeps_the_aspect_ratio_and_centres() {
        // A square image in a 40x10 area: 10 rows tall is 20 cell-widths, so 20 wide.
        let f = fit(100, 100, 40, 10, 1.0, 0.5, 0.5).unwrap();
        assert_eq!((f.dw, f.dh), (20, 10));
        assert_eq!((f.dx, f.dy), (10, 0));
        assert_eq!((f.sx, f.sy, f.sw, f.sh), (0, 0, 100, 100));
        // A wide image is limited by the width instead.
        let f = fit(400, 100, 40, 20, 1.0, 0.5, 0.5).unwrap();
        assert_eq!((f.dw, f.dh), (40, 5));
        assert_eq!(f.dy, 7);
    }

    #[test]
    fn zoom_crops_the_source_and_pan_stays_inside() {
        let f = fit(100, 100, 20, 10, 2.0, 0.5, 0.5).unwrap();
        assert_eq!((f.sw, f.sh), (50, 50));
        assert_eq!((f.sx, f.sy), (25, 25));
        // Centre far outside: clamped to the edge.
        let f = fit(100, 100, 20, 10, 2.0, 5.0, -3.0).unwrap();
        assert_eq!((f.sx, f.sy), (50, 0));
        assert!(f.sx + f.sw <= 100 && f.sy + f.sh <= 100);
        assert!(fit(0, 10, 10, 10, 1.0, 0.5, 0.5).is_none());
        assert!(fit(10, 10, 0, 10, 1.0, 0.5, 0.5).is_none());
    }

    #[test]
    fn fit_never_crops_past_the_image_at_any_zoom_or_size() {
        for (iw, ih) in [(1, 1), (3, 1000), (1000, 3), (1891, 1153)] {
            for (w, h) in [(1, 1), (7, 3), (200, 50)] {
                for z in [1.0, 1.3, 4.0, 16.0] {
                    for c in [0.0, 0.37, 1.0] {
                        let f = fit(iw, ih, w, h, z, c, 1.0 - c).unwrap();
                        assert!(f.sx + f.sw <= iw && f.sy + f.sh <= ih, "{iw}x{ih} {w}x{h} z{z} {f:?}");
                        assert!(f.dx >= 0 && f.dy >= 0 && f.dx + f.dw <= w && f.dy + f.dh <= h, "{f:?}");
                    }
                }
            }
        }
    }

    #[test]
    fn method_picks_graphics_only_where_it_can_show() {
        assert!(Method::Auto.uses_graphics(caps(true, false)));
        assert!(!Method::Auto.uses_graphics(caps(true, true)), "a bare VT defaults to cells");
        assert!(!Method::Auto.uses_graphics(caps(false, false)));
        assert!(Method::Graphics.uses_graphics(caps(true, true)));
        assert!(!Method::Graphics.uses_graphics(caps(false, false)), "no graphics without support");
        assert!(!Method::Cells.uses_graphics(caps(true, false)));
        assert_eq!(Method::parse("Cells"), Method::Cells);
        assert_eq!(Method::parse("kitty"), Method::Graphics);
        assert_eq!(Method::parse("bogus"), Method::Auto);
    }

    #[test]
    fn halfblocks_use_two_pixels_per_cell_and_spaces_on_a_vt() {
        let mut img = RgbaImage::new(1, 2);
        img.put_pixel(0, 0, image::Rgba([255, 0, 0, 255]));
        img.put_pixel(0, 1, image::Rgba([0, 0, 255, 255]));
        let b = halfblocks(&img, 1, 1, false, Rgba::rgb(0, 0, 0));
        let c = b.get(0, 0).unwrap();
        assert_eq!((c.ch, c.fg, c.bg), (crate::glyphs::HALF_BLOCK_UPPER, Rgba::rgb(255, 0, 0), Rgba::rgb(0, 0, 255)));
        let c = *halfblocks(&img, 1, 1, true, Rgba::rgb(0, 0, 0)).get(0, 0).unwrap();
        assert_eq!((c.ch, c.bg), (' ', Rgba::rgb(127, 0, 127)));
    }

    /// A temp folder with three small PNGs; returns the folder and the paths.
    fn folder() -> (PathBuf, Vec<PathBuf>) {
        static N: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("tetron-wm-imageview-{}-{n}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let paths: Vec<PathBuf> = ["a.png", "b.png", "c.png"].iter().map(|f| dir.join(f)).collect();
        for (i, p) in paths.iter().enumerate() {
            RgbaImage::from_pixel(40 + i as u32, 20, image::Rgba([10, 200, 30, 255])).save(p).unwrap();
        }
        std::fs::write(dir.join("notes.txt"), "not an image").unwrap();
        (dir, paths)
    }

    fn poll_until(v: &mut ImageView, store: &mut ImageStore, done: impl Fn(&ImageView) -> bool) {
        let t0 = Instant::now();
        while !done(v) {
            assert!(t0.elapsed() < Duration::from_secs(10), "viewer did not finish");
            v.poll(30, 12, caps(false, false), true, store);
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    #[test]
    fn loads_renders_cells_and_steps_through_the_folder() {
        let (dir, paths) = folder();
        let mut store = ImageStore::new();
        let mut v = ImageView::new(paths[1].clone(), Method::Auto);
        poll_until(&mut v, &mut store, |v| v.cells.is_some());
        assert_eq!(v.dims, (41, 20));
        assert_eq!(v.title(), "image: b.png (2/3)", "only images count");
        let buf = v.render(30, 12);
        // chafa resamples, so the colour is near, not exact.
        let green = |c: Rgba| c.g > 180 && c.r < 40 && c.b < 60;
        let mid = *buf.get(15, 5).unwrap();
        assert!(green(mid.bg) || green(mid.fg), "image colours drawn: {mid:?}");

        assert!(v.placement(crate::geometry::Rect::new(0, 0, 30, 12)).is_none(), "no graphics without support");
        // At fit, Right is the next image; it wraps at the end.
        v.key(ImageKey::Right);
        assert_eq!(v.path(), paths[2]);
        v.key(ImageKey::Next);
        assert_eq!(v.path(), paths[0]);
        v.key(ImageKey::Prev);
        assert_eq!(v.path(), paths[2]);
        assert!(v.key(ImageKey::Close));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn zoomed_arrows_pan_and_fit_resets() {
        let (dir, paths) = folder();
        let mut store = ImageStore::new();
        let mut v = ImageView::new(paths[0].clone(), Method::Auto);
        poll_until(&mut v, &mut store, |v| v.img.is_some());
        v.key(ImageKey::ZoomIn);
        v.key(ImageKey::ZoomIn);
        assert!(v.zoom > 1.5);
        let cx = v.cx;
        v.key(ImageKey::Right);
        assert!(v.cx > cx, "zoomed Right pans");
        assert_eq!(v.path(), paths[0], "and does not change image");
        for _ in 0..50 {
            v.key(ImageKey::Right);
        }
        let edge = v.cx;
        v.key(ImageKey::Left);
        assert!(v.cx < edge, "panning back responds at once after the edge");
        for _ in 0..100 {
            v.key(ImageKey::ZoomIn);
        }
        assert_eq!(v.zoom, ZOOM_MAX);
        v.key(ImageKey::Fit);
        assert_eq!((v.zoom, v.cx, v.cy), (1.0, 0.5, 0.5));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn pdf_page_count_is_read_from_pdfinfo() {
        assert_eq!(pdf_pages("Title:   x\nPages:          12\nEncrypted: no\n"), Some(12));
        assert_eq!(pdf_pages("Syntax Error"), None);
        assert!(is_pdf(Path::new("/x/Report.PDF")) && !is_pdf(Path::new("/x/a.png")));
    }

    /// A tiny two-page PDF (hand-written, valid xref offsets computed here).
    fn two_page_pdf(path: &Path) {
        let objs = [
            "<< /Type /Catalog /Pages 2 0 R >>",
            "<< /Type /Pages /Kids [3 0 R 4 0 R] /Count 2 >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] >>",
        ];
        let mut out = String::from("%PDF-1.4\n");
        let mut offs = Vec::new();
        for (i, o) in objs.iter().enumerate() {
            offs.push(out.len());
            out.push_str(&format!("{} 0 obj\n{o}\nendobj\n", i + 1));
        }
        let xref = out.len();
        out.push_str(&format!("xref\n0 {}\n0000000000 65535 f \n", objs.len() + 1));
        for o in offs {
            out.push_str(&format!("{o:010} 00000 n \n"));
        }
        out.push_str(&format!("trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n", objs.len() + 1));
        std::fs::write(path, out).unwrap();
    }

    #[test]
    fn pdf_pages_render_and_n_p_turn_pages() {
        if !crate::catalog::is_installed("pdftoppm") {
            return; // poppler not installed here
        }
        let (dir, _) = folder();
        let pdf = dir.join("doc.pdf");
        two_page_pdf(&pdf);
        let mut store = ImageStore::new();
        let mut v = ImageView::new(pdf.clone(), Method::Auto);
        assert!(v.is_pdf());
        poll_until(&mut v, &mut store, |v| v.img.is_some() || v.error.is_some());
        assert!(v.error.is_none(), "{:?}", v.error);
        assert_eq!(v.doc, Some((1, 2)), "page 1 of 2");
        assert_eq!(v.title(), "pdf: doc.pdf (page 1/2)");
        v.key(ImageKey::Next);
        assert_eq!(v.doc.map(|d| d.0), Some(2));
        v.key(ImageKey::Next);
        assert_eq!(v.doc.map(|d| d.0), Some(2), "no wrap past the last page");
        v.key(ImageKey::Prev);
        assert_eq!(v.doc.map(|d| d.0), Some(1));
        assert_eq!(v.path(), pdf, "pages, not the folder's other files");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn fullscreen_list_starts_at_the_shown_image_and_wraps() {
        let all: Vec<PathBuf> = ["a.png", "b.png", "c.png"].iter().map(PathBuf::from).collect();
        assert_eq!(starting_at(&all, Path::new("b.png")), vec![PathBuf::from("b.png"), "c.png".into(), "a.png".into()]);
        assert_eq!(starting_at(&all, Path::new("x.png")), vec![PathBuf::from("x.png")]);
    }

    #[test]
    fn mouse_drag_pans_with_the_pointer_only_when_zoomed() {
        let (dir, paths) = folder();
        let mut store = ImageStore::new();
        let mut v = ImageView::new(paths[0].clone(), Method::Auto);
        poll_until(&mut v, &mut store, |v| v.img.is_some());
        v.poll(30, 12, caps(false, false), true, &mut store);
        v.drag_by(5, 0);
        assert_eq!(v.cx, 0.5, "at fit there is nothing to pan");
        v.key(ImageKey::ZoomIn);
        v.key(ImageKey::ZoomIn);
        v.poll(30, 12, caps(false, false), true, &mut store);
        let cx = v.cx;
        v.drag_by(3, 0);
        assert!(v.cx < cx, "dragging right shows more of the left side");
        v.drag_by(-6, 0);
        assert!(v.cx > cx, "dragging left shows more of the right side");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn graphics_placement_uses_the_fitted_rect_and_crop() {
        let (dir, paths) = folder();
        let mut store = ImageStore::new();
        let mut v = ImageView::new(paths[0].clone(), Method::Auto);
        let kitty = caps(true, false);
        let t0 = Instant::now();
        while v.png_id.is_none() {
            assert!(t0.elapsed() < Duration::from_secs(10));
            v.poll(30, 12, kitty, true, &mut store);
            std::thread::sleep(Duration::from_millis(10));
        }
        v.poll(30, 12, kitty, true, &mut store);
        let (id, rect, src) = v.placement(crate::geometry::Rect::new(5, 3, 30, 12)).unwrap();
        assert!(store.png_bytes(id).is_some());
        // 40x20 in 30x11 (one row is the info line): 30 wide, 8 tall.
        assert_eq!((rect.w, rect.h), (30, 8));
        assert_eq!((rect.x, rect.y), (5, 4));
        assert_eq!(src, [0, 0, 40, 20]);
        v.key(ImageKey::Mode);
        v.poll(30, 12, kitty, true, &mut store);
        assert!(v.placement(crate::geometry::Rect::new(5, 3, 30, 12)).is_none(), "cells mode drops graphics");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn undecodable_file_shows_an_error() {
        let (dir, _) = folder();
        let bad = dir.join("broken.png");
        std::fs::write(&bad, b"not a png").unwrap();
        let mut store = ImageStore::new();
        let mut v = ImageView::new(bad, Method::Auto);
        poll_until(&mut v, &mut store, |v| v.error.is_some());
        let buf = v.render(60, 5);
        let text: String = (0..60).map(|x| buf.get(x, 2).unwrap().ch).collect();
        assert!(text.contains("cannot display broken.png"), "{text}");
        let _ = std::fs::remove_dir_all(dir);
    }
}
