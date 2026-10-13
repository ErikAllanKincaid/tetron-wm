//! Host system state (clock, CPU/mem, battery, WiFi, Bluetooth, volume) and the
//! traits that read and control it. Portable metrics come from the `sysinfo`
//! crate; WiFi/volume/Bluetooth go through a per-OS backend.

/// A full snapshot of host state, refreshed by the poller and read each frame.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SystemState {
    pub clock: ClockInfo,
    pub cpu_pct: f32,
    pub mem: MemInfo,
    pub battery: Option<BatteryInfo>,
    pub wifi: Option<WifiInfo>,
    /// The primary (default-route) network link, WiFi or not.
    pub net: Option<NetInfo>,
    pub bluetooth: BluetoothInfo,
    pub volume: VolumeInfo,
    pub known_networks: Vec<String>,
    pub caps: BackendCaps,
    /// Reachability of the saved remote systems (name → ssh port answered),
    /// refreshed by the poller so the Systems menu can show ●/○ dots.
    pub remotes_online: Vec<(String, bool)>,
    /// Upcoming calendar events (from `khal` when installed), soonest first.
    pub events: Vec<CalEvent>,
    /// tetron (P2P mesh VPN) status, polled from its daemon socket. Default =
    /// daemon not reachable (tetron may not be installed/running); the tray
    /// segment is hidden unless `tetron.reachable`.
    pub tetron: TetronInfo,
}

/// tetron daemon status for the menubar tray.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TetronInfo {
    /// The daemon's IPC socket answered a `Status` request this poll. When
    /// false, tetron is not installed/running and the tray segment is hidden.
    pub reachable: bool,
    /// The VPN is active (TUN up, networks connected), vs. reachable standby.
    pub active: bool,
    /// One entry per joined network.
    pub networks: Vec<TetronNetwork>,
}

/// One tetron network's summary for the tray.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TetronNetwork {
    /// Local display name of the network.
    pub name: String,
    /// Total members in the network.
    pub members: usize,
    /// Peers with a live connection right now.
    pub connected: usize,
    /// Any connected peer is a direct (not relayed) link.
    pub any_direct: bool,
    /// This network's data plane is up (vs. on standby). Drives the per-network
    /// connect/disconnect toggle.
    pub active: bool,
}

/// One upcoming calendar event for the menubar calendar popover.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CalEvent {
    /// Civil date of the event (local), for marking calendar days.
    pub year: i32,
    pub month: u32,
    pub day: u32,
    /// Display line, e.g. "09:30 Standup".
    pub text: String,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ClockInfo {
    pub time: String,
    pub date: String,
    pub uptime_secs: u64,
    /// Today's civil date (local time) for the menubar calendar; all zero until
    /// the first poll succeeds.
    pub year: i32,
    pub month: u32,
    pub day: u32,
}
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MemInfo {
    pub used: u64,
    pub total: u64,
}
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct BatteryInfo {
    pub pct: u8,
    pub charging: bool,
    /// Minutes to empty (discharging) or to full (charging), when known.
    pub mins: Option<u32>,
}
/// The primary network link: the interface carrying the default route.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct NetInfo {
    pub iface: String,
    pub ip: String,
    /// True for a WiFi interface; false for ethernet, tether, VPN, etc.
    pub wifi: bool,
}

/// Parse `ip -4 route get <addr>` output (`... dev wlan0 src 192.168.1.42 ...`)
/// into `(interface, ip)`.
pub fn parse_ip_route_get(out: &str) -> Option<(String, String)> {
    let w: Vec<&str> = out.split_whitespace().collect();
    let after = |key: &str| w.iter().position(|t| *t == key).and_then(|i| w.get(i + 1)).map(|s| s.to_string());
    Some((after("dev")?, after("src")?))
}

