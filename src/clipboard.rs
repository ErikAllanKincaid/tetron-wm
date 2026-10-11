//! Clipboard history: a capped, de-duplicated ring of everything copied inside
//! tetron-wm (OSC 52 stores from apps, Ctrl+Shift+C, mouse selections, the
//! `clip` CLI, the GUI bridge). Pure state with load/save helpers; the session
//! decides when to record and when to write the file. See
//! `DO-NOT-COMMIT/IDEAS_tetron-wm_clipboard_manager.md`.

use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::{Path, PathBuf};

/// Where an entry came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ClipSource {
    /// An app's OSC 52 store.
    App,
    /// Ctrl+Shift+C.
    Copy,
    /// A mouse text selection.
    Selection,
    /// `tetron-wm clip copy` or an xclip/xsel/wl-copy shim.
    Cli,
    /// The real GUI clipboard (X11/Wayland bridge).
    Gui,
}

impl ClipSource {
    /// Short tag shown in the list.
    pub fn tag(self) -> &'static str {
        match self {
            ClipSource::App => "app",
            ClipSource::Copy => "copy",
            ClipSource::Selection => "sel",
            ClipSource::Cli => "cli",
            ClipSource::Gui => "gui",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClipEntry {
    pub text: String,
    /// Unix time in milliseconds when it was last copied.
    #[serde(default)]
    pub time_ms: u64,
    pub source: ClipSource,
    #[serde(default)]
    pub pinned: bool,
}

/// Size limits, from the `clipboard_*` config keys.
#[derive(Clone, Copy, Debug)]
pub struct ClipLimits {
    /// Maximum number of entries (pinned entries do not count against it).
    pub max_entries: usize,
    /// Entries larger than this many bytes are not recorded.
    pub max_item_bytes: usize,
    /// Total size cap; the oldest unpinned entries are dropped past it.
    pub max_total_bytes: usize,
}

impl Default for ClipLimits {
    fn default() -> Self {
        ClipLimits { max_entries: 100, max_item_bytes: 256 * 1024, max_total_bytes: 4 * 1024 * 1024 }
    }
}

/// Newest first; pinned entries are kept (and listed) ahead of the rest.
#[derive(Clone, Debug, Default)]
pub struct ClipHistory {
    entries: Vec<ClipEntry>,
    limits: ClipLimits,
    paused: bool,
    /// Bumped on every change so the session can debounce disk writes.
    revision: u64,
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

impl ClipHistory {
    pub fn new(limits: ClipLimits) -> Self {
        ClipHistory { entries: Vec::new(), limits, paused: false, revision: 0 }
    }

    pub fn set_limits(&mut self, limits: ClipLimits) {
        self.limits = limits;
        self.enforce();
    }

    pub fn paused(&self) -> bool { self.paused }
    pub fn set_paused(&mut self, p: bool) { self.paused = p; }
    pub fn revision(&self) -> u64 { self.revision }
    pub fn len(&self) -> usize { self.entries.len() }
    pub fn is_empty(&self) -> bool { self.entries.is_empty() }
    pub fn entries(&self) -> &[ClipEntry] { &self.entries }

    /// Record a copy. Returns whether it was recorded (not paused, not empty or
    /// whitespace-only, not over the item size cap). A duplicate moves to the
    /// front and keeps its pinned flag.
    pub fn record(&mut self, text: &str, source: ClipSource) -> bool {
        if self.paused || text.trim().is_empty() || text.len() > self.limits.max_item_bytes {
            return false;
        }
        let mut pinned = false;
        if let Some(i) = self.entries.iter().position(|e| e.text == text) {
            pinned = self.entries.remove(i).pinned;
        }
        self.entries.insert(0, ClipEntry { text: text.to_string(), time_ms: now_ms(), source, pinned });
        self.sort_pinned_first();
        self.enforce();
        self.revision += 1;
        true
    }

    /// Stable sort keeping newest-first order inside each group.
    fn sort_pinned_first(&mut self) {
        self.entries.sort_by_key(|e| !e.pinned);
    }

    /// Drop the oldest unpinned entries past the count and byte caps.
    fn enforce(&mut self) {
        let unpinned = |v: &[ClipEntry]| v.iter().filter(|e| !e.pinned).count();
        while unpinned(&self.entries) > self.limits.max_entries {
            self.drop_oldest_unpinned();
        }
        let total = |v: &[ClipEntry]| v.iter().map(|e| e.text.len()).sum::<usize>();
        while total(&self.entries) > self.limits.max_total_bytes && unpinned(&self.entries) > 0 {
            self.drop_oldest_unpinned();
        }
    }

    fn drop_oldest_unpinned(&mut self) {
        if let Some(i) = self.entries.iter().rposition(|e| !e.pinned) {
            self.entries.remove(i);
        }
    }

    pub fn get(&self, i: usize) -> Option<&ClipEntry> { self.entries.get(i) }

    /// Newest entry regardless of pinning (what a plain paste would give).
    pub fn latest(&self) -> Option<&ClipEntry> {
        // First strict maximum: on equal timestamps the earlier (newer) index wins.
        self.entries.iter().fold(None, |best: Option<&ClipEntry>, e| match best {
            Some(b) if b.time_ms >= e.time_ms => Some(b),
            _ => Some(e),
        })
    }

    /// Indices matching a case-insensitive substring filter (all when empty).
    pub fn matching(&self, filter: &str) -> Vec<usize> {
        let f = filter.to_lowercase();
        self.entries
            .iter()
            .enumerate()
            .filter(|(_, e)| f.is_empty() || e.text.to_lowercase().contains(&f))
            .map(|(i, _)| i)
            .collect()
    }

    pub fn toggle_pin(&mut self, i: usize) {
        if let Some(e) = self.entries.get_mut(i) {
            e.pinned = !e.pinned;
            self.sort_pinned_first();
            self.enforce();
            self.revision += 1;
        }
    }

    pub fn delete(&mut self, i: usize) {
        if i < self.entries.len() {
            self.entries.remove(i);
            self.revision += 1;
        }
    }

    /// Remove everything, pinned included.
    pub fn clear(&mut self) {
        self.entries.clear();
        self.revision += 1;
    }

    // ── Disk ────────────────────────────────────────────────────────────────

    /// Load a jsonl file; unreadable or malformed lines are skipped.
    pub fn load(&mut self, path: &Path) {
        let Ok(text) = std::fs::read_to_string(path) else { return };
        let mut v: Vec<ClipEntry> = text.lines().filter_map(|l| serde_json::from_str(l).ok()).collect();
        v.retain(|e| !e.text.trim().is_empty());
        self.entries = v;
        self.sort_pinned_first();
        self.enforce();
    }

    /// Write the history as jsonl, atomically, readable only by the owner.
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700));
            }
        }
        let tmp = path.with_extension("jsonl.tmp");
        {
            let mut opts = std::fs::OpenOptions::new();
            opts.write(true).create(true).truncate(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                opts.mode(0o600);
            }
            let mut f = opts.open(&tmp)?;
            for e in &self.entries {
                writeln!(f, "{}", serde_json::to_string(e).map_err(std::io::Error::other)?)?;
            }
            f.flush()?;
        }
        std::fs::rename(&tmp, path)
    }
}

