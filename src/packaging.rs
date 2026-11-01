//! Whether apt (dpkg) owns the running tetron-wm, so nothing in the app ever
//! writes over a packaged binary (WMDEB-005): Settings > Updates upgrades it
//! through the release `.deb` and apt, and the dev channel refuses.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// The Debian package name.
pub const PKG: &str = "tetron-wm";

/// dpkg's record of the files each package installed.
const DPKG_INFO: &str = "/var/lib/dpkg/info";

/// Whether a dpkg `.list` file (one installed path per line) names `path`.
pub fn list_owns(list: &str, path: &Path) -> bool {
    let want = path.to_string_lossy();
    list.lines().any(|l| l == want)
}

/// The `.list` files of the tetron-wm package in `info_dir`: `tetron-wm.list`,
/// or `tetron-wm:<arch>.list` for a multi-arch install.
fn package_lists(info_dir: &Path) -> Vec<PathBuf> {
    let plain = info_dir.join(format!("{PKG}.list"));
    if plain.is_file() {
        return vec![plain];
    }
    let prefix = format!("{PKG}:");
    std::fs::read_dir(info_dir)
        .map(|rd| {
            rd.flatten()
                .map(|e| e.path())
                .filter(|p| {
                    p.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.starts_with(&prefix) && n.ends_with(".list"))
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Whether the tetron-wm package (per the dpkg records in `info_dir`) owns
/// `path`, compared after resolving symlinks.
pub fn owned_in(info_dir: &Path, path: &Path) -> bool {
    let real = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    package_lists(info_dir)
        .iter()
        .filter_map(|l| std::fs::read_to_string(l).ok())
        .any(|text| list_owns(&text, &real) || list_owns(&text, path))
}

/// Whether apt installed the running binary. Checked once per process.
pub fn running_is_packaged() -> bool {
    static PACKAGED: OnceLock<bool> = OnceLock::new();
    *PACKAGED.get_or_init(|| {
        let owned = std::env::current_exe().is_ok_and(|exe| owned_in(Path::new(DPKG_INFO), &exe));
        if owned {
            crate::dbg_log("packaging: the running tetron-wm is owned by the tetron-wm package (apt)");
        }
        owned
    })
}

/// How much of the app set is installed (WMINST-005): the suite script's
/// sizes, the same as the tetron-wm / tetron-wm-desktop / tetron-wm-extras debs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Size {
    Minimal,
    Desktop,
    Extras,
}

impl Size {
    pub fn name(self) -> &'static str {
        match self {
            Size::Minimal => "minimal",
            Size::Desktop => "desktop",
            Size::Extras => "extras",
        }
    }

    fn parse(s: &str) -> Option<Size> {
        match s.trim() {
            "minimal" => Some(Size::Minimal),
            "desktop" => Some(Size::Desktop),
            "extras" => Some(Size::Extras),
            _ => None,
        }
    }
}

/// Whether dpkg has package `name` installed: its file list exists (dpkg
/// deletes it when the package is removed).
fn dpkg_has(info_dir: &Path, name: &str) -> bool {
    info_dir.join(format!("{name}.list")).is_file()
}

/// The last `key=value` for `key` in the suite script's state file.
pub fn state_value(state: &str, key: &str) -> Option<String> {
    let prefix = format!("{key}=");
    state.lines().rev().find_map(|l| l.strip_prefix(&prefix)).map(str::to_string)
}

/// The installed size: the metapackages dpkg has, else the suite script's
/// record, else minimal.
pub fn size_from(info_dir: &Path, state: &str) -> Size {
    if dpkg_has(info_dir, "tetron-wm-extras") {
        Size::Extras
    } else if dpkg_has(info_dir, "tetron-wm-desktop") {
        Size::Desktop
    } else {
        state_value(state, "size").and_then(|s| Size::parse(&s)).unwrap_or(Size::Minimal)
    }
}

/// How tetron-wm was installed, for Settings > About.
pub fn method_name(packaged: bool, exe: &Path) -> &'static str {
    if packaged {
        "apt package"
    } else if exe.parent().and_then(|d| d.parent()).and_then(|r| r.file_name()).is_some_and(|n| n == ".cargo") {
        "cargo"
    } else {
        "binary"
    }
}

fn suite_state_text() -> String {
    crate::config::config_dir()
        .and_then(|d| std::fs::read_to_string(d.join("suite-state")).ok())
        .unwrap_or_default()
}

/// The installed size on this machine (reads dpkg's records and the suite
/// state each call: cheap file checks, so the Settings row stays current).
pub fn installed_size() -> Size {
    size_from(Path::new(DPKG_INFO), &suite_state_text())
}

/// The install method of the running binary.
pub fn install_method() -> &'static str {
    let exe = std::env::current_exe().unwrap_or_default();
    method_name(running_is_packaged(), &exe)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("tetron-wm-packaging-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn list_matches_whole_lines_only() {
        let list = "/.\n/usr\n/usr/bin\n/usr/bin/tetron-wm\n/usr/share/doc/tetron-wm\n";
        assert!(list_owns(list, Path::new("/usr/bin/tetron-wm")));
        assert!(!list_owns(list, Path::new("/usr/local/bin/tetron-wm")));
        assert!(!list_owns(list, Path::new("/usr/bin/tetron")), "no prefix match");
    }

    #[test]
    fn owned_through_the_package_list_and_symlinks() {
        let d = tmp("owned");
        let (info, bin) = (d.join("info"), d.join("usr/bin"));
        std::fs::create_dir_all(&info).unwrap();
        std::fs::create_dir_all(&bin).unwrap();
        let exe = bin.join("tetron-wm");
        std::fs::write(&exe, "x").unwrap();
        let raw = d.join("raw-tetron-wm");
        std::fs::write(&raw, "x").unwrap();
        assert!(!owned_in(&info, &exe), "no package installed");
        std::fs::write(info.join("tetron-wm.list"), format!("/.\n{}\n", std::fs::canonicalize(&exe).unwrap().display())).unwrap();
        assert!(owned_in(&info, &exe));
        assert!(!owned_in(&info, &raw), "a copy elsewhere is not the package's");
        #[cfg(unix)]
        {
            let link = d.join("link");
            std::os::unix::fs::symlink(&exe, &link).unwrap();
            assert!(owned_in(&info, &link), "resolved through a symlink");
        }
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn multiarch_list_name_is_found() {
        let d = tmp("multiarch");
        std::fs::write(d.join("tetron-wm-desktop.list"), "/usr/bin/tetron-wm\n").unwrap();
        assert!(!owned_in(&d, Path::new("/usr/bin/tetron-wm")), "another package's list is not ours");
        std::fs::write(d.join("tetron-wm:amd64.list"), "/usr/bin/tetron-wm\n").unwrap();
        assert!(owned_in(&d, Path::new("/usr/bin/tetron-wm")));
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn size_prefers_dpkg_then_suite_state_then_minimal() {
        let d = tmp("size");
        assert_eq!(size_from(&d, ""), Size::Minimal);
        assert_eq!(size_from(&d, "vt=1\nsize=desktop\nsize=extras\n"), Size::Extras, "last value wins");
        assert_eq!(size_from(&d, "size=bogus\n"), Size::Minimal);
        std::fs::write(d.join("tetron-wm-desktop.list"), "/.\n").unwrap();
        assert_eq!(size_from(&d, "size=minimal\n"), Size::Desktop, "dpkg wins over the record");
        std::fs::write(d.join("tetron-wm-extras.list"), "/.\n").unwrap();
        assert_eq!(size_from(&d, ""), Size::Extras);
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn method_from_ownership_and_path() {
        assert_eq!(method_name(true, Path::new("/usr/bin/tetron-wm")), "apt package");
        assert_eq!(method_name(false, Path::new("/home/u/.cargo/bin/tetron-wm")), "cargo");
        assert_eq!(method_name(false, Path::new("/usr/local/bin/tetron-wm")), "binary");
        assert_eq!(method_name(false, Path::new("tetron-wm")), "binary");
    }
}
