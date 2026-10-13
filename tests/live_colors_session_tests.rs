//! The session pushes terminal colors to the apphost: on start-up, on a theme
//! change, on Settings Preview / Revert, and when Settings closes mid-preview.
//! Own integration-test binary: it points `XDG_CONFIG_HOME` at a temp dir so the
//! Settings panel's config saves never touch the real config.

use std::path::Path;
use std::sync::{Arc, Mutex};
use tetron_wm::apphost::{AppHost, AppId};
use tetron_wm::buffer::CellBuffer;
use tetron_wm::config::Config;
use tetron_wm::kittygfx::Placement;
use tetron_wm::session::{ClientMsg, SessionCore};

type Calls = Arc<Mutex<Vec<(String, Option<String>, Option<String>)>>>;

/// An apphost with no apps that records every `set_colors` call.
struct Recorder {
    calls: Calls,
    proto: u32,
}

impl AppHost for Recorder {
    fn spawn(&mut self, _: &str, _: &[String], _: Option<&Path>, _: i32, _: i32) -> std::io::Result<AppId> {
        Err(std::io::Error::other("recorder hosts no apps"))
    }
    fn input(&mut self, _: AppId, _: &[u8]) {}
    fn resize(&mut self, _: AppId, _: i32, _: i32) {}
    fn kill(&mut self, _: AppId) {}
    fn is_alive(&mut self, _: AppId) -> bool { false }
    fn snapshot(&self, _: AppId) -> Option<CellBuffer> { None }
    fn placements(&self, _: AppId) -> Vec<Placement> { Vec::new() }
    fn image_png(&self, _: AppId, _: u32) -> Option<Vec<u8>> { None }
    fn list(&self) -> Vec<AppId> { Vec::new() }
    fn set_meta(&mut self, _: AppId, _: Vec<u8>) {}
    fn meta(&self, _: AppId) -> Option<Vec<u8>> { None }
    fn remove(&mut self, _: AppId) {}
    fn proto_version(&self) -> u32 { self.proto }
    fn set_colors(&mut self, theme: &str, bg: Option<&str>, fg: Option<&str>) {
        self.calls.lock().unwrap().push((theme.into(), bg.map(String::from), fg.map(String::from)));
    }
}

fn session(proto: u32) -> (SessionCore, Calls) {
    let calls: Calls = Arc::default();
    let core = SessionCore::with_apphost(100, 30, Config::default(), Box::new(Recorder { calls: calls.clone(), proto }));
    (core, calls)
}

fn open_terminal_section(core: &mut SessionCore) {
    core.apply(ClientMsg::OpenSettings);
    for _ in 0..7 {
        core.apply(ClientMsg::SettingsNextSection);
    }
}

#[test]
fn colors_follow_start_theme_preview_revert_and_close() {
    let dir = std::env::temp_dir().join(format!("tetron-colors-session-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::env::set_var("XDG_CONFIG_HOME", &dir);

    let (mut core, calls) = session(4);
    let n = |c: &Calls| c.lock().unwrap().len();
    let last = |c: &Calls| c.lock().unwrap().last().cloned().unwrap();

    // Start-up: a long-lived apphost may predate a config edit, so push once.
    assert_eq!(n(&calls), 1);
    assert_eq!(last(&calls), ("newmoon".to_string(), None, None));

    // A theme change reaches the apphost.
    core.apply(ClientMsg::SetTheme("nord".into()));
    assert_eq!(last(&calls), ("nord".to_string(), None, None));
    let base = n(&calls);

    // Staging a preset in Settings pushes nothing.
    open_terminal_section(&mut core);
    core.apply(ClientMsg::SettingsRight);
    assert_eq!(n(&calls), base, "staged colors stay in the panel");

    // Preview (row 3) pushes the staged colors, unsaved.
    for _ in 0..3 {
        core.apply(ClientMsg::SettingsDown);
    }
    core.apply(ClientMsg::SettingsToggle);
    assert_eq!(last(&calls), ("nord".to_string(), Some("#0d1117".into()), Some("#c9d1d9".into())));
    assert_eq!(core.config_for_test().terminal_bg, None, "preview is not saved");

    // Closing Settings mid-preview puts the saved colors back.
    core.apply(ClientMsg::SettingsClose);
    assert_eq!(last(&calls), ("nord".to_string(), None, None));

    // Preview again, then Revert (row 5) restores them too.
    open_terminal_section(&mut core);
    core.apply(ClientMsg::SettingsRight);
    for _ in 0..3 {
        core.apply(ClientMsg::SettingsDown);
    }
    core.apply(ClientMsg::SettingsToggle); // Preview
    let after_preview = n(&calls);
    core.apply(ClientMsg::SettingsDown);
    core.apply(ClientMsg::SettingsDown);
    core.apply(ClientMsg::SettingsToggle); // Revert
    assert_eq!(n(&calls), after_preview + 1);
    assert_eq!(last(&calls), ("nord".to_string(), None, None));

    // Save (row 4) persists the staged colors; the session pushes them.
    core.apply(ClientMsg::SettingsUp);
    core.apply(ClientMsg::SettingsUp);
    core.apply(ClientMsg::SettingsUp);
    core.apply(ClientMsg::SettingsUp);
    core.apply(ClientMsg::SettingsUp);
    core.apply(ClientMsg::SettingsRight); // stage dark again (preset row 0)
    for _ in 0..4 {
        core.apply(ClientMsg::SettingsDown);
    }
    core.apply(ClientMsg::SettingsToggle); // Save
    assert_eq!(core.config_for_test().terminal_bg.as_deref(), Some("#0d1117"));
    assert_eq!(last(&calls), ("nord".to_string(), Some("#0d1117".into()), Some("#c9d1d9".into())));

    let _ = std::fs::remove_dir_all(&dir);
}
