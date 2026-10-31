"""Packaging requirements for tetron-wm (WMDEB-*). Plan:
DO-NOT-COMMIT/PLAN_tetron-wm_deb_packaging_dpkg_aware_updater_theme_fallback_and_desktop_metapackage.md
Only packaging lives here; the rest of tetron-wm follows CLAUDE.md."""
from libspec import Requirement


class MissingToolAudit(Requirement):
    """REQUIREMENT-ID: WMDEB-001

    Every outside program tetron-wm runs is checked before use or fails
    visibly: no crash, no hang of the render loop, no menu item that fails
    silently. Every spawn goes through `run_capped`/`run_capped_status` or
    runs off the render loop. Audit: DO-NOT-COMMIT/AUDIT_tetron-wm_missing_tool_behavior_2026-10-31.md.
    Done 2026-10-31.
    """


class ThemeSearchList(Requirement):
    """REQUIREMENT-ID: WMDEB-002

    Theme files are looked up in an ordered list of directories; a name in an
    earlier directory wins:
    1. `$TETRON_WM_THEMES_DIR` when set, and then only that directory;
    2. `theme_dir` from config.toml when set, else `<config dir>/themes`
       (`~/.config/tetron-wm/themes`);
    3. `$XDG_DATA_HOME/tetron-wm/themes` (default
       `~/.local/share/tetron-wm/themes`, where in-app downloads go);
    4. each `$XDG_DATA_DIRS` entry (default `/usr/local/share:/usr/share`)
       plus `tetron-wm/themes`, for a later themes package.
    Missing directories are skipped. A theme's relative wallpaper path
    resolves against the directory the theme came from. The 4 built-in
    presets need no file. The example config is compiled into the binary;
    `tetron-wm config init` writes it to the config path only when no file
    is there, and says so either way.
    """


class SettingsRework(Requirement):
    """REQUIREMENT-ID: WMDEB-003

    Settings opens on Appearance. General > Config file (open, create from
    the example, reset with a confirm and a .bak, reload). Appearance > Get
    more themes lists `themes/` on main through the GitHub contents API and
    downloads missing .toml/.jpg/.jpeg/.png/.webp into the data dir, off the
    render loop, with a clear message on a 403 rate limit.
    """


class GitStyleHelpers(Requirement):
    """REQUIREMENT-ID: WMDEB-004

    `startpage` and `aimode` live in `<prefix>/lib/tetron-wm/` and run as
    `tetron-wm startpage|aimode`; nothing new on PATH. Without python3 the
    subcommand prints one line saying so.
    """


class DpkgAwareUpdater(Requirement):
    """REQUIREMENT-ID: WMDEB-005

    One install method per host. On a host where dpkg owns tetron-wm
    (`/var/lib/dpkg/info/tetron-wm.list`), nothing writes over the packaged
    binary: Settings > Updates installs the new release .deb with apt in a
    terminal window, and the dev channel refuses. The update check runs off
    the render loop.
    """


class LeanDeb(Requirement):
    """REQUIREMENT-ID: WMDEB-006

    `packaging/deb.toml`: the binary, the helpers in usr/lib/tetron-wm and
    the docs; Depends computed from the ELF; no Recommends (python3 and curl
    are Suggests); no maintainer scripts. `apt install` of the deb on a fresh
    Debian 13 adds exactly one package.
    """


class DesktopMetapackage(Requirement):
    """REQUIREMENT-ID: WMDEB-007

    `tetron-wm-desktop` (arch all) Depends `tetron-wm (>= version)` and
    Recommends the apps tetron-wm calls; services and kmscon are Suggests.
    """


class SuiteScriptOnDpkgHost(Requirement):
    """REQUIREMENT-ID: WMDEB-008

    The suite script installs the deb on a dpkg host, converts a raw install,
    downloads themes through the same directory listing, and ends with one
    tetron-wm on the host.
    """
