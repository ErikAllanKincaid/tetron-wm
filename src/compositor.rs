use crate::buffer::CellBuffer;
use crate::cell::{Cell, Rgba};
use crate::geometry::{Point, Rect};

/// A single renderable plane: a `CellBuffer` positioned at `origin` in screen space,
/// with a z-index for stacking order, an opacity modifier, and an optional scissor rect
/// that clips which screen cells the layer can paint.
pub struct Layer {
    /// Stacking order — lower z is rendered first (behind higher z).
    pub z: i32,
    /// Top-left corner of the layer in screen (compositor) coordinates.
    pub origin: Point,
    /// Source cell grid for this layer.
    pub buf: CellBuffer,
    /// Uniform opacity applied to the layer's `bg` alpha before blending (0.0–1.0).
    pub opacity: f32,
    /// If `Some`, only screen cells inside the rect are painted by this layer.
    pub scissor: Option<Rect>,
}

/// A single cell that changed between the last committed frame and the current composite.
/// Used by the terminal backend to emit only the minimal ANSI update.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CellChange {
    /// Screen column.
    pub x: i32,
    /// Screen row.
    pub y: i32,
    /// New cell value to write.
    pub cell: Cell,
}


/// Double-buffered compositor: composites z-ordered [`Layer`]s (with alpha blending),
/// overlays a block cursor, and diffs successive frames into a minimal [`CellChange`] list.
///
/// # Lifecycle
/// 1. Call [`composite`](Compositor::composite) to produce the new back buffer.
/// 2. Call [`diff`](Compositor::diff) to get only the cells that changed.
/// 3. Write those changes to the terminal.
/// 4. Call [`commit`](Compositor::commit) to promote back → front.
///
/// This type has no knowledge of PTYs, terminals, or windows — it only knows layers and cells.
pub struct Compositor {
    w: i32,
    h: i32,
    /// Last committed frame — what the physical terminal currently shows.
    front: CellBuffer,
    /// Most recent composite result — not yet committed to the terminal.
    back: CellBuffer,
}

impl Compositor {
    /// Create a compositor for a screen of `w` × `h` cells.
    pub fn new(w: i32, h: i32) -> Self {
        Self {
            w,
            h,
            front: CellBuffer::new(w, h),
            back: CellBuffer::new(w, h),
        }
    }

    /// Resize the compositor, discarding both buffers (caller must re-composite).
    pub fn resize(&mut self, w: i32, h: i32) {
        self.w = w;
        self.h = h;
        self.front = CellBuffer::new(w, h);
        self.back = CellBuffer::new(w, h);
    }

    /// Screen width in cells.
    pub fn width(&self) -> i32 { self.w }

    /// Screen height in cells.
    pub fn height(&self) -> i32 { self.h }

