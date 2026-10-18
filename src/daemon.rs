//! The tetron-wm daemon: owns the [`SessionCore`] (windows + PTYs) and serves frames
//! to one attached client at a time over a Unix socket. Apps keep running while
//! detached, so reattaching restores the live session.

use crate::compositor::Compositor;
use crate::config::Config;
use crate::protocol::{daemon_ctl_path, socket_path, DaemonStatus, Flags, FrameMsg};
use crate::session::{ClientMsg, SessionCore};
use std::fs::Permissions;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::os::unix::process::CommandExt;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use std::time::Duration;

/// Out-of-band control messages (`tetron-wm launch/tile/theme/msg`) queued by the
/// control-socket thread and drained into the session by the render loop.
type CtlQueue = Arc<std::sync::Mutex<Vec<ClientMsg>>>;

/// Shared out-of-band control state, set by the control-socket thread and polled
/// by the render loop: 0 = none, 1 = shutdown, 2 = reload.
const CTL_NONE: u8 = 0;
const CTL_SHUTDOWN: u8 = 1;
const CTL_RELOAD: u8 = 2;

/// Run the daemon event loop until `tetron-wm kill` (or a fatal socket error).
pub fn run() -> std::io::Result<()> {
    crate::dbg_init();
    // Confine the socket to a per-user 0700 directory so other local users
    // cannot connect to (and thus drive/observe) the session. Creating the
    // restrictive directory first avoids any window where the socket is exposed.
    crate::protocol::ensure_socket_dir()?;
    crate::dbg_log(&format!("daemon: session '{}'", crate::protocol::session_name()));

    let path = socket_path();
    let _ = std::fs::remove_file(&path);
    let listener = UnixListener::bind(&path)?;
    std::fs::set_permissions(&path, Permissions::from_mode(0o600))?;

    // Out-of-band control socket so `tetron-wm kill` / `tetron-wm reload` are honored even
    // while a client is attached (the client socket is served serially, so a
    // control message there would queue behind the attached client forever). A
    // listener thread reads Shutdown/Reload and flips this shared flag, which the
    // render loop polls each tick.
    let ctl_path = daemon_ctl_path();
    let _ = std::fs::remove_file(&ctl_path);
    let ctl = Arc::new(AtomicU8::new(CTL_NONE));
    let ctl_queue: CtlQueue = Arc::new(std::sync::Mutex::new(Vec::new()));
    // The clipboard state is created with the session below; the control thread
    // reads it through this slot (`tetron-wm clip`, the xclip/xsel shims).
    let clip_slot: Arc<std::sync::OnceLock<crate::clipboard::SharedClip>> = Arc::new(std::sync::OnceLock::new());
    // Published by the render loop for `tetron-wm ls` (answered on the control thread).
    let status = Arc::new(DaemonStatus::default());
    if let Ok(ctl_listener) = UnixListener::bind(&ctl_path) {
        let _ = std::fs::set_permissions(&ctl_path, Permissions::from_mode(0o600));
        let ctl_flag = Arc::clone(&ctl);
        let queue = Arc::clone(&ctl_queue);
        let slot = Arc::clone(&clip_slot);
        let status = Arc::clone(&status);
        std::thread::spawn(move || serve_control(ctl_listener, ctl_flag, queue, slot, status));
    }

    let cfg = Config::load();
    crate::theme::set(&cfg.theme);
    crate::iconset::set(cfg.icon_style);
    let (w, h) = (100, 30); // provisional until the first client reports its size
    let apphost = ensure_apphost()?;
    let mut core = SessionCore::with_apphost(w, h, cfg.clone(), Box::new(apphost));
    // App-managed state (icon positions, recent dirs) lives in state.toml, not config.toml.
    if let Some(p) = crate::state::state_path() {
        core.attach_state(crate::state::State::load_migrating(&cfg), p);
    }
    let _ = clip_slot.set(core.clip_handle());
    if cfg.clipboard_history {
        if let Some(b) = crate::clipbridge::detect(&cfg.clipboard_gui_bridge) {
            crate::clipbridge::spawn(core.clip_handle(), b);
        }
    }
    if let Some(p) = crate::clipboard::history_path() {
        core.enable_clipboard_persistence(p);
    }
    // Start the background system poller and feed its shared snapshot to the
    // session so the menubar tray reflects live host state. Keep `_poller` bound
    // for the daemon's lifetime so its thread is not detached/dropped.
    let _poller = crate::poller::SystemPoller::start();
    core.attach_tray_state(_poller.state());
    // Rebuild windows for any apps the apphost already owns (reload / crash
    // recovery). Only auto-launch the configured apps on a truly fresh start.
    // Auto-launch the configured apps ONLY on a genuinely fresh apphost. If the
    // apphost already owns apps (reload / crash recovery), restore their windows
    // instead — never re-launch, or we'd spawn duplicates of apps that are still
    // alive (e.g. when a window's restore meta is missing).
    if core.host_app_count() == 0 {
        for app in &cfg.apps {
            core.apply(ClientMsg::Launch {
                name: app.name.clone(),
                command: app.command.clone(),
                args: app.args.clone(),
            });
        }
    } else {
        let restored = core.restore_windows_from_host();
        crate::dbg_log(&format!("frontend: restored {restored} app window(s) from apphost"));
        // Wait briefly for the apphost to stream each restored app's first frame
        // so the first paint shows real content instead of a blank window flash.
        for _ in 0..30 {
            if core.app_windows_ready() {
                break;
            }
            std::thread::sleep(Duration::from_millis(16));
        }
    }
    // Safety check: if the running apphost is older than this binary can
    // safely talk to AND owns live apps, raise the "restart app server"
    // dialog so the user can save work first (instead of silent breakage).
    core.check_apphost_compat();
    // Honour a one-shot UI reopen hint (e.g. reopen Settings → Updates after an
    // in-app update reloaded the frontend).
    core.reopen_ui_from_hint();
    status.windows.store(core.window_count(), std::sync::atomic::Ordering::Relaxed);
    let mut comp = Compositor::new(w, h);

    // Connections are accepted on a thread and handed over as events, so a new
    // `attach` is seen even while a client is being served (refused, or it takes
    // over with `attach -d`). While unattached the loop blocks on the channel;
    // `tetron-wm kill`/`reload` also poke the main socket, which arrives here as
    // `Conn::Poke` so the control flag is re-checked either way.
    let conns = spawn_acceptor(listener);
    let mut reloading = false;
    let mut pending: Option<Conn> = None;
    loop {
        let conn = match pending.take() {
            Some(c) => c,
            None => match conns.recv() {
                Ok(c) => c,
                Err(_) => break,
            },
        };
        if apply_ctl(&ctl, &ctl_queue, &mut core) {
            // A control message arrived while unattached; this connection is the
            // wake poke. Don't serve it as a client.
        } else if let Conn::Client(client) = conn {
            if let Some(next) = serve_client(&mut core, &mut comp, client, &ctl, &ctl_queue, &conns, &status) {
                pending = Some(Conn::Client(next));
            }
        }
        core.clear_quit();
        if core.shutdown_requested() {
            break;
        }
        if core.reload_requested() {
            reloading = true;
            break;
        }
    }
    core.flush_clipboard();
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(&ctl_path);
    if !reloading {
        core.shutdown(); // full stop: kills apps + tells the apphost to exit
    } else {
        crate::dbg_log("daemon: reload — exiting to restart, apphost preserved");
    }
    // On reload we just drop `core` (its RemoteAppHost disconnects); the apphost
    // keeps running so the next frontend can restore the apps.
    Ok(())
}

