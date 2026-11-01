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


class PrefixedHelperNames(Requirement):
    """REQUIREMENT-ID: WMDEB-004

    The bundled helper scripts carry a tetron prefix so no generic name lands
    on PATH: `tetron-wm-startpage` (part of the desktop, its data lives in
    `~/.config/tetron-wm`) and `tetron-aimode` (an independent terminal app).
    They install on PATH like any app (`/usr/bin` from the deb, `~/.local/bin`
    from the suite installer) and the launcher finds them there. No
    compatibility for the old bare names `startpage` and `aimode`.
    """


class DpkgAwareUpdater(Requirement):
    """REQUIREMENT-ID: WMDEB-005

    One install method per host. On a host where dpkg owns tetron-wm
    (`/var/lib/dpkg/info/tetron-wm.list`), nothing writes over the packaged
    binary: Settings > Updates installs the new release .deb with apt in a
    terminal window, and the dev channel refuses. The update check runs off
    the render loop.

    install.sh on a dpkg host: package installed -> the release .deb upgrades
    it through apt, or, with no .deb in the release, nothing changes and it
    exits 3; a raw copy -> upgraded in place with a note; neither -> the
    .deb when the release has one, else the raw binary. The .deb is checked
    against its .sha256 first. Callers (Settings > Updates, Systems > Add
    Remote) never fall back to a source build after exit 3.
    """


class LeanDeb(Requirement):
    """REQUIREMENT-ID: WMDEB-006

    `packaging/deb.toml`: the binary, the helpers `tetron-wm-startpage` and
    `tetron-aimode` in usr/bin (WMDEB-004), and the docs in
    usr/share/doc/tetron-wm; Depends computed from the ELF only; no declared
    Depends or Recommends (python3, curl and the feature tools are
    Suggests, by their Debian names); no maintainer scripts. `apt install`
    of the deb on a fresh Debian 13 adds exactly one package. Release CI
    builds it from the released Linux binaries with the build-deb action
    pinned by commit SHA, tests the install in a clean container, and
    attaches the .deb and its .sha256 to the release, named
    `tetron-wm_<version>-<rev>_<arch>.deb` as install.sh expects (WMDEB-005).
    """


class DesktopMetapackage(Requirement):
    """REQUIREMENT-ID: WMDEB-007

    `tetron-wm-desktop` (`packaging/desktop.toml`, a metapackage: no
    binary, Architecture all) Depends `tetron-wm (>= {minor})`, so any
    later tetron-wm satisfies it and it never blocks an upgrade, and
    Recommends the apps tetron-wm calls, by names present in Debian 12 and
    13 and Ubuntu 22.04 and 24.04 (apt installs Recommends by default, so
    `apt install tetron-wm-desktop` brings the full set). Services
    (pipewire, wireplumber, network-manager, bluez), kmscon and fbi are
    Suggests, as are btop, age and openssh-server (USER review
    2026-10-31: an SSH server must never start just because the desktop
    set was installed). Tools Debian does not package (carbonyl, fresh,
    casttui, catt, uv, opencode, a symbols Nerd Font) stay with the
    installer scripts; catt and casttui join the suite script's desktop
    tier (WMDEB-008). The package lists come from USER's review list
    (DO-NOT-COMMIT/LIST_tetron-wm_deb_default_packages_review.md) until the
    app catalog generates them. Release CI builds it once, on the amd64 job, and tests it by
    installing it together with that job's tetron-wm .deb in a clean
    container, Recommends included.
    """


class ExtrasMetapackage(Requirement):
    """REQUIREMENT-ID: WMDEB-009

    `tetron-wm-extras` (`packaging/extras.toml`, a metapackage, Architecture
    all) Depends `tetron-wm-desktop (>= {minor})` and Recommends the
    nice-to-have apps tetron-wm does not need for its own features (USER
    review: fbi, git). Three sizes result: tetron-wm (nothing else),
    tetron-wm-desktop (every feature), tetron-wm-extras (every feature plus
    the extra apps). Release CI builds it once next to tetron-wm-desktop
    and checks its Recommends install together with the other two debs.
    """


class SuiteScriptOnDpkgHost(Requirement):
    """REQUIREMENT-ID: WMDEB-008

    The suite script installs the deb on a dpkg host, converts a raw install,
    downloads themes through the same directory listing, and ends with one
    tetron-wm on the host. Carried out by WMINST-002 (install matrix plan).
    """


