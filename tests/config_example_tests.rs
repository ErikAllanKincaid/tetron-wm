use tetron_wm::config::Config;

/// config.example.toml must parse and, apart from the explicit default tables,
/// equal the built-in defaults, so it can be dropped in as is.
#[test]
fn example_config_matches_defaults() {
    let mut c = Config::from_toml_str(include_str!("../config.example.toml")).expect("example parses");
    let d = Config::default();
    assert!(!c.keys.is_empty() && !c.tray_placement.is_empty());
    c.keys.clear();
    c.tray_placement.clear();
    c.default_apps = d.default_apps.clone();
    assert_eq!(c, d);
}

/// Every config key in `--help` must be documented in docs/CONFIG.md and shown in
/// config.example.toml, so the three can not drift apart.
#[test]
fn config_docs_cover_every_key() {
    let doc = include_str!("../docs/CONFIG.md");
    let example = include_str!("../config.example.toml");
    for (key, _, _) in tetron_wm::cli_help::CONFIG_KEYS {
        assert!(doc.contains(&format!("| `{key}` |")), "docs/CONFIG.md is missing {key}");
        // Legacy keys are app-managed (state.toml), so the example omits them.
        if !matches!(*key, "recent_dirs" | "desktop_positions") {
            assert!(example.contains(key), "config.example.toml is missing {key}");
        }
    }
}
