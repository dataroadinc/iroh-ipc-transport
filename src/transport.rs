//! The IPC [`CustomTransport`] factory.

#![cfg(unix)]

use std::io;
use std::path::PathBuf;

use iroh::endpoint::transports::{CustomEndpoint, CustomTransport};
use iroh_base::CustomAddr;
use tokio::net::UnixDatagram;

use crate::addr::ipc_custom_addr;
use crate::endpoint::IpcEndpoint;

/// An iroh custom transport that binds a Unix `SOCK_DGRAM` socket at a fixed
/// path and serves it as a native iroh path for co-located peers.
///
/// Add it to an endpoint with
/// [`Builder::add_custom_transport`](iroh::endpoint::Builder::add_custom_transport),
/// advertise [`local_addr`](Self::local_addr) to peers (as a
/// [`TransportAddr::Custom`](iroh_base::TransportAddr::Custom)), and install
/// [`PreferIpcTransport`](crate::PreferIpcTransport) so iroh prefers the socket
/// for same-host peers.
#[derive(Debug, Clone)]
pub struct IpcTransport {
    socket_path: PathBuf,
}

impl IpcTransport {
    /// Create a transport that binds its datagram socket at `socket_path` when
    /// the iroh endpoint binds.
    pub fn new(socket_path: impl Into<PathBuf>) -> Self {
        Self {
            socket_path: socket_path.into(),
        }
    }

    /// This transport's local IPC address — advertise it to peers so they can
    /// dial the socket.
    pub fn local_addr(&self) -> CustomAddr {
        ipc_custom_addr(&self.socket_path)
    }
}

/// Requested `SO_RCVBUF`/`SO_SNDBUF` for the datagram socket. QUIC bursts many
/// packets for a single co-located `put`; the default AF_UNIX receive buffer
/// (~208 KiB) overflows mid-burst, backpressures the sender, and triggers
/// QUIC PTO backoff (multi-second stalls). 8 MiB comfortably holds a
/// page-sized burst. Best-effort: the kernel clamps to `net.core.rmem_max` /
/// `wmem_max`, so on an untuned host this is capped — the batched `poll_recv`
/// drain is the primary fix; this is defense in depth.
const IPC_SOCKET_BUFFER_BYTES: usize = 8 * 1024 * 1024;

impl CustomTransport for IpcTransport {
    fn bind(&self) -> io::Result<Box<dyn CustomEndpoint>> {
        // A stale socket file from a previous run would make `bind` fail.
        let _ = std::fs::remove_file(&self.socket_path);
        let socket = UnixDatagram::bind(&self.socket_path)?;
        // Best-effort buffer enlargement; a failure just leaves the OS default,
        // so never fail `bind` over it.
        let sock = socket2::SockRef::from(&socket);
        let _ = sock.set_recv_buffer_size(IPC_SOCKET_BUFFER_BYTES);
        let _ = sock.set_send_buffer_size(IPC_SOCKET_BUFFER_BYTES);
        Ok(Box::new(IpcEndpoint::new(socket, self.socket_path.clone())))
    }
}