/// A connection to the main socket, after its handshake.
enum Conn {
    /// No client: a wake-up poke from `tetron-wm kill`/`reload` (they write a
    /// control message and close), a stalled or garbage connection.
    Poke,
    Client(ClientConn),
}

/// An attaching client: its first message (always `Resize`) already read.
struct ClientConn {
    reader: BufReader<UnixStream>,
    writer: UnixStream,
    first: ClientMsg,
}

impl ClientConn {
    /// `attach -d`: the client asked to evict whoever is attached.
    fn wants_takeover(&self) -> bool {
        matches!(self.first, ClientMsg::Resize { takeover: true, .. })
    }
}

/// Accept connections for the daemon's lifetime; each gets a short handshake
/// thread so a slow or silent peer never blocks the next `attach`.
fn spawn_acceptor(listener: UnixListener) -> mpsc::Receiver<Conn> {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(stream) = stream else { continue };
            let tx = tx.clone();
            std::thread::spawn(move || {
                let _ = tx.send(handshake(stream));
            });
        }
    });
    rx
}

/// Read the first line. A `Resize` makes it a client; anything else (control
/// message poke, EOF, timeout, garbage) is a [`Conn::Poke`].
fn handshake(stream: UnixStream) -> Conn {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
    let Ok(writer) = stream.try_clone() else { return Conn::Poke };
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    if let Ok(n) = reader.read_line(&mut line) {
        if n > 0 {
            if let Ok(first @ ClientMsg::Resize { .. }) = serde_json::from_str::<ClientMsg>(line.trim()) {
                let _ = reader.get_ref().set_read_timeout(None);
                return Conn::Client(ClientConn { reader, writer, first });
            }
        }
    }
    Conn::Poke
}