/// Parse `route -n get default` (macOS) into the interface name.
pub fn parse_route_n_get(out: &str) -> Option<String> {
    out.lines()
        .find_map(|l| l.trim().strip_prefix("interface:"))
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Format a minutes count as `h:mm`.
pub fn fmt_mins(m: u32) -> String {
    format!("{}:{:02}", m / 60, m % 60)
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WifiInfo {
    pub ssid: String,
    pub signal: u8,
    pub enabled: bool,
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BtDevice {
    pub name: String,
    pub addr: String,
    pub connected: bool,
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BluetoothInfo {
    pub enabled: bool,
    pub devices: Vec<BtDevice>,
}
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct VolumeInfo {
    pub level: u8,
    pub muted: bool,
}
/// Which controls the active backend can actually perform (a missing helper tool
/// flips a bit off, so its popover renders read-only).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct BackendCaps {
    pub volume: bool,
    pub wifi: bool,
    pub bluetooth: bool,
}

/// Four-segment signal bar, filled left-to-right for `signal` in 0..=4 (clamped).
pub fn bars_glyph(signal: u8) -> String {
    let n = signal.min(4) as usize;
    (0..4).map(|i| if i < n { '▮' } else { '·' }).collect()
}

/// Integer percentage of `used`/`total`, guarding division by zero.
pub fn mem_pct(used: u64, total: u64) -> u8 {
    if total == 0 {
        0
    } else {
        ((used as u128 * 100) / total as u128) as u8
    }
}

/// A control action requested by a popover click, dispatched by the session to
/// the active backend.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ControlIntent {
    VolumeUp,
    VolumeDown,
    VolumeSet(u8),
    ToggleMute,
    WifiSetEnabled(bool),
    WifiConnectKnown(String),
    BtSetEnabled(bool),
    BtConnect { addr: String, connect: bool },
    /// Activate (`Resume`) or deactivate (`Standby`) tetron. `network = None`
    /// toggles every joined network (daemon-wide); `Some(name)` toggles just
    /// that network. Handled by the session (sends to the tetron daemon),
    /// not the OS backend.
    TetronSetActive { network: Option<String>, active: bool },
    /// Open a terminal window showing `tetron status`, then an interactive
    /// shell where the full tetron CLI (invite/join/admin/…) is available.
    /// Handled by the session (launches a shell), not the OS backend.
    TetronOpenTerminal,
    /// Open `command args` in a new window (a tray popover's "Open <tool>"
    /// row: htop/top, free -h, jolt). Handled by the session, not the OS backend.
    OpenTool { title: String, command: String, args: Vec<String> },
    /// Step the menubar calendar popover a month back/forward. Handled by the
    /// session's tray state, not the OS backend.
    CalendarPrev,
    CalendarNext,
    /// Focus the window a notification came from / clear all notifications.
    /// Handled by the session, not the OS backend.
    NotifFocus(u64),
    NotifClear,
    /// Copy text to the clipboard (the network popover's IP row). Handled by
    /// the session, not the OS backend.
    CopyText(String),
}

/// The OS-specific slice of [`SystemState`] produced by a [`SystemMonitor`].
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SystemReadout {
    pub wifi: Option<WifiInfo>,
    pub net: Option<NetInfo>,
    pub bluetooth: BluetoothInfo,
    pub volume: VolumeInfo,
    pub known_networks: Vec<String>,
}

/// Reads the OS-specific parts of the snapshot. Portable metrics (CPU/mem/
/// battery) are filled in by the poller via `sysinfo`.
pub trait SystemMonitor: Send + Sync {
    fn read(&self) -> SystemReadout;
    fn caps(&self) -> BackendCaps;
}

/// Applies a [`ControlIntent`] to the host (best-effort, timeout-guarded).
pub trait SystemControl: Send + Sync {
    fn apply(&self, intent: &ControlIntent);
}

/// A backend is both a monitor and a controller.
pub trait Backend: SystemMonitor + SystemControl {}
impl<T: SystemMonitor + SystemControl> Backend for T {}

/// Fallback backend on unsupported targets: no caps, no-op control.
pub struct StubBackend;
impl SystemMonitor for StubBackend {
    fn read(&self) -> SystemReadout {
        SystemReadout::default()
    }
    fn caps(&self) -> BackendCaps {
        BackendCaps::default()
    }
}
impl SystemControl for StubBackend {
    fn apply(&self, _intent: &ControlIntent) {}
}

/// Pick the backend for the current OS. Returns a stub on unsupported targets.
pub fn backend() -> Box<dyn Backend> {
    #[cfg(target_os = "macos")]
    {
        Box::new(macos::MacBackend::new())
    }
    #[cfg(target_os = "linux")]
    {
        Box::new(linux::LinuxBackend::new())
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        Box::new(StubBackend)
    }
}

/// Run a command with a hard timeout, returning trimmed stdout on success.
/// Never blocks longer than `secs`; kills the child on timeout. This is the
/// **only** sanctioned way to shell out — it keeps a hung tool from freezing
/// any caller (the poller or a control dispatch).
pub fn run_capped(program: &str, args: &[&str], secs: u64) -> Option<String> {
    use std::process::{Command, Stdio};
    use std::time::{Duration, Instant};
    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let deadline = Instant::now() + Duration::from_secs(secs);
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let out = child.wait_with_output().ok()?;
                if !status.success() {
                    return None;
                }
                return String::from_utf8(out.stdout).ok();
            }
            Ok(None) => {
                if Instant::now() >= deadline {
                    let _ = child.kill();
                    return None;
                }
                std::thread::sleep(Duration::from_millis(20));
            }
            Err(_) => return None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn route_get_parses_dev_and_src() {
        let out = "1.1.1.1 via 192.168.1.1 dev wlan0 src 192.168.1.42 uid 1000 \n    cache";
        assert_eq!(parse_ip_route_get(out), Some(("wlan0".into(), "192.168.1.42".into())));
        assert_eq!(parse_ip_route_get("RTNETLINK answers: Network is unreachable"), None);
    }

    #[test]
    fn macos_route_get_parses_interface() {
        let out = "   route to: default\n    gateway: 192.168.1.1\n  interface: en0\n";
        assert_eq!(parse_route_n_get(out).as_deref(), Some("en0"));
        assert_eq!(parse_route_n_get("route: writing to routing socket: not in table"), None);
    }

    #[test]
    fn mins_format() {
        assert_eq!(fmt_mins(125), "2:05");
        assert_eq!(fmt_mins(9), "0:09");
    }
}

#[cfg(target_os = "macos")]
pub mod macos;
#[cfg(target_os = "linux")]
pub mod linux;
/// Cross-platform tetron daemon status probe (Unix socket; any OS tetron runs on).
pub mod tetron;