    /// Composite `layers` (sorted by z here — caller need not pre-sort) onto the back buffer,
    /// then overlay the cursor at `cursor` (if any), see [`cursor_cell`].
    ///
    /// Returns a shared reference to the resulting back buffer.
    pub fn composite(&mut self, layers: &[Layer], cursor: Option<Point>) -> &CellBuffer {
        let t = crate::theme::current();
        // Fill back buffer with the desktop background.
        let base = Cell {
            ch: ' ',
            fg: Rgba::rgb(90, 100, 120),
            bg: t.desktop_bg,
            attrs: Default::default(),
        };
        self.back.fill(base);

        // Sort layers by z (lowest first = rendered underneath).
        let mut order: Vec<&Layer> = layers.iter().collect();
        order.sort_by_key(|l| l.z);

        for layer in order {
            // Clip the layer to the screen and its scissor once, as layer-local
            // ranges, instead of testing every cell.
            let (ox, oy) = (layer.origin.x, layer.origin.y);
            let (mut x0, mut x1) = (0.max(-ox), layer.buf.width().min(self.w - ox));
            let (mut y0, mut y1) = (0.max(-oy), layer.buf.height().min(self.h - oy));
            if let Some(s) = layer.scissor {
                x0 = x0.max(s.x - ox);
                x1 = x1.min(s.x + s.w - ox);
                y0 = y0.max(s.y - oy);
                y1 = y1.min(s.y + s.h - oy);
            }
            if x0 >= x1 || y0 >= y1 {
                continue;
            }
            // `with_opacity` per alpha value, hoisted out of the cell loop (it
            // rounds a float); identity when the layer is fully opaque.
            let lut: Option<[u8; 256]> = (layer.opacity != 1.0).then(|| {
                std::array::from_fn(|a| Rgba::new(0, 0, 0, a as u8).with_opacity(layer.opacity).a)
            });
            let (gx0, gx1) = ((ox + x0) as usize, (ox + x1) as usize);
            for ly in y0..y1 {
                let src = &layer.buf.row(ly)[x0 as usize..x1 as usize];
                let dst = &mut self.back.row_mut(oy + ly)[gx0..gx1];
                for (d, s) in dst.iter_mut().zip(src) {
                    let mut src_bg = s.bg;
                    if let Some(lut) = &lut {
                        src_bg.a = lut[src_bg.a as usize];
                    }
                    *d = blend_with(*s, src_bg, *d);
                }
            }
        }

        // Double-width glyphs (emoji, CJK) occupy two columns, but the buffer is
        // one cell per column. Mark the cell to the right of each wide glyph as a
        // continuation (`\0`) so the renderer skips it — otherwise painting that
        // column overwrites and erases the glyph's right half (it then only shows
        // when the cursor happens to invalidate just the glyph cell).
        for y in 0..self.h {
            let row = self.back.row_mut(y);
            for x in 1..row.len() {
                if is_wide(row[x - 1].ch) {
                    row[x].ch = '\0';
                }
            }
        }

        // Overlay the mouse cursor.
        if let Some(p) = cursor {
            if let Some(c) = self.back.get(p.x, p.y) {
                self.back.set(p.x, p.y, cursor_cell(*c));
            }
        }

        &self.back
    }

    /// Return every cell that differs between the current back buffer and the last committed
    /// front buffer. Feed this to the terminal backend to minimise ANSI output.
    pub fn diff(&self) -> Vec<CellChange> {
        let mut out = Vec::new();
        for y in 0..self.h {
            let (b, f) = (self.back.row(y), self.front.row(y));
            if b == f {
                continue;
            }
            for (x, (b, f)) in b.iter().zip(f).enumerate() {
                if b != f {
                    out.push(CellChange { x: x as i32, y, cell: *b });
                }
            }
        }
        out
    }

    /// Promote the back buffer to the front (mark it as "what the terminal shows").
    /// Call this after the terminal backend has written all changes from [`diff`](Self::diff).
    /// Swaps instead of copying: the old front becomes the next back buffer,
    /// which [`composite`](Self::composite) refills completely, so call
    /// `composite` before the next `diff`.
    pub fn commit(&mut self) {
        std::mem::swap(&mut self.front, &mut self.back);
    }
}

/// Whether `c` renders as a double-width (two-column) glyph — emoji and CJK. A
/// pragmatic range check covering the icon emoji we use plus the common wide
/// scripts; good enough for the cell grid (full Unicode width tables aren't
/// needed here).
fn is_wide(c: char) -> bool {
    let c = c as u32;
    // Fast path: everything below Hangul Jamo (ASCII, Latin, box drawing) is narrow.
    c >= 0x1100 && matches!(c,
        0x1100..=0x115F        // Hangul Jamo
        | 0x2E80..=0xA4CF      // CJK radicals … Yi
        | 0xAC00..=0xD7A3      // Hangul syllables
        | 0xF900..=0xFAFF      // CJK compatibility ideographs
        | 0xFE30..=0xFE4F      // CJK compatibility forms
        | 0xFF00..=0xFF60      // fullwidth forms
        | 0xFFE0..=0xFFE6
        | 0x1F300..=0x1FAFF    // emoji, symbols & pictographs
        | 0x20000..=0x3FFFD    // CJK extension B+
    )
}