class AppListSingleSource(Requirement):
    """REQUIREMENT-ID: WMINST-001

    The app sets are defined once (plan: PLAN_tetron-wm_install_methods_
    sizes_console_matrix.md). `packaging/desktop.toml` and
    `packaging/extras.toml` define the desktop and extras sizes by Debian
    name. `packaging/os-names.toml` gives, per Debian name, the check that
    tells it is present (a command, `font=NAME`, or `consolefonts`) and the
    Fedora, Arch and Homebrew names where they differ (an empty name: not
    on that OS), each verified in a container or against Homebrew; it also
    holds packages that are on some OSes but in no deb (uv, blueutil), and
    the services the suite script offers separately. `packaging/
    script-only.toml` lists apps no distro packages (catt, casttui, fresh,
    the Nerd Font, the helper scripts on a raw install) with their size.
    `scripts/gen_suite_lists.py` (stdlib) writes `assets/suite-apps.txt`
    from them, one `size:group:check:apt:dnf:pacman:brew` line per package
    and `size:script:name` per script-only app. Every Recommends of both
    tomls appears in it, and each entry of the suite script's former lists
    is either in it or in the generator's written drop list with a reason.
    """


class SuiteScriptSizes(Requirement):
    """REQUIREMENT-ID: WMINST-002

    Depends on WMINST-001 and WMDEB-005; carries out WMDEB-008 and the plan's
    WMINST-003 (other systems), since both replace the same hand-written
    lists. `install-tetron-wm-suite.sh`:

    - Asks every user the size: minimal (tetron-wm only), desktop (the
      default, preselected) or extras; `--size` answers it, `-y` takes
      desktop, `--no-extras` means minimal, `--extras` keeps the default.
    - Reads the apps from `assets/suite-apps.txt` (the checkout's copy when
      run from one, else fetched from the branch), never from lists of its
      own. A size installs its lines and the smaller sizes' lines; gui
      lines only with a desktop session; services lines only after their
      own question (default no).
    - On a Debian or Ubuntu host where tetron-wm is the apt package and the
      release has the size's metapackage, installs that metapackage from the
      release (checked against its .sha256) with apt, so apt tracks the set.
      Otherwise (tetron-wm not from the package, or no metapackage in the
      release) it installs the same apps one by one with the package
      manager. A metapackage is never installed next to a raw tetron-wm,
      since it would add a second copy.
    - On a dpkg host where tetron-wm is a raw copy and the release has the
      tetron-wm .deb, offers to convert it (default no; `--convert` answers
      yes): removes the raw copy, then installs the package.
    - Installs the script-only apps of the size (fresh, uv where not
      packaged, catt, casttui, the symbols Nerd Font, and the helper scripts
      when tetron-wm is not the deb), and calls install.sh with its own
      dependency step skipped.
    - Downloads the themes listed in `themes/` on the branch (GitHub
      contents API) into `~/.local/share/tetron-wm/themes`, falling back to
      the built-in list when the listing fails; never overwrites a file.
    - Records `size` and `method` (deb or raw) in `suite-state`.
    - `--dry-run` prints the size, method and every package it would
      install. Release URL and tag can be overridden (tests).
    """


class InstallShBinaryOnly(Requirement):
    """REQUIREMENT-ID: WMINST-004

    `install.sh` installs only tetron-wm: the deb or the raw binary per
    WMDEB-005. Its optional-dependency step is removed (the apps belong to
    the suite script's sizes); at the end it prints the suite script
    command for the apps, unless the suite script called it
    (TETRON_WM_SKIP_DEPS=1). TETRON_WM_INSTALL_DEPS is no longer read.
    When it installs or upgrades the tetron-wm deb and tetron-wm-desktop or
    tetron-wm-extras is installed, the release's matching metapackage debs
    are checked against their .sha256 and installed in the same apt
    command, so a release's new Recommends reach existing machines (USER
    2026-10-31). Systems > Add Remote no longer sets
    TETRON_WM_INSTALL_DEPS.
    """


class InstallMethodAndSize(Requirement):
    """REQUIREMENT-ID: WMINST-005

    tetron-wm knows how it was installed and which size is present, without
    a shell-out: the method is "apt package" when dpkg owns the running
    binary, "cargo" when it runs from a cargo bin directory, else "binary";
    the size is extras or desktop when dpkg has the tetron-wm-extras or
    tetron-wm-desktop package, else the `size` in the suite script's
    `~/.config/tetron-wm/suite-state`, else minimal. Settings > About shows
    both. `packaging/desktop.toml` suggests `kmscon (>= 9.1)`, since Debian
    12 ships kmscon 9.0.0 without truecolor and mouse passthrough.
    """
