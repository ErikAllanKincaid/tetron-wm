use tetron_wm::imageview::{fit, ImageView, Method};

#[test]
fn a_new_view_shows_loading_until_decoded() {
    let v = ImageView::new("/nonexistent/cat.png", Method::Auto);
    let buf = v.render(30, 8);
    let text: String = (0..buf.height()).flat_map(|y| (0..buf.width()).map(move |x| (x, y))).map(|(x, y)| buf.get(x, y).unwrap().ch).collect();
    assert!(text.contains("cat.png"), "{text}");
    assert_eq!(v.title(), "image: cat.png");
}

#[test]
fn fit_is_public_and_centres_a_tall_image() {
    let f = fit(10, 100, 40, 10, 1.0, 0.5, 0.5).unwrap();
    assert_eq!((f.dw, f.dh), (2, 10));
    assert_eq!(f.dx, 19);
}
