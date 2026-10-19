//! `tetron-wm --help` text. The config-key table is checked against the
//! `Config` struct by a test, so a new setting cannot ship undocumented.

/// (key, default, description) for every `config.toml` setting.
pub const CONFIG_KEYS: &[(&str, &str, &str)] = &[
    ("snapping_enabled", "false", "drag a window to a screen edge to snap it"),
    ("snap_threshold", "3", "cells from the edge that trigger snapping"),
    ("window_shadows", "false", "draw window drop shadows"),
    ("natural_scroll", "true", "invert wheel/touchpad scroll direction"),
    ("grid_rows", "2", "tiling grid rows (1-6)"),
    ("grid_cols", "2", "tiling grid columns (1-6)"),
    ("tile_gap", "0", "cells of gutter between tiled windows"),
    ("auto_tile", "false", "auto-arrange all windows into the grid"),
    ("launch_maximized", "false", "open new app windows maximized"),
    ("theme", "newmoon", "built-in preset or themes/<name>.toml stem"),
    ("theme_dir", "<config>/themes", "directory of *.toml theme files"),
    ("terminal_bg", "theme", "terminal default background (#rrggbb or name)"),
    ("terminal_fg", "theme", "terminal default foreground (#rrggbb or name)"),
    ("truecolor", "auto", "force 24-bit color on/off"),
    ("wallpaper", "wallpaper.jpg", "wallpaper image (needs chafa)"),
    ("wallpaper_enabled", "true", "master wallpaper toggle"),
    ("icon_style", "ascii", "file icon glyphs: ascii | nerd | emoji"),
    ("default_project_dir", "~", "root of the working-directory picker"),
    ("recent_dirs", "[]", "legacy: now in ~/.local/share/tetron-wm/state.toml"),
    ("show_hidden_dirs", "false", "show dot directories in the picker"),
    ("apps", "[]", "[[apps]] auto-started at launch (and docked)"),
    ("launcher", "[]", "[[launcher]] apps in the launcher (falls back to apps)"),
    ("default_apps", "built-in", "[default_apps] file role -> handler"),
    ("filemanager_view", "-", "default file manager view: icon | list"),
    ("desktop_enabled", "true", "show desktop icons"),
    ("desktop_scrim", "#000000c8", "icon label backdrop over wallpaper; none/off disables"),
    ("desktop_pins", "[]", "[[desktop_pins]] pinned desktop shortcuts"),
    ("desktop_positions", "{}", "legacy: now in ~/.local/share/tetron-wm/state.toml"),
    ("assistant_command", "opencode", "agent CLI for the assistant (opencode | hermes | any)"),
    ("assistant_args", "[]", "extra arguments for the assistant CLI"),
    ("assistant_mode", "panel", "assistant layout: panel | window"),
    ("update_branch", "main", "updater channel: main (release) | dev (source build)"),
    ("clipboard_history", "true", "record everything copied; browse from the menubar clipboard segment"),
    ("clipboard_persist", "true", "save the history to disk (~/.local/share/tetron-wm, mode 0600)"),
    ("clipboard_record_selection", "true", "record mouse selections, not just explicit copies"),
    ("clipboard_history_size", "100", "maximum unpinned history entries"),
    ("clipboard_item_max_kb", "256", "do not record entries larger than this"),
    ("clipboard_shims", "auto", "xclip/xsel/wl-copy/wl-paste shims for launched apps: auto (no display) | on | off"),
    ("clipboard_fake_display", "false", "dummy WAYLAND_DISPLAY (no display) so Neovim uses the clipboard shims"),
    ("clipboard_gui_bridge", "auto", "mirror the X11/Wayland clipboard into history: auto | on | off"),
    ("clipboard_osc52_read", "focused", "who may read the clipboard via OSC 52: focused | always | never"),
    ("clipboard_ignore_apps", "[]", "do not record while these apps are focused (e.g. password managers)"),
    ("clipboard_ttl_secs", "0", "expire unpinned entries after N seconds (0 = never)"),
    ("web_home", "https://lite.duckduckgo.com/lite/", "start page for w3m and other text browsers launched without a URL"),
    ("screenshot_dir", "", "where screenshots are saved (empty = ~/Pictures if present, else $HOME)"),
    ("leader", "ctrl+space", "leader chord, ctrl+<key> (needs reload)"),
    ("keys", "{}", "[keys] leader remaps: action = \"x\" or [\"x\", \"enter\"]"),
    ("tray_placement", "{}", "[tray_placement] segment -> bar | info | auto (defaults: cpu, mem, bluetooth info; rest auto)"),
    ("dock_badges", "built-in", "[dock_badges] app keyword -> badge color"),
    ("tray_tools", "built-in", "[tray_tools] cpu/mem/battery/bluetooth/network -> Open-row commands"),
];