/// Composite a single source cell `src` over a destination cell `dst`, modulating
/// `src.bg` alpha by `opacity` before blending.
///
/// Three cases:
/// 1. `src` has a glyph → ink it over the blended background.
/// 2. `src` is blank with an **opaque** background → a solid space that *covers*
///    whatever was beneath (a normal window's empty area must hide lower windows).
/// 3. `src` is blank with a **translucent** background → a shadow/tint: blend the
///    background but keep the glyph from below showing through.
#[cfg(test)]
fn blend_cell(src: Cell, dst: Cell, opacity: f32) -> Cell {
    blend_with(src, src.bg.with_opacity(opacity), dst)
}

/// [`blend_cell`] with `src.bg` already scaled by the layer opacity.
#[inline]
fn blend_with(src: Cell, src_bg: Rgba, dst: Cell) -> Cell {
    let out_bg = src_bg.over(dst.bg);
    if src.ch != ' ' {
        Cell {
            ch: src.ch,
            fg: src.fg.over(out_bg),
            bg: out_bg,
            attrs: src.attrs,
        }
    } else if src_bg.a == 255 {
        // Opaque blank cell: clears the content beneath it.
        Cell {
            ch: ' ',
            fg: src.fg,
            bg: out_bg,
            attrs: src.attrs,
        }
    } else {
        // Translucent blank cell (shadow / tint): keep the lower glyph.
        Cell {
            ch: dst.ch,
            fg: dst.fg,
            bg: out_bg,
            attrs: dst.attrs,
        }
    }
}

