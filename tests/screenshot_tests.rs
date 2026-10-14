use tetron_wm::buffer::CellBuffer;
use tetron_wm::cell::Rgba;
use tetron_wm::screenshot::{render, Psf};

#[test]
fn renders_text_and_blocks() {
    let Some(font) = Psf::load_system() else { return }; // font not installed: skip
    let mut b = CellBuffer::new(20, 3);
    b.write_str(0, 0, "tetron-wm ┌─┐ ▀▄█", Rgba::rgb(255, 255, 255), Rgba::rgb(0, 0, 80));
    let img = render(&b, &font);
    assert_eq!(img.width() as usize, 20 * font.w);
    assert_eq!(img.height() as usize, 3 * font.h);
    // the full block cell must be solid foreground
    let x = (16 * font.w + 1) as u32;
    assert_eq!(img.get_pixel(x, 1).0, [255, 255, 255]);
    if let Ok(p) = std::env::var("TETRON_SHOT_OUT") { img.save(p).unwrap(); }
}
