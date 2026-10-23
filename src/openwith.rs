//! The Default Apps engine: classify a file into a `Role` and resolve the role to
//! an action (open with a builtin viewer, run a configured app, or show a menu).

use std::collections::BTreeMap;
use std::path::Path;

/// A coarse file category used to pick a default application.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Role {
    Image,
    Video,
    Audio,
    Text,
    Code,
    Archive,
    Pdf,
    Directory,
    Executable,
    Other,
}

impl Role {
    /// The config key for this role's handler.
    pub fn key(self) -> &'static str {
        match self {
            Role::Image => "image",
            Role::Video => "video",
            Role::Audio => "audio",
            Role::Text => "text",
            Role::Code => "code",
            Role::Archive => "archive",
            Role::Pdf => "pdf",
            Role::Directory => "directory",
            Role::Executable => "executable",
            Role::Other => "other",
        }
    }

    /// A human-readable label for this role (used in Get-Info and previews).
    pub fn label(self) -> &'static str {
        match self {
            Role::Image => "Image",
            Role::Video => "Video",
            Role::Audio => "Audio",
            Role::Text => "Text",
            Role::Code => "Code",
            Role::Archive => "Archive",
            Role::Pdf => "PDF",
            Role::Directory => "Folder",
            Role::Executable => "Executable",
            Role::Other => "Document",
        }
    }
}

/// Classify `path` into a `Role` from its extension only — **never reads file
/// contents**. This is called for every entry while listing a directory, so it
/// must do zero I/O: magic-byte sniffing (`infer`) opens files, which stalls for
/// seconds on large files and hangs indefinitely on iCloud-offloaded ("dataless")
/// files, freezing the single-threaded desktop loop. Extension-less / mislabeled
/// files simply classify as `Other`.
pub fn classify(path: &Path, is_dir: bool) -> Role {
    if is_dir {
        return Role::Directory;
    }
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .unwrap_or_default();
    role_for_ext(&ext).unwrap_or(Role::Other)
}

fn role_for_ext(ext: &str) -> Option<Role> {
    Some(match ext {
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "bmp" | "tiff" | "ico" => Role::Image,
        "mp4" | "mkv" | "mov" | "webm" | "avi" => Role::Video,
        "mp3" | "flac" | "wav" | "ogg" | "m4a" | "opus" => Role::Audio,
        "md" | "txt" | "log" | "json" | "toml" | "yaml" | "yml" | "csv" | "ini" | "conf" => Role::Text,
        "rs" | "py" | "js" | "ts" | "go" | "c" | "h" | "cpp" | "hpp" | "java" | "rb" | "sh" | "lua" | "html" | "css" => Role::Code,
        "zip" | "tar" | "gz" | "tgz" | "bz2" | "xz" | "7z" | "rar" | "zst" => Role::Archive,
        "pdf" => Role::Pdf,
        _ => return None,
    })
}

/// What to do when opening a path.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OpenAction {
    /// A directory the file manager should navigate into.
    Navigate,
    /// A built-in tetron-wm viewer, e.g. "@image".
    Builtin(&'static str),
    /// Launch a terminal app with the file path appended.
    RunApp { command: String, args: Vec<String> },
    /// No default — let the user pick.
    OpenWithMenu,
}

/// Resolve how to open `path`, given the configured `[default_apps]` map.
pub fn resolve(path: &Path, is_dir: bool, handlers: &BTreeMap<String, String>) -> OpenAction {
    let role = classify(path, is_dir);
    if role == Role::Directory {
        // Directories always navigate (the handler may still say "@navigate").
        return OpenAction::Navigate;
    }
    match handlers.get(role.key()).map(String::as_str) {
        Some(cmd) if !cmd.trim().is_empty() => open_with(cmd, path),
        // No handler chosen: video and audio play in the window with mpv
        // (any terminal) rather than asking every time.
        _ => match role {
            Role::Video if crate::catalog::is_installed("mpv") => open_with("mpv --vo=tct", path),
            Role::Audio if crate::catalog::is_installed("mpv") => open_with("mpv --no-video", path),
            _ => OpenAction::OpenWithMenu,
        },
    }
}

