//! Full screen video under kmscon, with no root.
//!
//! kmscon holds DRM master on its VT and gives the client a pty, so a player
//! started from there can neither draw nor read keys. A *waiter* runs as the
//! user on a spare VT in its own logind session (a systemd unit the suite
//! installer sets up): the client sends it a file path, the waiter switches
//! the seat to its VT through logind (polkit allows an active session), runs
//! mpv there with the VT as its terminal, and switches back when mpv exits.
//! Nothing here needs root; the request socket is owner-only and the peer uid
//! is checked as well.

use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// The default spare VT (`tetron-wm fullscreen-waiter --vt N` overrides it).
pub const DEFAULT_VT: u32 = 9;

/// What to show. The waiter picks the program itself from this, so a request
/// can never name a command to run.
#[derive(Serialize, Deserialize, Debug, Default, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    /// One video, played with mpv.
    #[default]
    Video,
    /// Images, shown with fbi (or fim), first one first.
    Images,
    /// One PDF, shown with fbgs.
    Pdf,
}

impl Kind {
    pub fn parse(s: &str) -> Option<Kind> {
        match s {
            "video" => Some(Kind::Video),
            "images" => Some(Kind::Images),
            "pdf" => Some(Kind::Pdf),
            _ => None,
        }
    }
    fn name(self) -> &'static str {
        match self {
            Kind::Video => "video",
            Kind::Images => "images",
            Kind::Pdf => "pdf",
        }
    }
}

/// The most files one request may carry (a folder of images).
const MAX_FILES: usize = 2000;

/// One request: what to show, the absolute files, and the VT to come back to.
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct Request {
    #[serde(default)]
    pub kind: Kind,
    pub files: Vec<String>,
    pub return_vt: u32,
}

/// The owner-only socket the waiter listens on.
pub fn socket_path() -> PathBuf {
    let dir = std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        // SAFETY: getuid never fails.
        .unwrap_or_else(|| PathBuf::from(format!("/run/user/{}", unsafe { libc::getuid() })));
    dir.join("tetron-wm-fullscreen.sock")
}

/// Whether a waiter answers on its socket (a bare connect, which the waiter
/// treats as a ping).
pub fn waiter_available() -> bool {
    UnixStream::connect(socket_path()).is_ok()
}

/// Check a requested file. Only an absolute path to a regular file is played,
/// returned with symlinks resolved, so it can never be read as an option and a
/// directory, device or fifo is refused.
pub fn validate(path: &str) -> Result<PathBuf, String> {
    if path.contains('\0') {
        return Err("path has a NUL byte".into());
    }
    let p = Path::new(path);
    if !p.is_absolute() {
        return Err("path is not absolute".into());
    }
    let real = std::fs::canonicalize(p).map_err(|e| format!("cannot open the file: {e}"))?;
    let meta = std::fs::metadata(&real).map_err(|e| format!("cannot read the file: {e}"))?;
    if !meta.is_file() {
        return Err("not a regular file".into());
    }
    Ok(real)
}

