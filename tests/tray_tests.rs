use tetron_wm::system::{ClockInfo, MemInfo, SystemState, VolumeInfo, WifiInfo};
use tetron_wm::tray::{tray_segments, SegmentKind};

fn sample() -> SystemState {
    SystemState {
        clock: ClockInfo {
            time: "09:41".into(),
            date: "Wed 04 Jun".into(),
            uptime_secs: 0,
            year: 2026,
            month: 6,
            day: 4,
        },
        cpu_pct: 32.0,
        mem: MemInfo { used: 6, total: 10 },
        wifi: Some(WifiInfo { ssid: "wlan".into(), signal: 3, enabled: true }),
        volume: VolumeInfo { level: 60, muted: false },
        ..Default::default()
    }
}

#[test]
fn segments_are_right_aligned_and_on_row_zero() {
    let segs = tray_segments(&sample(), 100, 9, 0);
    // Volume is the farthest-right segment (a fixed eyes-free corner knob).
    let vol = segs.iter().find(|s| s.kind == SegmentKind::Volume).unwrap();
    let max_x = segs.iter().map(|s| s.rect.x + s.rect.w).max().unwrap();
    assert_eq!(vol.rect.x + vol.rect.w, max_x);
    assert!(segs.iter().all(|s| s.rect.y == 0));
}

#[test]
fn narrow_width_drops_cpu_but_keeps_clock() {
    let wide = tray_segments(&sample(), 100, 9, 0);
    let narrow = tray_segments(&sample(), 24, 9, 0);
    assert!(narrow.iter().any(|s| s.kind == SegmentKind::Clock));
    assert!(narrow.iter().any(|s| s.kind == SegmentKind::Volume), "volume is always kept");
    assert!(!narrow.iter().any(|s| s.kind == SegmentKind::Cpu));
    assert!(wide.iter().any(|s| s.kind == SegmentKind::Cpu));
}

#[test]
fn segments_do_not_overlap() {
    let segs = tray_segments(&sample(), 100, 9, 0);
    for pair in segs.windows(2) {
        assert!(pair[0].rect.x + pair[0].rect.w <= pair[1].rect.x);
    }
}

use tetron_wm::geometry::Point;
use tetron_wm::system::ControlIntent;
use tetron_wm::tray::Tray;

#[test]
fn clicking_a_segment_opens_then_closes_its_popover() {
    let mut tray = Tray::new();
    let segs = tray_segments(&sample(), 100, 9, 0);
    let vol = segs.iter().find(|s| s.kind == SegmentKind::Volume).unwrap();
    assert!(tray.on_menubar_click(Point::new(vol.rect.x, 0), &segs));
    assert_eq!(tray.open(), Some(SegmentKind::Volume));
    // Clicking the same segment again closes it.
    assert!(tray.on_menubar_click(Point::new(vol.rect.x, 0), &segs));
    assert_eq!(tray.open(), None);
}

#[test]
fn volume_popover_arrows_yield_intents() {
    let mut tray = Tray::new();
    tray.force_open(SegmentKind::Volume);
    let r = tray.render(100, 30, &sample());
    let up = r.hits.iter().find(|h| h.intent == ControlIntent::VolumeUp).unwrap();
    assert_eq!(tray.on_popover_click(up.rect.center(), &r), Some(ControlIntent::VolumeUp));
    let down = r.hits.iter().find(|h| h.intent == ControlIntent::VolumeDown).unwrap();
    assert_eq!(tray.on_popover_click(down.rect.center(), &r), Some(ControlIntent::VolumeDown));
}

#[test]
fn closed_tray_renders_nothing() {
    let tray = Tray::new();
    let r = tray.render(100, 30, &sample());
    assert!(r.layers.is_empty());
    assert!(r.bounds.is_none());
}

#[test]
fn clock_segment_is_time_only() {
    // The clock shows only the time now; the date/day live in the calendar popover.
    let segs = tray_segments(&sample(), 100, 9, 0);
    let clock = segs.iter().find(|s| s.kind == SegmentKind::Clock).unwrap();
    assert_eq!(clock.text, "09:41");
}

#[test]
fn clock_popover_is_a_calendar_with_month_nav() {
    let mut tray = Tray::new();
    tray.force_open(SegmentKind::Clock);
    let r = tray.render(100, 30, &sample());
    assert!(!r.layers.is_empty());
    let prev = r.hits.iter().find(|h| h.intent == ControlIntent::CalendarPrev);
    let next = r.hits.iter().find(|h| h.intent == ControlIntent::CalendarNext);
    assert!(prev.is_some() && next.is_some(), "calendar has ◂ ▸ month navigation");
    // Stepping a month changes the rendered grid (bounds stay a calendar box).
    tray.calendar_step(1);
    let r2 = tray.render(100, 30, &sample());
    assert!(r2.bounds.is_some());
}

