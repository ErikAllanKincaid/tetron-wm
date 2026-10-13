//! `HostReq::SetColors`: the running apphost re-applies the theme and terminal
//! color overrides, so hosted apps repaint without a restart. Lives in its own
//! integration-test binary because the theme is a process-wide global.

use std::time::Duration;
use tetron_wm::apphost::proto::{HostReq, PROTO_VERSION};
use tetron_wm::apphost::{AppHost, LocalAppHost, RemoteAppHost};
use tetron_wm::cell::Rgba;
use tetron_wm::theme::Theme;

#[test]
fn set_colors_request_round_trips_and_proto_is_v4() {
    let version = PROTO_VERSION;
    assert!(version >= 4);
    let req = HostReq::SetColors { theme: "nord".into(), terminal_bg: Some("#112233".into()), terminal_fg: None };
    let json = serde_json::to_string(&req).unwrap();
    match serde_json::from_str::<HostReq>(&json).unwrap() {
        HostReq::SetColors { theme, terminal_bg, terminal_fg } => {
            assert_eq!((theme.as_str(), terminal_bg.as_deref(), terminal_fg), ("nord", Some("#112233"), None));
        }
        other => panic!("unexpected {other:?}"),
    }
}

#[test]
fn local_host_ignores_set_colors() {
    // The in-process host shares the daemon's theme, so the default is a no-op.
    let mut host = LocalAppHost::new();
    host.set_colors("nord", Some("#112233"), None);
}

#[test]
fn running_app_repaints_with_new_default_colors() {
    let path = std::env::temp_dir().join(format!("tetron-colors-test-{}.sock", std::process::id()));
    let _ = std::fs::remove_file(&path);
    let listener = std::os::unix::net::UnixListener::bind(&path).unwrap();
    let server = std::thread::spawn(move || {
        let mut local = LocalAppHost::new();
        let mut shutdown = false;
        if let Some(Ok(stream)) = listener.incoming().next() {
            tetron_wm::apphost::server::server_serve_for_test(&mut local, stream, &mut shutdown);
        }
    });
    std::thread::sleep(Duration::from_millis(50));
    let mut remote = RemoteAppHost::connect(&path).unwrap();
    assert!(remote.proto_version() >= 4);
    let id = remote.spawn("cat", &[], None, 20, 5).expect("spawn");

    let bg_at = |remote: &RemoteAppHost| remote.snapshot(id).and_then(|g| g.get(10, 3).map(|c| c.bg));
    let wait_for = |remote: &RemoteAppHost, want: Rgba| {
        for _ in 0..150 {
            if bg_at(remote) == Some(want) {
                return true;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        false
    };

    // Override the terminal background.
    remote.set_colors("nord", Some("#112233"), Some("#aabbcc"));
    assert!(wait_for(&remote, Rgba { r: 0x11, g: 0x22, b: 0x33, a: 255 }), "terminal_bg applied live");

    // Clearing the override goes back to following the theme (and the theme itself switches live).
    remote.set_colors("pastoral", None, None);
    assert!(wait_for(&remote, Theme::pastoral().window_bg), "theme switch + cleared override applied live");

    remote.kill(id);
    remote.shutdown_apphost();
    let _ = std::fs::remove_file(&path);
    let _ = server.join();
}