/// The mpv command line: no user config or scripts, no youtube-dl hook, no
/// following references in a playlist, and `--` so the file is never an option.
pub fn mpv_args(file: &Path) -> Vec<String> {
    let mut v: Vec<String> = [
        "--no-config", "--load-scripts=no", "--ytdl=no", "--access-references=no",
        "--vo=drm", "--fs", "--msg-level=all=error", "--",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    v.push(file.to_string_lossy().into_owned());
    v
}

/// Images with fbi (or fim): autozoom, no status line, `--` before the files.
fn fbi_args(program: &str, files: &[PathBuf]) -> Vec<String> {
    let mut v: Vec<String> = if program == "fim" { vec!["-a".into()] } else { vec!["-a".into(), "--noverbose".into()] };
    v.push("--".into());
    v.extend(files.iter().map(|p| p.to_string_lossy().into_owned()));
    v
}

/// The program and arguments for a request, from which programs `have` says
/// exist. Files are already validated (absolute, regular).
fn command_for(kind: Kind, files: &[PathBuf], have: &dyn Fn(&str) -> bool) -> Result<(String, Vec<String>), String> {
    match kind {
        Kind::Video if files.len() == 1 && have("mpv") => Ok(("mpv".into(), mpv_args(&files[0]))),
        Kind::Video if files.len() == 1 => Err("mpv is not installed".into()),
        Kind::Images if !files.is_empty() => {
            let program = if have("fbi") { "fbi" } else if have("fim") { "fim" } else { return Err("neither fbi nor fim is installed".into()) };
            Ok((program.into(), fbi_args(program, files)))
        }
        // The path is absolute, so it can not be read as an option.
        Kind::Pdf if files.len() == 1 && have("fbgs") && have("gs") => Ok(("fbgs".into(), vec![files[0].to_string_lossy().into_owned()])),
        Kind::Pdf if files.len() == 1 => Err("fbgs and ghostscript (gs) are needed for a PDF".into()),
        _ => Err(format!("a {} request needs a different number of files", kind.name())),
    }
}

/// The VT that is showing now, from `/sys/class/tty/tty0/active` ("tty3").
pub fn active_vt() -> Option<u32> {
    let s = std::fs::read_to_string("/sys/class/tty/tty0/active").ok()?;
    s.trim().strip_prefix("tty")?.parse().ok()
}

/// Switch the seat to VT `n` through logind (no root: polkit allows an active
/// or inactive local session).
fn switch_vt(n: u32) -> Result<(), String> {
    let n = n.to_string();
    let args = [
        "--system", "call", "org.freedesktop.login1", "/org/freedesktop/login1/seat/seat0",
        "org.freedesktop.login1.Seat", "SwitchTo", "u", n.as_str(),
    ];
    match crate::system::run_capped_status("busctl", &args, 5) {
        crate::system::Capped::Done => Ok(()),
        crate::system::Capped::Failed(e) => Err(format!("switching to tty{n}: {e}")),
        crate::system::Capped::StillRunning => Err(format!("switching to tty{n} timed out")),
        crate::system::Capped::Missing(e) => Err(format!("busctl is not available: {e}")),
    }
}

/// Wait up to `secs` for VT `n` to be the active one.
fn wait_active(n: u32, secs: u64) -> bool {
    let end = std::time::Instant::now() + std::time::Duration::from_secs(secs);
    while std::time::Instant::now() < end {
        if active_vt() == Some(n) {
            return true;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    active_vt() == Some(n)
}

/// The uid on the other end of a unix socket (`SO_PEERCRED`).
fn peer_uid(s: &UnixStream) -> Option<u32> {
    use std::os::fd::AsRawFd;
    let mut cred = libc::ucred { pid: 0, uid: 0, gid: 0 };
    let mut len = std::mem::size_of::<libc::ucred>() as libc::socklen_t;
    // SAFETY: `cred` and `len` are valid for SO_PEERCRED, which fills a ucred.
    let rc = unsafe { libc::getsockopt(s.as_raw_fd(), libc::SOL_SOCKET, libc::SO_PEERCRED, (&mut cred as *mut libc::ucred).cast(), &mut len) };
    (rc == 0).then_some(cred.uid)
}

/// Waiter: serve play requests on this VT until killed. One at a time.
pub fn serve(vt: u32) -> std::io::Result<()> {
    serve_on(&socket_path(), vt)
}

fn serve_on(sock: &Path, vt: u32) -> std::io::Result<()> {
    if UnixStream::connect(sock).is_ok() {
        eprintln!("tetron-wm fullscreen-waiter: one is already running ({})", sock.display());
        return Ok(());
    }
    let _ = std::fs::remove_file(sock);
    // SAFETY: umask only changes this process's file creation mask.
    let old = unsafe { libc::umask(0o177) };
    let bound = UnixListener::bind(sock);
    // SAFETY: restores the mask saved above.
    unsafe { libc::umask(old) };
    let listener = bound?;
    crate::dbg_log(&format!("fullscreen waiter: listening on {} for tty{vt}", sock.display()));
    // SAFETY: getuid never fails.
    let me = unsafe { libc::getuid() };
    for conn in listener.incoming() {
        let Ok(mut conn) = conn else { continue };
        if peer_uid(&conn) != Some(me) {
            crate::dbg_log("fullscreen waiter: refused a connection from another user");
            continue;
        }
        let mut line = String::new();
        // 1 MiB holds a folder's worth of paths; a longer line is not a request.
        let read = BufReader::new((&conn).take(1 << 20)).read_line(&mut line);
        if read.map(|n| n == 0).unwrap_or(true) {
            continue; // a ping
        }
        let reply = match serde_json::from_str::<Request>(&line) {
            Err(e) => Err(format!("bad request: {e}")),
            Ok(req) => handle(vt, &req),
        };
        let text = match &reply {
            Ok(()) => "ok\n".to_string(),
            Err(e) => format!("error: {e}\n"),
        };
        if let Err(e) = &reply {
            crate::dbg_log(&format!("fullscreen waiter: {e}"));
        }
        let _ = conn.write_all(text.as_bytes());
    }
    Ok(())
}

/// Show one request: validate, switch here, run the program, switch back.
fn handle(vt: u32, req: &Request) -> Result<(), String> {
    if req.files.is_empty() || req.files.len() > MAX_FILES {
        return Err(format!("a request needs 1 to {MAX_FILES} files"));
    }
    let files = req.files.iter().map(|f| validate(f)).collect::<Result<Vec<_>, _>>()?;
    let (program, args) = command_for(req.kind, &files, &crate::catalog::is_installed)?;
    crate::dbg_log(&format!("fullscreen waiter: {} {} file(s) with {program}, first {} (back to tty{})", req.kind.name(), files.len(), files[0].display(), req.return_vt));
    switch_vt(vt)?;
    if !wait_active(vt, 3) {
        let _ = switch_vt(req.return_vt);
        return Err(format!("tty{vt} did not become active"));
    }
    // Let kmscon release the display before mpv asks for it.
    std::thread::sleep(std::time::Duration::from_millis(250));
    // Clear the VT and hide its cursor so neither shows around the video.
    print!("\x1b[2J\x1b[H\x1b[?25l");
    let _ = std::io::stdout().flush();
    let status = std::process::Command::new(&program).args(&args).status();
    print!("\x1b[?25h");
    let _ = std::io::stdout().flush();
    crate::dbg_log(&format!("fullscreen waiter: {program} ended with {status:?}"));
    switch_vt(req.return_vt)?;
    wait_active(req.return_vt, 3);
    match status {
        Ok(s) if s.success() => Ok(()),
        Ok(s) => Err(format!("{program} ended with {s}")),
        Err(e) => Err(format!("could not run {program}: {e}")),
    }
}

/// Client side: ask the waiter to show `files` and wait until it is done.
/// Returns the process exit code for `run_fullscreen`.
pub fn play(kind: Kind, files: &[String]) -> i32 {
    let fail = |msg: &str| {
        crate::dbg_log(&format!("fullscreen play: {msg}"));
        eprintln!("tetron-wm: {msg}");
        1
    };
    let Some(return_vt) = active_vt() else { return fail("cannot tell which VT is showing") };
    let mut abs = Vec::with_capacity(files.len());
    for f in files {
        match std::path::absolute(f) {
            Ok(p) => abs.push(p.to_string_lossy().into_owned()),
            Err(e) => return fail(&format!("bad path: {e}")),
        }
    }
    let mut conn = match UnixStream::connect(socket_path()) {
        Ok(c) => c,
        Err(e) => return fail(&format!("the full screen waiter is not running ({e})")),
    };
    let req = serde_json::to_string(&Request { kind, files: abs, return_vt }).unwrap_or_default();
    if conn.write_all(format!("{req}\n").as_bytes()).is_err() {
        return fail("could not send the request");
    }
    let mut reply = String::new();
    let _ = BufReader::new(&conn).read_line(&mut reply);
    match reply.trim() {
        "ok" => 0,
        "" => fail("the waiter closed the connection"),
        other => fail(other.strip_prefix("error: ").unwrap_or(other)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("tetron-wm-fs-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn validate_accepts_only_absolute_regular_files() {
        let d = tmp("validate");
        let f = d.join("clip.mp4");
        std::fs::write(&f, b"x").unwrap();
        assert_eq!(validate(f.to_str().unwrap()).unwrap(), std::fs::canonicalize(&f).unwrap());
        assert!(validate("clip.mp4").is_err(), "relative");
        assert!(validate("-x").is_err(), "leading dash is relative");
        assert!(validate(d.to_str().unwrap()).is_err(), "directory");
        assert!(validate("/nonexistent/clip.mp4").is_err(), "missing");
        assert!(validate("/dev/null").is_err(), "device");
        assert!(validate("/tmp/a\0b").is_err(), "NUL");
    }

    #[test]
    fn validate_resolves_symlinks() {
        let d = tmp("symlink");
        let f = d.join("real.mp4");
        std::fs::write(&f, b"x").unwrap();
        let l = d.join("link.mp4");
        std::os::unix::fs::symlink(&f, &l).unwrap();
        assert_eq!(validate(l.to_str().unwrap()).unwrap(), std::fs::canonicalize(&f).unwrap());
    }

    #[test]
    fn mpv_gets_hardening_flags_and_the_file_after_double_dash() {
        let args = mpv_args(Path::new("/v/-weird.mp4"));
        assert_eq!(args.last().unwrap(), "/v/-weird.mp4");
        assert_eq!(args[args.len() - 2], "--");
        for flag in ["--no-config", "--load-scripts=no", "--ytdl=no", "--access-references=no", "--vo=drm"] {
            assert!(args.iter().any(|a| a == flag), "{flag}");
        }
    }

    #[test]
    fn waiter_answers_bad_requests_with_errors_and_ignores_pings() {
        use std::os::unix::fs::PermissionsExt;
        let d = tmp("serve");
        let sock = d.join("w.sock");
        let s2 = sock.clone();
        std::thread::spawn(move || {
            let _ = serve_on(&s2, 9);
        });
        for _ in 0..100 {
            if UnixStream::connect(&sock).is_ok() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        assert_eq!(std::fs::metadata(&sock).unwrap().permissions().mode() & 0o777, 0o600, "owner only");
        let ask = |line: &str| {
            let mut c = UnixStream::connect(&sock).unwrap();
            c.write_all(line.as_bytes()).unwrap();
            let mut reply = String::new();
            BufReader::new(&c).read_line(&mut reply).unwrap();
            reply
        };
        assert!(ask("not json\n").starts_with("error: bad request"));
        assert!(ask("{\"files\":[\"relative.mp4\"],\"return_vt\":1}\n").starts_with("error: path is not absolute"));
        assert!(ask("{\"files\":[\"/nonexistent/x.mp4\"],\"return_vt\":1}\n").starts_with("error: cannot open"));
        assert!(ask("{\"files\":[],\"return_vt\":1}\n").starts_with("error: a request needs 1 to"));
        // A ping (connect and close) must not wedge the waiter.
        drop(UnixStream::connect(&sock).unwrap());
        assert!(ask("{\"kind\":\"images\",\"files\":[\"/dev/null\"],\"return_vt\":1}\n").starts_with("error: not a regular file"));
    }

    #[test]
    fn command_picks_the_program_per_kind_and_never_takes_one_from_the_request() {
        let f = |names: &[&str]| names.iter().map(PathBuf::from).collect::<Vec<_>>();
        let all = |_: &str| true;
        let none = |_: &str| false;
        let only = |n: &'static str| move |b: &str| b == n;
        let (p, a) = command_for(Kind::Video, &f(&["/v/a.mp4"]), &all).unwrap();
        assert_eq!((p.as_str(), a.last().unwrap().as_str()), ("mpv", "/v/a.mp4"));
        assert!(command_for(Kind::Video, &f(&["/a", "/b"]), &all).is_err(), "one video only");
        assert!(command_for(Kind::Video, &f(&["/a"]), &none).is_err());
        let (p, a) = command_for(Kind::Images, &f(&["/i/1.png", "/i/2.png"]), &all).unwrap();
        assert_eq!(p, "fbi");
        assert_eq!(a, ["-a", "--noverbose", "--", "/i/1.png", "/i/2.png"]);
        assert_eq!(command_for(Kind::Images, &f(&["/i/1.png"]), &only("fim")).unwrap().0, "fim");
        assert!(command_for(Kind::Images, &f(&["/i/1.png"]), &none).is_err());
        assert_eq!(command_for(Kind::Pdf, &f(&["/d/x.pdf"]), &all).unwrap(), ("fbgs".to_string(), vec!["/d/x.pdf".to_string()]));
        assert!(command_for(Kind::Pdf, &f(&["/d/x.pdf"]), &only("fbgs")).is_err(), "needs gs too");
        assert!(command_for(Kind::Images, &[], &all).is_err());
    }

    #[test]
    fn kind_names_round_trip() {
        for k in [Kind::Video, Kind::Images, Kind::Pdf] {
            assert_eq!(Kind::parse(k.name()), Some(k));
        }
        assert_eq!(Kind::parse("rm"), None);
    }

    #[test]
    fn request_round_trips() {
        let r = Request { kind: Kind::Images, files: vec!["/v/a b.png".into(), "/v/c.png".into()], return_vt: 1 };
        let back: Request = serde_json::from_str(&serde_json::to_string(&r).unwrap()).unwrap();
        assert_eq!(back, r);
    }
}
