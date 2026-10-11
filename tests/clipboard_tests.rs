use std::time::{Duration, Instant};
use tetron_wm::config::Config;
use tetron_wm::session::{ClientMsg, SessionCore};

/// Pump the session until `done` is true or two seconds pass.
fn pump_until(core: &mut SessionCore, mut done: impl FnMut(&SessionCore) -> bool) -> bool {
    let t0 = Instant::now();
    while t0.elapsed() < Duration::from_secs(3) {
        core.pump_app_events();
        if done(core) {
            return true;
        }
        std::thread::sleep(Duration::from_millis(30));
    }
    false
}

fn launch(core: &mut SessionCore, script: &str) {
    core.apply(ClientMsg::Launch { name: "t".into(), command: "sh".into(), args: vec!["-c".into(), script.into()] });
}

#[test]
fn osc52_store_is_recorded_in_history() {
    let mut core = SessionCore::new(80, 24, Config::default());
    // "hello" in base64 is aGVsbG8=
    launch(&mut core, r"printf '\033]52;c;aGVsbG8=\033\\'; sleep 3");
    assert!(pump_until(&mut core, |c| c.clip_history().len() == 1), "store recorded");
    let e = core.clip_history().get(0).unwrap();
    assert_eq!(e.text, "hello");
    assert_eq!(e.source, tetron_wm::clipboard::ClipSource::App);
    core.shutdown();
}

#[test]
fn history_off_records_nothing() {
    let cfg = Config { clipboard_history: false, ..Config::default() };
    let mut core = SessionCore::new(80, 24, cfg);
    launch(&mut core, r"printf '\033]52;c;aGVsbG8=\033\\'; sleep 2");
    assert!(!pump_until(&mut core, |c| c.clip_history().len() == 1));
    core.shutdown();
}

#[test]
fn ignore_apps_skips_recording() {
    let cfg = Config { clipboard_ignore_apps: vec!["secret".into()], ..Config::default() };
    let mut core = SessionCore::new(80, 24, cfg);
    // The window title is "t"; the command line carries the marker.
    launch(&mut core, r"# secret-manager
printf '\033]52;c;aGVsbG8=\033\\'; sleep 2");
    assert!(!pump_until(&mut core, |c| c.clip_history().len() == 1));
    core.shutdown();
}

/// An app that asks for the clipboard with `ESC ] 52 ; c ; ? BEL` gets an OSC 52
/// reply carrying the current clipboard.
#[test]
fn osc52_read_is_answered_with_the_clipboard() {
    let out = std::env::temp_dir().join(format!("twm-osc52-{}", std::process::id()));
    let _ = std::fs::remove_file(&out);
    let mut core = SessionCore::new(80, 24, Config::default());
    // First window sets the clipboard, second one reads it (and is focused).
    launch(&mut core, r"printf '\033]52;c;aGVsbG8=\033\\'; sleep 4");
    assert!(pump_until(&mut core, |c| c.clip_history().len() == 1));
    let script = format!(
        r"stty raw -echo; printf '\033]52;c;?\007'; dd bs=1 count=16 2>/dev/null > {}; sleep 1",
        out.display()
    );
    launch(&mut core, &script);
    let got = pump_until(&mut core, |_| std::fs::metadata(&out).map(|m| m.len() >= 16).unwrap_or(false));
    assert!(got, "reply written to the app's stdin");
    assert_eq!(std::fs::read(&out).unwrap(), b"\x1b]52;c;aGVsbG8=\x07");
    let _ = std::fs::remove_file(&out);
    core.shutdown();
}

#[test]
fn osc52_read_never_is_not_answered() {
    let out = std::env::temp_dir().join(format!("twm-osc52n-{}", std::process::id()));
    let _ = std::fs::remove_file(&out);
    let cfg = Config { clipboard_osc52_read: "never".into(), ..Config::default() };
    let mut core = SessionCore::new(80, 24, cfg);
    let script = format!(
        r"stty raw -echo; printf '\033]52;c;?\007'; dd bs=1 count=1 2>/dev/null > {} & sleep 1.5; kill %1 2>/dev/null",
        out.display()
    );
    launch(&mut core, &script);
    let got = pump_until(&mut core, |_| std::fs::metadata(&out).map(|m| m.len() > 0).unwrap_or(false));
    assert!(!got, "no reply when reads are denied");
    let _ = std::fs::remove_file(&out);
    core.shutdown();
}

#[test]
fn persistence_round_trip_through_a_session() {
    let dir = std::env::temp_dir().join(format!("twm-clip-sess-{}", std::process::id()));
    let path = dir.join("h.jsonl");
    let mut core = SessionCore::new(80, 24, Config::default());
    core.enable_clipboard_persistence(path.clone());
    launch(&mut core, r"printf '\033]52;c;aGVsbG8=\033\\'; sleep 3");
    assert!(pump_until(&mut core, |c| c.clip_history().len() == 1));
    core.flush_clipboard();
    core.shutdown();
    let mut again = SessionCore::new(80, 24, Config::default());
    again.enable_clipboard_persistence(path.clone());
    assert_eq!(again.clip_history().get(0).map(|e| e.text.as_str()), Some("hello"));
    again.shutdown();
    let off = Config { clipboard_persist: false, ..Config::default() };
    let mut mem = SessionCore::new(80, 24, off);
    mem.enable_clipboard_persistence(path);
    assert!(mem.clip_history().is_empty(), "persist off: nothing loaded");
    mem.shutdown();
    let _ = std::fs::remove_dir_all(&dir);
}