/// `~/.local/share/tetron-wm/clipboard-history.jsonl` (honours `XDG_DATA_HOME`).
pub fn history_path() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_DATA_HOME")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .or_else(|| dirs::home_dir().map(|h| h.join(".local/share")))?;
    Some(base.join("tetron-wm").join("clipboard-history.jsonl"))
}

/// The OSC 52 reply to a clipboard query: `ESC ] 52 ; <kind> ; <base64> <term>`.
/// `kind` is `c` (clipboard) or `p` (primary); `bel` picks the terminator the
/// app used for its query (BEL, else ST).
pub fn osc52_reply(kind: char, text: &str, bel: bool) -> String {
    use base64::Engine;
    let b64 = base64::engine::general_purpose::STANDARD.encode(text.as_bytes());
    format!("\x1b]52;{kind};{b64}{}", if bel { "\x07" } else { "\x1b\\" })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn h() -> ClipHistory { ClipHistory::new(ClipLimits::default()) }

    #[test]
    fn records_newest_first_and_dedupes() {
        let mut c = h();
        assert!(c.record("one", ClipSource::Copy));
        assert!(c.record("two", ClipSource::App));
        assert!(c.record("one", ClipSource::Selection));
        let t: Vec<&str> = c.entries().iter().map(|e| e.text.as_str()).collect();
        assert_eq!(t, ["one", "two"]);
        assert_eq!(c.get(0).unwrap().source, ClipSource::Selection);
    }

    #[test]
    fn ignores_blank_oversize_and_paused() {
        let mut c = ClipHistory::new(ClipLimits { max_item_bytes: 4, ..ClipLimits::default() });
        assert!(!c.record("", ClipSource::Copy));
        assert!(!c.record("  \n\t", ClipSource::Copy));
        assert!(!c.record("12345", ClipSource::Copy));
        assert!(c.record("1234", ClipSource::Copy));
        c.set_paused(true);
        assert!(!c.record("abc", ClipSource::Copy));
        assert_eq!(c.len(), 1);
    }

    #[test]
    fn count_cap_drops_oldest_but_keeps_pinned() {
        let mut c = ClipHistory::new(ClipLimits { max_entries: 2, ..ClipLimits::default() });
        c.record("keep", ClipSource::Copy);
        c.toggle_pin(0);
        for t in ["a", "b", "c"] {
            c.record(t, ClipSource::Copy);
        }
        let t: Vec<&str> = c.entries().iter().map(|e| e.text.as_str()).collect();
        assert_eq!(t, ["keep", "c", "b"], "pinned first, two newest unpinned");
    }

    #[test]
    fn byte_cap_drops_oldest_unpinned() {
        let mut c = ClipHistory::new(ClipLimits { max_total_bytes: 6, ..ClipLimits::default() });
        c.record("aaaa", ClipSource::Copy);
        c.record("bbbb", ClipSource::Copy);
        assert_eq!(c.len(), 1);
        assert_eq!(c.get(0).unwrap().text, "bbbb");
    }

    #[test]
    fn pin_survives_rerecord_and_sorts_first() {
        let mut c = h();
        c.record("x", ClipSource::Copy);
        c.record("y", ClipSource::Copy);
        c.toggle_pin(1); // x
        assert_eq!(c.get(0).unwrap().text, "x");
        c.record("x", ClipSource::App);
        assert!(c.get(0).unwrap().pinned);
        assert_eq!(c.latest().unwrap().text, "x");
    }

    #[test]
    fn matching_filters_case_insensitively() {
        let mut c = h();
        c.record("Hello World", ClipSource::Copy);
        c.record("other", ClipSource::Copy);
        assert_eq!(c.matching("WORLD").len(), 1);
        assert_eq!(c.matching("").len(), 2);
        assert!(c.matching("zzz").is_empty());
    }

    #[test]
    fn jsonl_round_trip_with_private_mode() {
        let dir = std::env::temp_dir().join(format!("twm-clip-{}", std::process::id()));
        let path = dir.join("sub/clipboard-history.jsonl");
        let mut c = h();
        c.record("line1\nline2", ClipSource::Selection);
        c.record("p", ClipSource::Cli);
        c.toggle_pin(1);
        c.save(&path).unwrap();
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(std::fs::metadata(&path).unwrap().permissions().mode() & 0o777, 0o600);
        let mut d = h();
        d.load(&path);
        assert_eq!(d.entries(), c.entries());
        std::fs::write(&path, "garbage\n{\"text\":\"ok\",\"source\":\"copy\"}\n").unwrap();
        d.load(&path);
        assert_eq!(d.len(), 1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn osc52_reply_format() {
        assert_eq!(osc52_reply('c', "hi", true), "\x1b]52;c;aGk=\x07");
        assert_eq!(osc52_reply('p', "hi", false), "\x1b]52;p;aGk=\x1b\\");
    }
}
