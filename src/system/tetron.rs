//! tetron (P2P mesh VPN) status probe for the menubar tray.
//!
//! Connects to the tetron daemon's Unix socket and sends one
//! [`IpcMessage::Status`], mapping the reply into a [`TetronInfo`]. tetron is a
//! separate, optional product; `tetron-proto` is the single source of truth for
//! the wire protocol, shared with tetron-systray and tetron-webui.
//!
//! The crate's own client helpers are async (tokio `Framed`/`MsgpackCodec`), but
//! the poller is a synchronous background thread that needs exactly one blocking
//! round-trip. Rather than pull the async runtime into that path, we frame the
//! request ourselves — the wire format is a 4-byte big-endian length prefix plus
//! an `rmp_serde::to_vec_named` msgpack body (see `tetron_proto::ipc`'s
//! `MsgpackCodec`). The decoder accepts named or positional msgpack, so this
//! stays compatible with the daemon.

use crate::system::{TetronInfo, TetronNetwork};
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::time::Duration;
use tetron_proto::ipc::{self, ConnType, IpcMessage};

/// Matches `tetron_proto::ipc::MAX_FRAME_LEN` — reject a bogus/hostile length so
/// a malformed reply can't make us allocate an unbounded buffer.
const MAX_FRAME_LEN: usize = 1_048_576;

/// Poll the tetron daemon for status. Returns a default (not reachable)
/// [`TetronInfo`] when the daemon is not answering — socket missing, connect or
/// I/O timeout, or an unexpected reply. tetron simply may not be installed or
/// running, which is not an error; the tray then hides the segment.
pub fn poll() -> TetronInfo {
    probe().unwrap_or_default()
}

fn probe() -> Option<TetronInfo> {
    let mut stream = UnixStream::connect(ipc::socket_path()).ok()?;
    // Bound every blocking call so a stuck daemon can never hang the poller.
    let timeout = Duration::from_secs(1);
    stream.set_read_timeout(Some(timeout)).ok()?;
    stream.set_write_timeout(Some(timeout)).ok()?;

    // Request frame: [u32 BE length][msgpack body].
    let body = rmp_serde::to_vec_named(&IpcMessage::Status).ok()?;
    stream.write_all(&(body.len() as u32).to_be_bytes()).ok()?;
    stream.write_all(&body).ok()?;
    stream.flush().ok()?;

    // Response frame: same framing.
    let mut len_buf = [0u8; 4];
    stream.read_exact(&mut len_buf).ok()?;
    let len = u32::from_be_bytes(len_buf) as usize;
    if len == 0 || len > MAX_FRAME_LEN {
        return None;
    }
    let mut buf = vec![0u8; len];
    stream.read_exact(&mut buf).ok()?;
    let msg: IpcMessage = rmp_serde::from_slice(&buf).ok()?;

    match msg {
        IpcMessage::StatusResponse { active, networks, .. } => Some(TetronInfo {
            reachable: true,
            active,
            networks: networks
                .into_iter()
                .map(|n| {
                    let connected = n.peers.iter().filter(|p| p.connection.is_some()).count();
                    let any_direct = n.peers.iter().any(|p| {
                        matches!(
                            p.connection.as_ref().map(|c| &c.conn_type),
                            Some(ConnType::Direct)
                        )
                    });
                    TetronNetwork {
                        name: n.network,
                        members: n.member_count,
                        connected,
                        any_direct,
                    }
                })
                .collect(),
        }),
        // The daemon answered but not with a status (e.g. an Error). Treat as
        // reachable-but-unknown rather than inventing network data.
        _ => Some(TetronInfo { reachable: true, active: false, networks: Vec::new() }),
    }
}