/// (name, description) for every environment variable.
pub const ENV_VARS: &[(&str, &str)] = &[
    ("TETRON_WM_THEME", "theme override (wins over config theme)"),
    ("TETRON_WM_THEMES_DIR", "directory of theme files (wins over theme_dir)"),
    ("TETRON_WM_WALLPAPER", "wallpaper path (wins over config wallpaper)"),
    ("TETRON_WM_TRUECOLOR", "1/0 force truecolor on/off (wins over config truecolor)"),
    ("TETRON_WM_GRAPHICS", "1 force Kitty graphics on, 0 off"),
    ("TETRON_WM_GPM", "1 force the gpm mouse reader on, 0 off (Linux console)"),
    ("TETRON_WM_DEBUG", "set to print the exact system-switch script that will run"),
    ("TETRON_WM_SESSION", "session name (same as -s); set inside app windows so tetron-wm commands there reach their own session"),
    ("TETRON_WM_BIN_DIR", "install.sh/updater install dir (default /usr/local/bin)"),
    ("TETRON_WM_INSTALL_DEPS", "1 lets a piped install.sh install dependencies"),
    ("XDG_CONFIG_HOME", "config root (default ~/.config)"),
    ("XDG_DATA_HOME", "data root (default ~/.local/share)"),
    ("XDG_RUNTIME_DIR", "socket directory"),
    ("COLORTERM / TERM / TERM_PROGRAM", "terminal capability detection"),
    ("EDITOR / SHELL", "default editor / shell for launched apps"),
    ("PATH / HOME / SHELL / LANG", "captured into the apphost service by `service install`"),
];

/// The in-app shortcut section, generated from the overlay's own table.
fn shortcuts() -> String {
    let mut s = String::from("\nIN-APP SHORTCUTS (leader = Ctrl+Space, release, then a key):\n");
    for sec in crate::help::help_sections() {
        for (keys, desc) in sec.rows {
            s += &format!("  {keys:<8} {desc}\n");
        }
        for note in sec.notes {
            s += &format!("  {note}\n");
        }
    }
    s
}

/// The full `--help` text.
pub fn text() -> String {
    let mut s = format!(
        "tetron-wm {} - a window manager and desktop for the terminal\n\n\
USAGE:\n  tetron-wm [-s NAME] [COMMAND]\n\n\
COMMANDS:\n\
\x20 (none)                start the daemon if needed and attach\n\
\x20 attach [-d] [-s NAME] attach to a running daemon; -d evicts a client already\n\
\x20                       attached (otherwise attach is refused while one is)\n\
\x20 ls                    list sessions: attached / detached / recoverable / stale, windows\n\
\x20 -s, --session NAME    use the named session (own daemon, apps, sockets);\n\
\x20                       default: $TETRON_WM_SESSION, else \"default\"\n\
\x20 reload                restart the frontend only; apps keep running\n\
\x20 kill                  stop the daemon (closes all windows)\n\
\x20 kill-app <id|all>     close app <id> (see ps); all reaps only exited apps\n\
\x20 ps                    list running apps\n\
\x20 launch <cmd> [args]   open an app window in the running desktop\n\
\x20 tile                  tile all windows into the grid\n\
\x20 theme <name>          switch theme\n\
\x20 theme --list          list built-in and file themes\n\
\x20 theme --dump <name>   write a built-in theme to <config dir>/themes/<name>.toml\n\
\x20                       to edit (e.g. theme --dump nord, then set theme = \"nord\")\n\
\x20 msg '<json>'          send a raw ClientMsg\n\
\x20 clip copy|paste|list|clear|pin N|delete N   the clipboard history (see `clip` alone)\n\
\x20 service [install|uninstall|status]  per-user apphost service\n\
\x20 --daemon, --apphost   run the daemon / app host directly\n\
\x20 -V, --version         print version\n\
\x20 -h, --help            print this help (also works after any command)\n\n\
FILES:\n  config:  $XDG_CONFIG_HOME/tetron-wm/config.toml (default ~/.config/tetron-wm/)\n\
\x20 themes:  <config dir>/themes/*.toml\n  log:     ~/tetron-wm-debug.log\n\n\
CONFIG KEYS (config.toml):\n",
        env!("CARGO_PKG_VERSION")
    );
    let w = CONFIG_KEYS.iter().map(|k| k.0.len()).max().unwrap_or(0);
    let dw = CONFIG_KEYS.iter().map(|k| k.1.len()).max().unwrap_or(0);
    for (k, d, desc) in CONFIG_KEYS {
        s += &format!("  {k:<w$}  {d:<dw$}  {desc}\n");
    }
    s += &shortcuts();
    s += "\nKEYS ([keys] actions; defaults):\n";
    for (action, keys, desc) in crate::keymap::DEFAULTS {
        s += &format!("  {action:<10} {:<16} {desc}\n", keys.join(" "));
    }
    s += "\nENVIRONMENT:\n";
    let w = ENV_VARS.iter().map(|e| e.0.len()).max().unwrap_or(0);
    for (n, desc) in ENV_VARS {
        s += &format!("  {n:<w$}  {desc}\n");
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every `pub` field of `struct Config` must have a CONFIG_KEYS row, and vice versa.
    #[test]
    fn config_keys_match_config_struct() {
        let src = include_str!("config.rs");
        let body = src.split("pub struct Config {").nth(1).unwrap().split("\n}\n").next().unwrap();
        let mut fields: Vec<&str> = body
            .lines()
            .filter_map(|l| l.trim().strip_prefix("pub "))
            .filter_map(|l| l.split(':').next())
            .collect();
        let mut keys: Vec<&str> = CONFIG_KEYS.iter().map(|k| k.0).collect();
        fields.sort();
        keys.sort();
        assert_eq!(fields, keys);
    }

    #[test]
    fn text_lists_every_env_var() {
        let t = text();
        for (n, _) in ENV_VARS {
            assert!(t.contains(n));
        }
    }
}