/// The mouse cursor drawn over cell `c`: a solid white cell on a dark
/// background, black on a light one, keeping the character under it.
///
/// It used to invert the cell (swap fg and bg). That vanishes whenever the two
/// colours are close, on block or shade glyphs (swapping the halves of `▀`
/// draws the same picture: chafa wallpaper, carbonyl, image art), and on a
/// kernel VT, which has no bright backgrounds, so inverted bright text became
/// black on black (USER tests on a bare VT, 16 colours). An explicit
/// black/white cell shows at every colour depth, mono included.
pub fn cursor_cell(c: Cell) -> Cell {
    use crate::theme::luminance;
    // A transparent bg composites over black.
    let bg = if c.bg.a == 0 { Rgba::rgb(0, 0, 0) } else { c.bg };
    let (white, black) = (Rgba::rgb(255, 255, 255), Rgba::rgb(0, 0, 0));
    let (fg, bg) = if luminance(bg) < 0.3 { (black, white) } else { (white, black) };
    // A block glyph would paint over the cursor colour with its own shape.
    let ch = if matches!(c.ch as u32, 0x2580..=0x259F) { ' ' } else { c.ch };
    Cell { ch, fg, bg, attrs: crate::cell::CellAttrs::default() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::buffer::CellBuffer;
    use crate::cell::{Cell, Rgba};
    use crate::geometry::Point;

    fn wide_layer() -> Layer {
        let mut buf = CellBuffer::new(4, 1);
        // A double-width emoji at (0,0); the rest are blank spaces.
        buf.set(0, 0, Cell { ch: '\u{1F4C1}', fg: Rgba::rgb(255, 255, 255), bg: Rgba::TRANSPARENT, attrs: Default::default() });
        Layer { z: 0, origin: Point::new(0, 0), buf, opacity: 1.0, scissor: None }
    }

    /// The original per-cell composite, kept as the reference the fast path
    /// must match exactly.
    fn reference_composite(w: i32, h: i32, layers: &[Layer], cursor: Option<Point>) -> CellBuffer {
        let t = crate::theme::current();
        let mut back = CellBuffer::new(w, h);
        back.fill(Cell { ch: ' ', fg: Rgba::rgb(90, 100, 120), bg: t.desktop_bg, attrs: Default::default() });
        let mut order: Vec<&Layer> = layers.iter().collect();
        order.sort_by_key(|l| l.z);
        for layer in order {
            for ly in 0..layer.buf.height() {
                for lx in 0..layer.buf.width() {
                    let (gx, gy) = (layer.origin.x + lx, layer.origin.y + ly);
                    if gx < 0 || gy < 0 || gx >= w || gy >= h { continue; }
                    if let Some(s) = layer.scissor {
                        if !s.contains(Point::new(gx, gy)) { continue; }
                    }
                    let src = *layer.buf.get(lx, ly).unwrap();
                    let dst = *back.get(gx, gy).unwrap();
                    back.set(gx, gy, blend_cell(src, dst, layer.opacity));
                }
            }
        }
        for y in 0..h {
            for x in 0..w - 1 {
                if is_wide(back.get(x, y).unwrap().ch) {
                    let mut cont = *back.get(x + 1, y).unwrap();
                    cont.ch = '\0';
                    back.set(x + 1, y, cont);
                }
            }
        }
        if let Some(p) = cursor {
            if let Some(c) = back.get(p.x, p.y) {
                back.set(p.x, p.y, cursor_cell(*c));
            }
        }
        back
    }

    /// Small xorshift so the test needs no dependency and is reproducible.
    struct Rng(u64);
    impl Rng {
        fn next(&mut self) -> u64 { self.0 ^= self.0 << 13; self.0 ^= self.0 >> 7; self.0 ^= self.0 << 17; self.0 }
        fn range(&mut self, lo: i32, hi: i32) -> i32 { lo + (self.next() % (hi - lo + 1) as u64) as i32 }
        fn u8(&mut self) -> u8 { self.next() as u8 }
        fn alpha(&mut self) -> u8 { match self.next() % 4 { 0 => 0, 1 => 255, _ => self.u8() } }
    }

    fn random_layer(r: &mut Rng, w: i32, h: i32) -> Layer {
        let (lw, lh) = (r.range(0, w + 4), r.range(0, h + 4));
        let mut buf = CellBuffer::new(lw, lh);
        const CHARS: [char; 8] = [' ', 'a', '\u{1F4C1}', '\u{4E2D}', '\u{2580}', '\0', '\u{1100}', 'Z'];
        for y in 0..lh {
            for x in 0..lw {
                let ch = CHARS[(r.next() % 8) as usize];
                let fg = Rgba::new(r.u8(), r.u8(), r.u8(), r.alpha());
                let bg = Rgba::new(r.u8(), r.u8(), r.u8(), r.alpha());
                let attrs = crate::cell::CellAttrs { bold: r.next().is_multiple_of(2), italic: false, underline: r.next().is_multiple_of(3), inverse: false };
                buf.set(x, y, Cell { ch, fg, bg, attrs });
            }
        }
        let opacity = match r.next() % 4 { 0 | 1 => 1.0, 2 => 0.0, _ => (r.next() % 1001) as f32 / 1000.0 };
        let scissor = (r.next().is_multiple_of(2)).then(|| crate::geometry::Rect::new(r.range(-3, w), r.range(-3, h), r.range(-2, w + 3), r.range(-2, h + 3)));
        Layer { z: r.range(-2, 2), origin: Point::new(r.range(-lw - 2, w + 2), r.range(-lh - 2, h + 2)), buf, opacity, scissor }
    }

    #[test]
    fn fast_composite_matches_reference() {
        let mut r = Rng(0x9E3779B97F4A7C15);
        for _ in 0..3000 {
            let (w, h) = (r.range(0, 24), r.range(0, 10));
            let layers: Vec<Layer> = (0..r.range(0, 5)).map(|_| random_layer(&mut r, w, h)).collect();
            let cursor = (r.next().is_multiple_of(2)).then(|| Point::new(r.range(-1, w), r.range(-1, h)));
            let mut comp = Compositor::new(w, h);
            // Dirty both buffers first: the swap in commit must not leak old cells.
            let junk = random_layer(&mut r, w, h);
            comp.composite(std::slice::from_ref(&junk), None);
            comp.commit();
            comp.composite(&[junk], cursor);
            let got = comp.composite(&layers, cursor).clone();
            assert_eq!(got, reference_composite(w, h, &layers, cursor));
        }
    }

    #[test]
    fn diff_after_commit_matches_full_compare() {
        let mut r = Rng(42);
        let (w, h) = (20, 8);
        let mut comp = Compositor::new(w, h);
        let mut prev = CellBuffer::new(w, h);
        for _ in 0..300 {
            let layers: Vec<Layer> = (0..r.range(0, 4)).map(|_| random_layer(&mut r, w, h)).collect();
            let back = comp.composite(&layers, None).clone();
            let want: Vec<CellChange> = (0..h).flat_map(|y| (0..w).map(move |x| (x, y)))
                .filter(|&(x, y)| back.get(x, y) != prev.get(x, y))
                .map(|(x, y)| CellChange { x, y, cell: *back.get(x, y).unwrap() }).collect();
            assert_eq!(comp.diff(), want);
            comp.commit();
            prev = back;
        }
    }

    #[test]
    fn with_opacity_lut_matches_per_cell() {
        for o in (0..=1000).map(|i| i as f32 / 1000.0).chain([1.5, -0.5, f32::NAN]) {
            let lut: [u8; 256] = std::array::from_fn(|a| Rgba::new(0, 0, 0, a as u8).with_opacity(o).a);
            for a in 0..=255u8 {
                assert_eq!(lut[a as usize], Rgba::new(1, 2, 3, a).with_opacity(o).a);
            }
        }
    }

    /// `cargo test --release -- --ignored composite_timing --nocapture`
    #[test]
    #[ignore]
    fn composite_timing() {
        let (w, h) = (200, 50);
        let mut r = Rng(7);
        let full = |r: &mut Rng, x, y, lw, lh, opacity| {
            let mut l = random_layer(r, 1, 1);
            l.buf = CellBuffer::new(lw, lh);
            for yy in 0..lh { l.buf.write_str(0, yy, &"x".repeat(lw as usize), Rgba::rgb(200, 200, 200), Rgba::rgb(20, 20, 30)); }
            l.origin = Point::new(x, y); l.opacity = opacity; l.scissor = None; l
        };
        // Wallpaper, two windows, a translucent shadow: the shape of a desktop frame.
        let layers = vec![full(&mut r, 0, 0, w, h, 1.0), full(&mut r, 5, 3, 120, 35, 1.0), full(&mut r, 60, 10, 120, 35, 1.0), full(&mut r, 61, 11, 120, 35, 0.5)];
        let n = 2000;
        let t = std::time::Instant::now();
        for _ in 0..n { std::hint::black_box(reference_composite(w, h, &layers, Some(Point::new(3, 3)))); }
        let old = t.elapsed();
        let mut comp = Compositor::new(w, h);
        let t = std::time::Instant::now();
        for _ in 0..n { comp.composite(&layers, Some(Point::new(3, 3))); std::hint::black_box(comp.diff()); comp.commit(); }
        let new = t.elapsed();
        println!("per frame: reference {:?} (composite only), fast {:?} (composite+diff+commit)", old / n, new / n);
    }

    #[test]
    fn wide_glyph_marks_next_cell_as_continuation() {
        let mut comp = Compositor::new(4, 1);
        let back = comp.composite(&[wide_layer()], None);
        assert_eq!(back.get(0, 0).unwrap().ch, '\u{1F4C1}', "wide glyph kept");
        assert_eq!(back.get(1, 0).unwrap().ch, '\0', "right half marked as continuation");
    }

    #[test]
    fn renderer_skips_continuation_cells() {
        // A continuation change must not emit cursor movement or output.
        let change = CellChange {
            x: 5,
            y: 2,
            cell: Cell { ch: '\0', fg: Rgba::rgb(1, 2, 3), bg: Rgba::TRANSPARENT, attrs: Default::default() },
        };
        let caps = crate::terminal::Caps { depth: crate::display::ColorDepth::TrueColor, vt: false, pixel_mouse: true, kitty_graphics: true };
        let out = crate::terminal::frame_to_ansi(&[change], &caps);
        assert!(out.is_empty() || !out.contains("\u{1b}[3;6H"), "continuation cell should not be painted");
    }
}