/// How handler `cmd` (a `[default_apps]` value or an Open-with choice: `@image`,
/// `@navigate`, or a command line) opens `path`.
pub fn open_with(cmd: &str, path: &Path) -> OpenAction {
    match cmd.trim() {
        "@image" => OpenAction::Builtin("@image"),
        "@navigate" => OpenAction::Navigate,
        "" => OpenAction::OpenWithMenu,
        cmd => {
            let mut parts = cmd.split_whitespace().map(String::from);
            let program = parts.next().unwrap_or_default();
            let mut args: Vec<String> = parts.collect();
            // fresh opened with a file skips its saved workspace and then saves
            // one holding only that file, losing the folder's tab list.
            // `--restore` keeps the tabs and adds the file.
            let is_fresh = Path::new(&program).file_name().is_some_and(|n| n == "fresh");
            if is_fresh && !args.iter().any(|a| a == "--restore" || a == "--no-restore") {
                args.insert(0, "--restore".into());
            }
            args.push(path.to_string_lossy().to_string());
            OpenAction::RunApp { command: program, args }
        }
    }
}

/// The default handler map for a fresh config (OS-aware where it matters).
pub fn default_handlers() -> BTreeMap<String, String> {
    let editor = std::env::var("EDITOR").ok().filter(|e| !e.is_empty()).unwrap_or_else(|| "vi".into());
    let mut m = BTreeMap::new();
    m.insert("image".into(), "@image".into());
    m.insert("directory".into(), "@navigate".into());
    m.insert("text".into(), editor.clone());
    m.insert("code".into(), editor.clone());
    // audio/video/archive/pdf/other left unset → Open-with menu until the user picks.
    // OS roles used by shortcuts:
    m.insert("editor".into(), editor);
    m.insert("terminal".into(), std::env::var("SHELL").unwrap_or_else(|_| "bash".into()));
    m
}

/// Candidate handlers for a role, offered in the Settings → Default Apps
/// chooser and the right-click "Open with…" menu: only programs that are
/// installed and suit the file type. Video and audio play inside the window
/// (`--vo=tct` draws video as truecolor characters on any terminal); a GUI
/// player window is offered too when a display exists.
pub fn candidates(role: Role) -> Vec<String> {
    let gui = std::env::var_os("DISPLAY").is_some() || std::env::var_os("WAYLAND_DISPLAY").is_some();
    let mut v = vec![String::new()]; // "" = Open-with menu / unset
    fn add(v: &mut Vec<String>, bin: &str, cmd: &str) {
        if crate::catalog::is_installed(bin) {
            v.push(cmd.to_string());
        }
    }
    match role {
        Role::Image => {
            v.push("@image".into());
            if gui {
                add(&mut v, "feh", "feh");
            }
        }
        Role::Directory => v.push("@navigate".into()),
        Role::Video => {
            add(&mut v, "mpv", "mpv --vo=tct");
            add(&mut v, "cvlc", "cvlc --play-and-exit --vout caca");
            if gui {
                add(&mut v, "mpv", "mpv");
                add(&mut v, "vlc", "vlc");
            }
        }
        Role::Audio => {
            add(&mut v, "mpv", "mpv --no-video");
            add(&mut v, "cvlc", "cvlc --play-and-exit");
        }
        Role::Pdf | Role::Archive => add(&mut v, "less", "less"),
        Role::Text | Role::Code | Role::Executable | Role::Other => {
            // emacs in the terminal (`-nw`), not its GUI window.
            for (bin, cmd) in [
                ("fresh", "fresh"),
                ("nvim", "nvim"),
                ("vim", "vim"),
                ("vi", "vi"),
                ("hx", "hx"),
                ("micro", "micro"),
                ("nano", "nano"),
                ("emacs", "emacs -nw"),
                ("less", "less"),
                ("bat", "bat"),
            ] {
                add(&mut v, bin, cmd);
            }
        }
    }
    v
}