/// A final frame that tells a client to exit and print `notice`.
fn notice_frame(notice: &str) -> Vec<u8> {
    let mut buf = serde_json::to_vec(&FrameMsg {
        changes: Vec::new(),
        cursor: None,
        flags: Flags { detach: true, ..Default::default() },
        images: Vec::new(),
        image_data: Vec::new(),
        clear: false,
        switch_to: None,
        clipboard: None,
        notice: Some(notice.to_string()),
    })
    .unwrap_or_default();
    buf.push(b'\n');
    buf
}

/// Ensure the apphost process is running and return a connected handle. Spawns
/// `tetron-wm --apphost` (detached) if its socket is absent, then connects.
fn ensure_apphost() -> std::io::Result<crate::apphost::RemoteAppHost> {
    use crate::protocol::apphost_socket_path;
    let path = apphost_socket_path();
    if UnixStream::connect(&path).is_err() {
        // Prefer the installed per-user service so the *supervised* apphost owns
        // the apps. On macOS that one runs in the GUI login session, which is why
        // Keychain-backed logins (e.g. the Claude Code CLI) work inside it — a
        // detached on-demand apphost we spawn here would not. Only spawn our own
        // when no service is installed, or as a fallback if the service didn't
        // bring its socket up in time.
        // The per-user service supervises the default session's apphost only; a
        // named session always gets its own detached one.
        let used_service = crate::protocol::session_name() == crate::protocol::DEFAULT_SESSION && crate::service::ensure_started();
        if !used_service {
            spawn_detached_apphost()?;
        }
        if !wait_for_socket(&path) && used_service {
            // The service was installed but didn't start in time; fall back so the
            // daemon still gets a working apphost rather than failing to connect.
            spawn_detached_apphost()?;
            wait_for_socket(&path);
        }
    }
    crate::apphost::RemoteAppHost::connect(&path)
}

/// Spawn `tetron-wm --apphost` detached into its own process group.
fn spawn_detached_apphost() -> std::io::Result<()> {
    let exe = std::env::current_exe()?;
    std::process::Command::new(exe)
        .arg("--apphost")
        .env("TETRON_WM_SESSION", crate::protocol::session_name())
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .process_group(0)
        .spawn()?;
    Ok(())
}

