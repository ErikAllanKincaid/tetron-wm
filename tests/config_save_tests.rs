//! `Config::load` / `Config::save` against a real file: comments and hand edits
//! survive, nothing is written when nothing changed, and a file that does not
//! parse is never overwritten. One test function: it sets `XDG_CONFIG_HOME`, a
//! process-wide variable, so it owns this binary.

use tetron_wm::config::Config;

#[test]
fn save_preserves_comments_and_never_overwrites_a_broken_file() {
    let dir = std::env::temp_dir().join(format!("tetron-cfg-save-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let cfgdir = dir.join("tetron-wm");
    std::fs::create_dir_all(&cfgdir).unwrap();
    let path = cfgdir.join("config.toml");
    std::env::set_var("XDG_CONFIG_HOME", &dir);

    let original = "# my config\nsnap_threshold = 4  # tight\n\n#[[launcher]]\n#name = \"TTT\"\n";
    std::fs::write(&path, original).unwrap();

    // Loading and saving without a change leaves the file byte-for-byte alone.
    let mut c = Config::load();
    c.save().unwrap();
    assert_eq!(std::fs::read_to_string(&path).unwrap(), original);

    // A real change touches only that key; comments and the commented-out block stay.
    c.window_shadows = !c.window_shadows;
    c.save().unwrap();
    let out = std::fs::read_to_string(&path).unwrap();
    assert!(out.starts_with("# my config\nsnap_threshold = 4  # tight\n"), "{out}");
    assert!(out.contains("#[[launcher]]\n#name = \"TTT\""), "{out}");
    assert!(out.contains("window_shadows"));
    assert_eq!(Config::load().window_shadows, c.window_shadows);

    // A hand edit made while the process runs is not reverted by a later save.
    let edited = out.replace("snap_threshold = 4", "snap_threshold = 9");
    std::fs::write(&path, edited).unwrap();
    c.auto_tile = !c.auto_tile;
    c.save().unwrap();
    let out = std::fs::read_to_string(&path).unwrap();
    assert!(out.contains("snap_threshold = 9"), "{out}");
    assert!(out.contains("auto_tile"));

    // A file with a syntax error loads as defaults but is never overwritten.
    std::fs::write(&path, "snap_threshold = = 1\n").unwrap();
    let broken = Config::load();
    assert_eq!(broken.snap_threshold, Config::default().snap_threshold);
    let mut b = broken;
    b.window_shadows = !b.window_shadows;
    assert!(b.save().is_err());
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "snap_threshold = = 1\n");

    std::env::remove_var("XDG_CONFIG_HOME");
    let _ = std::fs::remove_dir_all(&dir);
}
