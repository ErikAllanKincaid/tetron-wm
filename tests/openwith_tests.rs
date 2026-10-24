use std::collections::BTreeMap;
use std::path::Path;
use tetron_wm::openwith::{classify, resolve, OpenAction, Role};

fn map(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
}

#[test]
fn resolve_routes_by_handler() {
    let m = map(&[("image", "@image"), ("text", "vi"), ("directory", "@navigate")]);
    assert_eq!(resolve(Path::new("/x/cat.png"), false, &m), OpenAction::Builtin("@image"));
    assert_eq!(resolve(Path::new("/x/folder"), true, &m), OpenAction::Navigate);
    assert_eq!(
        resolve(Path::new("/x/notes.md"), false, &m),
        OpenAction::RunApp { command: "vi".into(), args: vec!["/x/notes.md".into()] }
    );
}

#[test]
fn unknown_or_unset_handler_is_menu() {
    let m = map(&[]); // nothing configured
    assert_eq!(resolve(Path::new("/x/mystery"), false, &m), OpenAction::OpenWithMenu);
}

#[test]
fn classifies_by_extension() {
    assert_eq!(classify(Path::new("/x/cat.png"), false), Role::Image);
    assert_eq!(classify(Path::new("/x/a.JPG"), false), Role::Image); // case-insensitive
    assert_eq!(classify(Path::new("/x/notes.md"), false), Role::Text);
    assert_eq!(classify(Path::new("/x/main.rs"), false), Role::Code);
    assert_eq!(classify(Path::new("/x/song.mp3"), false), Role::Audio);
    assert_eq!(classify(Path::new("/x/clip.mp4"), false), Role::Video);
    assert_eq!(classify(Path::new("/x/pack.zip"), false), Role::Archive);
    assert_eq!(classify(Path::new("/x/doc.pdf"), false), Role::Pdf);
}

#[test]
fn directory_and_unknown() {
    assert_eq!(classify(Path::new("/x/folder"), true), Role::Directory);
    assert_eq!(classify(Path::new("/x/mystery"), false), Role::Other);
}

#[test]
fn fresh_keeps_its_tab_list() {
    use tetron_wm::openwith::{open_with, OpenAction};
    let p = std::path::Path::new("/x/notes.txt");
    assert_eq!(
        open_with("fresh", p),
        OpenAction::RunApp { command: "fresh".into(), args: vec!["--restore".into(), "/x/notes.txt".into()] }
    );
    assert_eq!(
        open_with("/usr/bin/fresh --no-restore", p),
        OpenAction::RunApp { command: "/usr/bin/fresh".into(), args: vec!["--no-restore".into(), "/x/notes.txt".into()] },
        "an explicit choice is kept"
    );
    assert_eq!(
        open_with("nvim", p),
        OpenAction::RunApp { command: "nvim".into(), args: vec!["/x/notes.txt".into()] }
    );
}

#[test]
fn pdf_handlers() {
    use tetron_wm::openwith::{open_with, OpenAction};
    let p = std::path::Path::new("/x/a.pdf");
    assert_eq!(open_with("@pdf", p), OpenAction::Builtin("@image"), "the built-in viewer shows PDFs");
    match open_with("@pdftext", p) {
        OpenAction::RunApp { command, args } => {
            assert_eq!(command, "sh");
            assert!(args[1].contains("pdftotext -layout") && args.last().unwrap() == "/x/a.pdf", "{args:?}");
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn casttui_gets_the_media_file_as_its_argument() {
    use tetron_wm::openwith::{open_with, OpenAction};
    let p = std::path::Path::new("/m/My Clip.mp4");
    assert_eq!(
        open_with("casttui", p),
        OpenAction::RunApp { command: "casttui".into(), args: vec!["/m/My Clip.mp4".into()] },
        "the whole path stays one argument, spaces and all"
    );
}

#[test]
fn configured_extras_join_the_open_with_list() {
    use tetron_wm::openwith::with_extra;
    let base = vec![String::new(), "mpv --vo=tct".to_string()];
    let extra = vec!["casttui".to_string(), "  ".into(), "mpv --vo=tct".into(), " vlc ".into(), "casttui".into()];
    assert_eq!(
        with_extra(base.clone(), &extra),
        vec!["".to_string(), "mpv --vo=tct".into(), "casttui".into(), "vlc".into()],
        "kept in order after the detected ones; blanks and repeats dropped; trimmed"
    );
    assert_eq!(with_extra(base.clone(), &[]), base, "no extras changes nothing");
}
