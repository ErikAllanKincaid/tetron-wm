use tetron_wm::config::Config;

#[test]
fn desktop_scrim_color_resolves() {
    use tetron_wm::cell::Rgba;
    // Unset → the default semi-transparent black.
    let c = Config::default();
    assert_eq!(c.desktop_scrim_color(), Rgba { r: 0, g: 0, b: 0, a: 200 });
    // "none"/"off" → fully transparent (no scrim).
    let c = Config { desktop_scrim: Some("none".into()), ..Config::default() };
    assert_eq!(c.desktop_scrim_color(), Rgba::TRANSPARENT);
    let c = Config { desktop_scrim: Some("OFF".into()), ..Config::default() };
    assert_eq!(c.desktop_scrim_color(), Rgba::TRANSPARENT);
    // A color string → that color (alpha honored).
    let c = Config { desktop_scrim: Some("#112233aa".into()), ..Config::default() };
    assert_eq!(c.desktop_scrim_color(), Rgba { r: 0x11, g: 0x22, b: 0x33, a: 0xaa });
    // Garbage → falls back to the default.
    let c = Config { desktop_scrim: Some("not-a-color".into()), ..Config::default() };
    assert_eq!(c.desktop_scrim_color(), Rgba { r: 0, g: 0, b: 0, a: 200 });
}

#[test]
fn defaults_are_sane() {
    let c = Config::default();
    // tetron-wm defaults snapping OFF (docs/CONFIG.md: snapping + window shadows
    // off, theme nord).
    assert!(!c.snapping_enabled);
    assert_eq!(c.snap_threshold, 3);
    // Nothing auto-starts and nothing is pinned by default; the desktop shows
    // only the real contents of ~/Desktop.
    assert!(c.apps.is_empty());
    assert!(c.desktop_pins.is_empty());
}

#[test]
fn parses_toml_overrides() {
    let toml = r#"
snapping_enabled = false
snap_threshold = 5
[[apps]]
name = "shell"
command = "bash"
"#;
    let c = Config::from_toml_str(toml).unwrap();
    assert!(!c.snapping_enabled);
    assert_eq!(c.snap_threshold, 5);
    assert_eq!(c.apps.len(), 1);
    assert_eq!(c.apps[0].command, "bash");
}

#[test]
fn save_and_load_roundtrip() {
    let dir = std::env::temp_dir().join(format!("tetron-wm-cfg-{}", std::process::id()));
    std::env::set_var("XDG_CONFIG_HOME", &dir);
    let c = Config {
        snapping_enabled: false,
        window_shadows: false,
        snap_threshold: 7,
        ..Config::default()
    };
    c.save().unwrap();
    let loaded = Config::load();
    assert!(!loaded.snapping_enabled);
    assert!(!loaded.window_shadows);
    assert_eq!(loaded.snap_threshold, 7);
    std::env::remove_var("XDG_CONFIG_HOME");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn config_defaults_grid_2x2() {
    let c = Config::default();
    assert_eq!(c.grid_rows, 2);
    assert_eq!(c.grid_cols, 2);
    assert_eq!(c.tile_gap, 0);
    assert!(!c.auto_tile);
}

#[test]
fn default_apps_has_builtin_image_handler() {
    let c = Config::default();
    assert_eq!(c.default_apps.get("image").map(String::as_str), Some("@image"));
    assert_eq!(c.default_apps.get("directory").map(String::as_str), Some("@navigate"));
}

#[test]
fn desktop_has_no_default_pins() {
    // The desktop shows only the real contents of ~/Desktop; pins are opt-in.
    let c = Config::default();
    assert!(c.desktop_enabled);
    assert!(c.desktop_pins.is_empty());
    assert!(c.desktop_positions.is_empty());
}