#[test]
fn calendar_grid_marks_today() {
    // June 2026: the 4th is a Thursday — the calendar must place it in column 3.
    let weeks = tetron_wm::calendar::month_grid(2026, 6);
    assert_eq!(weeks[0][3], Some(4));
}

#[test]
fn bell_segment_appears_with_notifications() {
    let none = tray_segments(&sample(), 100, 9, 0);
    assert!(!none.iter().any(|s| s.kind == SegmentKind::Bell));
    let some = tray_segments(&sample(), 100, 9, 3);
    let bell = some.iter().find(|s| s.kind == SegmentKind::Bell).unwrap();
    assert!(bell.text.contains('3'));
}

#[test]
fn bell_popover_rows_focus_and_clear() {
    let mut tray = Tray::new();
    tray.notify(7, "claude".into(), "09:41".into());
    tray.notify(9, "btop".into(), "09:42".into());
    assert_eq!(tray.notif_count(), 2);
    tray.force_open(SegmentKind::Bell);
    let r = tray.render(100, 30, &sample());
    let focus = r.hits.iter().find(|h| h.intent == ControlIntent::NotifFocus(9));
    assert!(focus.is_some(), "newest notification row focuses its window");
    let clear = r.hits.iter().find(|h| h.intent == ControlIntent::NotifClear);
    assert!(clear.is_some(), "clear-all row present");
    tray.clear_notifs_for(9);
    assert!(!tray.has_notif_for(9));
    assert!(tray.has_notif_for(7));
    tray.clear_notifs();
    assert_eq!(tray.notif_count(), 0);
}

#[test]
fn calendar_lists_upcoming_events() {
    let mut st = sample();
    st.events = vec![
        tetron_wm::system::CalEvent { year: 2026, month: 6, day: 10, text: "09:30 Standup".into() },
    ];
    let mut tray = Tray::new();
    tray.force_open(SegmentKind::Clock);
    let r = tray.render(100, 30, &st);
    let buf = &r.layers[0].buf;
    let mut found = false;
    for y in 0..buf.height() {
        let row: String = (0..buf.width()).filter_map(|x| buf.get(x, y).map(|c| c.ch)).collect();
        if row.contains("Standup") {
            found = true;
        }
    }
    assert!(found, "event line rendered in the calendar popover");
}

#[test]
fn network_label_is_wifi_or_net_without_the_ssid() {
    use tetron_wm::system::NetInfo;
    let mut st = sample();
    st.wifi = Some(WifiInfo { ssid: "HomeNetwork5G".into(), signal: 3, enabled: true });
    let label = |st: &SystemState| tray_segments(st, 100, 9, 0).into_iter().find(|s| s.kind == SegmentKind::Wifi).map(|s| s.text);
    let t = label(&st).unwrap();
    assert!(t.ends_with(" wifi") && !t.contains("Home"), "label: {t:?}");
    // Wired default route wins over an idle WiFi radio.
    st.net = Some(NetInfo { iface: "eth0".into(), ip: "10.0.0.5".into(), wifi: false });
    assert!(label(&st).unwrap().ends_with(" net"));
    // Wired only, no WiFi hardware.
    st.wifi = None;
    assert!(label(&st).unwrap().ends_with(" net"));
    // No link and no WiFi: segment hidden.
    st.net = None;
    assert!(label(&st).is_none());
}

#[test]
fn network_popover_shows_ip_and_copies_it() {
    use tetron_wm::system::{ControlIntent, NetInfo};
    use tetron_wm::tray::Tray;
    let mut st = sample();
    st.net = Some(NetInfo { iface: "wlan0".into(), ip: "192.168.1.42".into(), wifi: true });
    let mut tray = Tray::new();
    tray.force_open(SegmentKind::Wifi);
    let r = tray.render(100, 30, &st);
    let text: String = r.layers.iter().flat_map(|l| {
        let b = &l.buf;
        (0..b.height()).flat_map(move |y| (0..b.width()).filter_map(move |x| b.get(x, y).map(|c| c.ch)).chain(std::iter::once('\n')))
    }).collect();
    assert!(text.contains("192.168.1.42 (wlan0)"), "{text}");
    assert!(text.contains("SSID  wlan"), "{text}");
    assert!(r.hits.iter().any(|h| h.intent == ControlIntent::CopyText("192.168.1.42".into())));
    // No link: Offline, no copy row.
    st.net = None;
    st.wifi = None;
    let r = tray.render(100, 30, &st);
    assert!(r.hits.iter().all(|h| !matches!(h.intent, ControlIntent::CopyText(_))));
}