/// Poll the apphost socket for up to ~5s. Returns whether it came up.
fn wait_for_socket(path: &std::path::Path) -> bool {
    for _ in 0..100 {
        if UnixStream::connect(path).is_ok() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    false
}

/// Serve one attached client, publishing attached/window-count for `tetron-wm ls`.
/// Returns the next client when this one was evicted by `attach -d`.
fn serve_client(
    core: &mut SessionCore,
    comp: &mut Compositor,
    client: ClientConn,
    ctl: &Arc<AtomicU8>,
    ctl_queue: &CtlQueue,
    conns: &mpsc::Receiver<Conn>,
    status: &DaemonStatus,
) -> Option<ClientConn> {
    use std::sync::atomic::Ordering::Relaxed;
    status.attached.store(true, Relaxed);
    let next = serve_client_inner(core, comp, client, ctl, ctl_queue, conns, status);
    // On takeover the next client re-sets this at once.
    status.attached.store(false, Relaxed);
    status.windows.store(core.window_count(), Relaxed);
    next
}

/// Serve one attached client until it detaches (socket closes) or asks to detach.
fn serve_client_inner(
    core: &mut SessionCore,
    comp: &mut Compositor,
    client: ClientConn,
    ctl: &Arc<AtomicU8>,
    ctl_queue: &CtlQueue,
    conns: &mpsc::Receiver<Conn>,
    status: &DaemonStatus,
) -> Option<ClientConn> {
    let ClientConn { reader: mut r, mut writer, first } = client;

    // Read client messages on a thread; the main loop drains them and ticks.
    // The handshake already consumed the first message (a `Resize`): replay it
    // so it takes the normal path.
    let (tx, rx) = mpsc::channel::<ClientMsg>();
    let _ = tx.send(first);
    std::thread::spawn(move || {
        let mut line = String::new();
        loop {
            line.clear();
            match r.read_line(&mut line) {
                Ok(0) | Err(_) => break, // EOF == detached
                Ok(_) => {
                    if let Ok(msg) = serde_json::from_str::<ClientMsg>(line.trim()) {
                        if tx.send(msg).is_err() {
                            break;
                        }
                    }
                }
            }
        }
    });

    // Force a full repaint for the freshly attached client.
    comp.resize(comp.width(), comp.height());
    // Image ids whose bytes this client has already received (reset per attach,
    // mirroring the full cell-repaint above).
    let mut sent_image_ids: std::collections::HashSet<u64> = std::collections::HashSet::new();
    // Ask the client to wipe the terminal (cells + images) before the next
    // frame. Set on attach and on every resize: the emulator may have reflowed
    // cells and kept image placements the incremental diff/delete stream no
    // longer knows about, so only a full re-baseline is trustworthy.
    let mut clear_pending = true;

    loop {
        // A new connection while this client is attached: `attach -d` evicts
        // us; a plain attach is refused with a message.
        while let Ok(conn) = conns.try_recv() {
            let Conn::Client(mut other) = conn else { continue };
            if other.wants_takeover() {
                crate::dbg_log("daemon: attach -d — evicting the attached client");
                let _ = writer.write_all(&notice_frame("detached by another client (attach -d)"));
                let _ = writer.shutdown(std::net::Shutdown::Both);
                return Some(other);
            }
            crate::dbg_log("daemon: attach refused — a client is already attached");
            let _ = other.writer.write_all(&notice_frame(
                "session already attached from another terminal; use `tetron-wm attach -d` to take it over",
            ));
        }
        // Apply all pending input.
        loop {
            match rx.try_recv() {
                Ok(ClientMsg::Resize { w, h, .. }) => {
                    comp.resize(w, h);
                    core.apply(ClientMsg::Resize { w, h, takeover: false });
                    clear_pending = true;
                    // The client's wipe frees transmitted image data; re-send it.
                    sent_image_ids.clear();
                }
                Ok(msg) => core.apply(msg),
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => return None, // client gone
            }
        }
        // Out-of-band `tetron-wm kill` / `tetron-wm reload` (control socket) — honored even
        // though this client is attached.
        apply_ctl(ctl, ctl_queue, core);
        if core.shutdown_requested() {
            return None;
        }

        core.reap_dead();
        status.windows.store(core.window_count(), std::sync::atomic::Ordering::Relaxed);
        core.pump_app_events();
        core.refresh_app_graphics();
        core.sync_app_meta();
        core.pump_thumbnails();
        core.refresh_activity();
        core.poll_desktop_dir();
        let frame = core.build_frame();
        let shot = core.take_screenshot();
        let composed = comp.composite(&frame.layers, frame.cursor);
        if let Some(path) = shot {
            match crate::screenshot::save_png(composed, std::path::Path::new(&path)) {
                Ok(()) => crate::dbg_log(&format!("screenshot: wrote {path}")),
                Err(e) => crate::dbg_log(&format!("screenshot: {path}: {e}")),
            }
        }
        let changes = comp.diff();
        let flags = Flags {
            launcher_open: core.launcher_open(),
            spotlight_open: core.spotlight_open(),
            store_focused: core.focused_is_store(),
            settings_focused: core.focused_is_settings(),
            settings_editing: core.settings_editing(),
            dirpicker_open: core.dirpicker_open(),
            dirpicker_creating: core.dirpicker_creating(),
            help_open: core.help_open(),
            power_dialog: core.power_dialog_open(),
            no_window: core.focused().is_none(),
            clipmenu_open: core.clipmenu_open(),
            filemanager_focused: core.focused_is_filemanager(),
            filemanager_editing: core.filemanager_editing(),
            filemanager_context: core.filemanager_context(),
            desktop_editing: core.desktop_editing(),
            renaming: core.renaming(),
            confirm_close: core.confirm_close_open(),
            launch_warn: core.launch_warn_open(),
            power_editing: core.power_form_editing(),
            logs_focused: core.focused_is_logs(),
            activity_focused: core.focused_is_activity(),
            activity_confirming: core.activity_confirming(),
            detach: core.quit_requested(),
            reload: core.reload_requested(),
            app_area: core.app_mouse_area(),
            app_cursor: core.app_cursor_keys(),
        };
        // Send PNG bytes once per image id (base64); later frames carry only the
        // small placement list.
        let mut image_data = Vec::new();
        for p in &frame.images {
            if p.visible && sent_image_ids.insert(p.id) {
                if let Some(png) = core.image_png(p.id) {
                    image_data.push(crate::protocol::ImageBlob {
                        id: p.id,
                        png_base64: crate::kitty::b64(&png),
                    });
                }
            }
        }
        let mut buf = serde_json::to_vec(&FrameMsg {
            changes,
            cursor: frame.cursor,
            flags,
            images: frame.images.clone(),
            image_data,
            clear: clear_pending,
            switch_to: core.switch_spec(),
            clipboard: core.take_clipboard(),
            notice: None,
        })
            .unwrap_or_default();
        buf.push(b'\n');
        if writer.write_all(&buf).is_err() {
            return None; // client gone
        }
        comp.commit();
        clear_pending = false;

        if core.quit_requested() {
            if let Some(spec) = core.switch_spec() {
                crate::dbg_log(&format!("daemon: switch frame delivered (→ {} {})", spec.name, spec.host));
            }
            return None; // detach (flag already delivered)
        }
        if core.reload_requested() {
            return None; // reload flag delivered; daemon will restart, apphost untouched
        }
        std::thread::sleep(Duration::from_millis(16));
    }
}

/// If a control message has arrived, apply it to `core` (setting its
/// shutdown/reload flag, which the loops already act on) and return `true`.
fn apply_ctl(ctl: &Arc<AtomicU8>, queue: &CtlQueue, core: &mut SessionCore) -> bool {
    // Drain under the lock into a local batch, then release it before applying:
    // `core.apply` can be slow (spawns, theme reload), and holding the lock
    // across it would block the control thread's push. `into_inner` recovers a
    // poisoned lock so one panic can't wedge the control path forever.
    let batch: Vec<ClientMsg> = {
        let mut q = queue.lock().unwrap_or_else(|e| e.into_inner());
        q.drain(..).collect()
    };
    for msg in batch {
        core.apply(msg);
    }
    match ctl.swap(CTL_NONE, Ordering::SeqCst) {
        CTL_SHUTDOWN => {
            core.apply(ClientMsg::Shutdown);
            true
        }
        CTL_RELOAD => {
            core.apply(ClientMsg::Reload);
            true
        }
        _ => false,
    }
}

/// Accept control connections for the daemon's lifetime, flipping the shared flag
/// on `Shutdown` / `Reload`. Each connection is one short-lived message from the
/// `tetron-wm kill` / `tetron-wm reload` CLI.
fn serve_control(
    listener: UnixListener,
    ctl: Arc<AtomicU8>,
    queue: CtlQueue,
    clip: Arc<std::sync::OnceLock<crate::clipboard::SharedClip>>,
    status: Arc<DaemonStatus>,
) {
    for stream in listener.incoming() {
        let Ok(stream) = stream else { continue };
        let mut r = BufReader::new(stream);
        let mut line = String::new();
        if r.read_line(&mut line).is_ok() {
            match serde_json::from_str::<ClientMsg>(line.trim()) {
                Ok(ClientMsg::Shutdown) => ctl.store(CTL_SHUTDOWN, Ordering::SeqCst),
                Ok(ClientMsg::Reload) => ctl.store(CTL_RELOAD, Ordering::SeqCst),
                // `tetron-wm ls`: one JSON line back; an older daemon never replies.
                Ok(ClientMsg::Status) => {
                    if let Ok(mut buf) = serde_json::to_vec(&status.snapshot()) {
                        buf.push(b'\n');
                        let _ = std::io::Write::write_all(r.get_mut(), &buf);
                    }
                }
                // Clipboard requests are answered right here (one JSON line back),
                // so they work with no client attached and never wait on a frame.
                Ok(ClientMsg::Clip(cmd)) => {
                    let reply = match clip.get() {
                        Some(shared) => crate::clipboard::handle_cmd(shared, cmd),
                        None => crate::clipboard::ClipReply { error: "daemon is still starting".into(), ..Default::default() },
                    };
                    if let Ok(mut buf) = serde_json::to_vec(&reply) {
                        buf.push(b'\n');
                        let _ = std::io::Write::write_all(r.get_mut(), &buf);
                    }
                }
                // Any other message (e.g. `tetron-wm launch/tile/theme/msg` from the
                // CLI or the desktop assistant) queues for the render loop.
                Ok(msg) => {
                    crate::dbg_log(&format!("ctl: queued {msg:?}"));
                    queue.lock().unwrap_or_else(|e| e.into_inner()).push(msg);
                }
                Err(_) => {}
            }
        }
    }
}
