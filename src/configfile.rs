//! Persisting `config.toml` without destroying what the user wrote in it.
//!
//! The old save serialized the whole in-memory `Config` over the file, which
//! dropped comments, reordered keys, discarded unknown keys and reverted any hand
//! edit made while the daemon was running. This module instead tracks a
//! *baseline* per file (what this process last loaded or wrote), works out which
//! top-level keys the process actually changed since, re-reads the file from
//! disk, applies only those keys with `toml_edit` (so comments, order and every
//! untouched key survive), and writes atomically. It writes nothing when no key
//! changed, and it refuses to touch a file that does not parse.

use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use toml_edit::{DocumentMut, Item};

#[derive(Default)]
struct Tracked {
    /// The serialized config this process last loaded or wrote.
    baseline: Option<toml::Table>,
    /// A `.bak` copy has been made in this process.
    backed_up: bool,
}

fn tracked() -> &'static Mutex<HashMap<PathBuf, Tracked>> {
    static T: OnceLock<Mutex<HashMap<PathBuf, Tracked>>> = OnceLock::new();
    T.get_or_init(Mutex::default)
}

/// Remember `table` (the serialized config just loaded from `path`) as the
/// baseline for later changed-key detection.
pub fn note_loaded(path: &Path, table: toml::Table) {
    if let Ok(mut m) = tracked().lock() {
        m.entry(path.to_path_buf()).or_default().baseline = Some(table);
    }
}

/// Save `new` (the serialized live config) into `path`, touching only the
/// top-level keys that differ from this process's baseline. `default_table`
/// supplies the baseline when this process never loaded the file.
pub fn save(path: &Path, new: toml::Table, default_table: impl FnOnce() -> toml::Table) -> io::Result<()> {
    let mut map = tracked().lock().map_err(|_| io::Error::other("config tracker poisoned"))?;
    let t = map.entry(path.to_path_buf()).or_default();
    let base = match &t.baseline {
        Some(b) => b.clone(),
        None => default_table(),
    };
    if base == new {
        return Ok(()); // nothing changed in this process: do not even read the file
    }

    let existing = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) if e.kind() == io::ErrorKind::NotFound => String::new(),
        Err(e) => return Err(e),
    };
    let mut doc: DocumentMut = existing.parse().map_err(|e| {
        io::Error::other(format!("{} has a syntax error, not saving: {e}", path.display()))
    })?;
    apply_table(doc.as_table_mut(), &new, &base);
    let out = doc.to_string();
    if out != existing {
        if !t.backed_up && path.exists() {
            let mut bak = path.as_os_str().to_owned();
            bak.push(".bak");
            let _ = std::fs::copy(path, PathBuf::from(bak));
            t.backed_up = true;
        }
        write_atomic(path, &out)?;
    }
    t.baseline = Some(new);
    Ok(())
}

/// Write `text` to `path` through a temp file in the same directory, then rename.
pub fn write_atomic(path: &Path, text: &str) -> io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    let tmp = path.with_file_name(format!(".{name}.tmp{}", std::process::id()));
    std::fs::write(&tmp, text)?;
    std::fs::rename(&tmp, path).inspect_err(|_| {
        let _ = std::fs::remove_file(&tmp);
    })
}

/// Apply the differences between `base` and `new` to `tbl`, leaving every other
/// key (and all comments) alone.
fn apply_table(tbl: &mut toml_edit::Table, new: &toml::Table, base: &toml::Table) {
    for (k, v) in new {
        if base.get(k) != Some(v) {
            set_item(tbl, k, v, base.get(k));
        }
    }
    for k in base.keys() {
        if !new.contains_key(k) {
            tbl.remove(k);
        }
    }
}

fn set_item(tbl: &mut toml_edit::Table, k: &str, v: &toml::Value, base_v: Option<&toml::Value>) {
    // A table that exists on disk is merged key by key, so its other entries and
    // comments survive (even entries the baseline never knew about).
    if let (toml::Value::Table(nt), Some(Item::Table(sub))) = (v, tbl.get_mut(k)) {
        let empty = toml::Table::new();
        let bt = match base_v {
            Some(toml::Value::Table(b)) => b,
            _ => &empty,
        };
        apply_table(sub, nt, bt);
        return;
    }
    let Some(item) = item_for(k, v) else { return };
    match tbl.get_mut(k) {
        Some(old) => match (old.as_value(), &item) {
            // Keep the old value's decor so a trailing `# comment` stays.
            (Some(ov), Item::Value(nv)) => {
                let mut nv = nv.clone();
                *nv.decor_mut() = ov.decor().clone();
                *old = Item::Value(nv);
            }
            _ => *old = item,
        },
        None => {
            tbl.insert(k, item);
        }
    }
}