#[test]
fn popover_offers_the_right_tool() {
    use tetron_wm::traytools::TrayTools;
    use tetron_wm::tray::segment_app;
    let cfg = TrayTools::new();
    let none = |_: &str| false;
    let all = |_: &str| true;
    // CPU: htop when present, else top.
    let cpu = segment_app(SegmentKind::Cpu, &cfg, &all).unwrap();
    assert_eq!((cpu.command.as_str(), cpu.label.as_str()), ("htop", "Open htop"));
    assert_eq!(segment_app(SegmentKind::Cpu, &cfg, &none).unwrap().command, "top");
    // Memory: free -h (held open), vm_stat without free, no action with neither.
    let mem = segment_app(SegmentKind::Mem, &cfg, &all).unwrap();
    assert_eq!((mem.command.as_str(), mem.label.as_str()), ("sh", "Open free -h"));
    assert!(mem.args[1].starts_with("free -h;"), "{:?}", mem.args);
    let only_vm = |b: &str| b == "vm_stat";
    assert!(segment_app(SegmentKind::Mem, &cfg, &only_vm).unwrap().args[1].starts_with("vm_stat;"));
    assert!(segment_app(SegmentKind::Mem, &cfg, &none).is_none());
    // Battery: jolt, else upower, else pmset, else no row.
    assert_eq!(segment_app(SegmentKind::Battery, &cfg, &all).unwrap().command, "jolt");
    let only_up = |b: &str| b == "upower";
    assert!(segment_app(SegmentKind::Battery, &cfg, &only_up).unwrap().args[1].starts_with("upower -i"));
    assert!(segment_app(SegmentKind::Battery, &cfg, &none).is_none());
    // Network: wlctl first, ip as the held fallback.
    assert_eq!(segment_app(SegmentKind::Wifi, &cfg, &all).unwrap().command, "wlctl");
    let only_ip = |b: &str| b == "ip";
    assert!(segment_app(SegmentKind::Wifi, &cfg, &only_ip).unwrap().args[1].starts_with("ip -br addr"));
    // Bluetooth: bluetuith first, bluetoothctl next.
    assert_eq!(segment_app(SegmentKind::Bluetooth, &cfg, &all).unwrap().command, "bluetuith");
    let only_ctl = |b: &str| b == "bluetoothctl";
    assert_eq!(segment_app(SegmentKind::Bluetooth, &cfg, &only_ctl).unwrap().command, "bluetoothctl");
    // Other segments have no action row.
    assert!(segment_app(SegmentKind::Clock, &cfg, &all).is_none());
}

#[test]
fn cpu_popover_has_a_clickable_open_row() {
    use tetron_wm::system::ControlIntent;
    use tetron_wm::tray::Tray;
    let mut tray = Tray::new();
    tray.force_open(SegmentKind::Cpu);
    let r = tray.render(100, 30, &sample());
    // top is always available, htop when installed: either way one OpenTool row.
    let hit = r.hits.iter().find(|h| matches!(h.intent, ControlIntent::OpenTool { .. }));
    let hit = hit.expect("CPU popover has an Open row");
    match &hit.intent {
        ControlIntent::OpenTool { title, command, .. } => {
            assert_eq!(title, "CPU");
            assert!(command == "htop" || command == "top", "{command}");
        }
        _ => unreachable!(),
    }
    // Clicking that row maps back to the same intent.
    let p = tetron_wm::geometry::Point::new(hit.rect.x + 1, hit.rect.y);
    assert!(matches!(tray.on_popover_click(p, &r), Some(ControlIntent::OpenTool { .. })));
}

#[test]
fn tray_never_covers_the_left_chrome_while_items_can_be_shed() {
    use tetron_wm::chrome::TASKBAR_X;
    // Every width from roomy down to just past what the always-kept segments
    // (volume + clock) need: no segment may start left of the taskbar.
    for width in 45..=120 {
        let segs = tray_segments(&sample(), width, 0, 0);
        let left = segs.iter().map(|s| s.rect.x).min().unwrap();
        assert!(left >= TASKBAR_X, "width {width}: tray starts at {left}, covers the launcher");
    }
}

#[test]
fn shedding_goes_in_order_cpu_first() {
    // Just too narrow for cpu: it goes before anything else the sample has.
    let full = tray_segments(&sample(), 120, 0, 0);
    let w = full.iter().map(|s| s.rect.w).sum::<i32>() + 2 * (full.len() as i32 - 1);
    let tight = tray_segments(&sample(), tetron_wm::chrome::TASKBAR_X + w - 1, 0, 0);
    assert!(!tight.iter().any(|s| s.kind == SegmentKind::Cpu));
    assert!(tight.iter().any(|s| s.kind == SegmentKind::Mem));
}