/// A `toml_edit` item for one key/value, built by serializing and re-parsing so
/// tables, arrays of tables and scalars all take their usual TOML shape.
fn item_for(k: &str, v: &toml::Value) -> Option<Item> {
    let mut t = toml::Table::new();
    t.insert(k.to_string(), v.clone());
    let text = toml::to_string_pretty(&t).ok()?;
    let doc: DocumentMut = text.parse().ok()?;
    doc.as_table().get(k).cloned()
}

/// Remove top-level `keys` from the file at `path` if present, preserving
/// everything else. Used to retire keys that moved elsewhere. A missing or
/// unparsable file is left alone.
pub fn remove_keys(path: &Path, keys: &[&str]) -> io::Result<()> {
    let Ok(existing) = std::fs::read_to_string(path) else { return Ok(()) };
    let Ok(mut doc) = existing.parse::<DocumentMut>() else { return Ok(()) };
    let mut changed = false;
    for k in keys {
        changed |= doc.as_table_mut().remove(k).is_some();
    }
    if changed {
        write_atomic(path, &doc.to_string())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    fn dir(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("tetron-cfgfile-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn table(c: &Config) -> toml::Table {
        toml::Table::try_from(c).unwrap()
    }

    /// Load `text` the way `Config::load` does and note the baseline.
    fn load(path: &Path, text: &str) -> Config {
        std::fs::write(path, text).unwrap();
        let c = Config::from_toml_str(text).unwrap();
        note_loaded(path, table(&c));
        c
    }

    const SAMPLE: &str = "\
# my notes about tetron-wm
snap_threshold = 5   # keep this small

# theme I like
theme = \"nord\"

#[[launcher]]
#name = \"old\"

[keys]
# open the launcher
launcher = \"l\"

[dock_badges]
claude = \"orange\"
";

    #[test]
    fn comments_order_and_other_keys_survive_a_save() {
        let d = dir("comments");
        let p = d.join("config.toml");
        let mut c = load(&p, SAMPLE);
        c.snap_threshold = 7;
        save(&p, table(&c), || table(&Config::default())).unwrap();
        let out = std::fs::read_to_string(&p).unwrap();
        assert_eq!(out, SAMPLE.replace("snap_threshold = 5 ", "snap_threshold = 7 "));
        assert!(out.contains("# keep this small") && out.contains("#[[launcher]]"));
    }

    #[test]
    fn nothing_changed_writes_and_backs_up_nothing() {
        let d = dir("noop");
        let p = d.join("config.toml");
        let c = load(&p, SAMPLE);
        let before = std::fs::metadata(&p).unwrap().modified().unwrap();
        save(&p, table(&c), || table(&Config::default())).unwrap();
        assert_eq!(std::fs::metadata(&p).unwrap().modified().unwrap(), before);
        assert!(!d.join("config.toml.bak").exists());
    }

    #[test]
    fn a_hand_edit_to_another_key_survives_the_daemon_saving() {
        let d = dir("handedit");
        let p = d.join("config.toml");
        let mut c = load(&p, SAMPLE);
        // The user edits the theme in the file while the daemon is running.
        let edited = std::fs::read_to_string(&p).unwrap().replace("\"nord\"", "\"dracula\"");
        std::fs::write(&p, edited).unwrap();
        // The daemon changes a different key and saves.
        c.snap_threshold = 9;
        save(&p, table(&c), || table(&Config::default())).unwrap();
        let out = std::fs::read_to_string(&p).unwrap();
        assert!(out.contains("theme = \"dracula\""), "hand edit kept: {out}");
        assert!(out.contains("snap_threshold = 9"));
    }

    #[test]
    fn a_key_reset_to_none_is_removed_and_its_neighbours_kept() {
        let d = dir("remove");
        let p = d.join("config.toml");
        let mut c = load(&p, "wallpaper = \"a.jpg\"\nsnap_threshold = 4\n");
        c.wallpaper = None;
        save(&p, table(&c), || table(&Config::default())).unwrap();
        assert_eq!(std::fs::read_to_string(&p).unwrap(), "snap_threshold = 4\n");
    }

    #[test]
    fn a_table_is_merged_keeping_other_entries_and_comments() {
        let d = dir("table");
        let p = d.join("config.toml");
        let mut c = load(&p, SAMPLE);
        c.keys.insert("spotlight".into(), crate::keymap::KeySpec::One("s".into()));
        save(&p, table(&c), || table(&Config::default())).unwrap();
        let out = std::fs::read_to_string(&p).unwrap();
        assert!(out.contains("# open the launcher\nlauncher = \"l\""), "{out}");
        assert!(out.contains("spotlight = \"s\""));
        assert!(out.contains("[dock_badges]\nclaude = \"orange\""));
        assert_eq!(Config::from_toml_str(&out).unwrap().keys.len(), 2);
    }

    #[test]
    fn arrays_of_tables_and_new_root_keys_round_trip() {
        let d = dir("roundtrip");
        let p = d.join("config.toml");
        let mut c = load(&p, SAMPLE);
        c.launcher.push(crate::config::AppEntry {
            name: "TTT".into(),
            command: "ttt".into(),
            args: vec![],
            category: Some("Editors".into()),
            requires_cwd: None,
            cwd: None,
            cli: None,
            warn: None,
        });
        c.natural_scroll = !c.natural_scroll; // a brand-new root key after the tables exist
        save(&p, table(&c), || table(&Config::default())).unwrap();
        let out = std::fs::read_to_string(&p).unwrap();
        let back = Config::from_toml_str(&out).unwrap();
        assert_eq!(back.launcher, c.launcher);
        assert_eq!(back.natural_scroll, c.natural_scroll);
        assert_eq!(back.snap_threshold, 5);
        assert!(out.contains("# theme I like"), "{out}");
    }

    #[test]
    fn a_file_with_a_syntax_error_is_never_overwritten() {
        let d = dir("broken");
        let p = d.join("config.toml");
        let mut c = Config::default();
        std::fs::write(&p, "snap_threshold = = 5\n").unwrap();
        c.window_shadows = !c.window_shadows;
        let err = save(&p, table(&c), || table(&Config::default())).unwrap_err();
        assert!(err.to_string().contains("syntax error"), "{err}");
        assert_eq!(std::fs::read_to_string(&p).unwrap(), "snap_threshold = = 5\n");
        // Once the user fixes the file, the pending change saves.
        std::fs::write(&p, "snap_threshold = 5\n").unwrap();
        save(&p, table(&c), || table(&Config::default())).unwrap();
        assert!(std::fs::read_to_string(&p).unwrap().contains("window_shadows"));
    }

    #[test]
    fn first_rewrite_makes_a_backup_and_no_temp_file_is_left() {
        let d = dir("backup");
        let p = d.join("config.toml");
        let mut c = load(&p, SAMPLE);
        c.snap_threshold = 6;
        save(&p, table(&c), || table(&Config::default())).unwrap();
        assert_eq!(std::fs::read_to_string(d.join("config.toml.bak")).unwrap(), SAMPLE);
        let stray: Vec<_> = std::fs::read_dir(&d)
            .unwrap()
            .flatten()
            .filter(|e| e.file_name().to_string_lossy().contains(".tmp"))
            .collect();
        assert!(stray.is_empty());
    }

    #[test]
    fn a_missing_file_is_created_with_only_the_changed_keys() {
        let d = dir("missing");
        let p = d.join("sub").join("config.toml");
        let c = Config { snap_threshold: 8, ..Config::default() };
        save(&p, table(&c), || table(&Config::default())).unwrap();
        assert_eq!(std::fs::read_to_string(&p).unwrap(), "snap_threshold = 8\n");
    }

    #[test]
    fn remove_keys_retires_only_the_named_keys() {
        let d = dir("retire");
        let p = d.join("config.toml");
        std::fs::write(&p, "recent_dirs = [\"/a\"]\n# mine\nsnap_threshold = 3\n").unwrap();
        remove_keys(&p, &["recent_dirs", "desktop_positions"]).unwrap();
        assert_eq!(std::fs::read_to_string(&p).unwrap(), "# mine\nsnap_threshold = 3\n");
    }
}
