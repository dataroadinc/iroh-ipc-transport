//! The IPC [`CustomEndpoint`] and [`CustomSender`] over a Unix `SOCK_DGRAM`
//! socket.
//!
//! One `UnixDatagram` per iroh endpoint, shared (via `Arc`) between the
//! receiving [`CustomEndpoint::poll_recv`] and the [`CustomSender`] its
//! [`create_sender`](CustomEndpoint::create_sender) hands out. Because the
//! socket is *bound* to a path, a peer's `recv_from` learns the sender's path
//! — that path is the sender's [`CustomAddr`].

#![cfg(unix)]

use std::io;
use std::path::PathBuf;
use std::sync::Arc;
use std::task::{Context, Poll};

use iroh::endpoint::transports::{CustomEndpoint, CustomSender, RecvInfo, Transmit};
use iroh_base::CustomAddr;
use tokio::io::ReadBuf;
use tokio::net::UnixDatagram;

use crate::IPC_TRANSPORT_ID;
use crate::addr::{ipc_custom_addr, path_from_custom_addr};

/// A bound IPC endpoint: one `SOCK_DGRAM` Unix socket used for both send and
/// receive, advertising its own socket path as its local [`CustomAddr`].
#[derive(Debug)]
pub(crate) struct IpcEndpoint {
    socket: Arc<UnixDatagram>,
    local_addr: CustomAddr,
    watchable: n0_watcher::Watchable<Vec<CustomAddr>>,
}

impl IpcEndpoint {
    /// Wrap a `socket` bound at `local_path`.
    pub(crate) fn new(socket: UnixDatagram, local_path: PathBuf) -> Self {
        let local_addr = ipc_custom_addr(&local_path);
        let watchable = n0_watcher::Watchable::new(vec![local_addr.clone()]);
        Self {
            socket: Arc::new(socket),
            local_addr,
            watchable,
        }
    }
}

impl CustomEndpoint for IpcEndpoint {
    fn watch_local_addrs(&self) -> n0_watcher::Direct<Vec<CustomAddr>> {
        self.watchable.watch()
    }

    fn create_sender(&self) -> Arc<dyn CustomSender> {
        Arc::new(IpcSender {
            socket: Arc::clone(&self.socket),
        })
    }

    fn poll_recv(
        &mut self,
        cx: &mut Context<'_>,
        bufs: &mut [io::IoSliceMut<'_>],
        metas: &mut [noq_udp::RecvMeta],
        recv_infos: &mut [RecvInfo],
    ) -> Poll<io::Result<usize>> {
        if bufs.is_empty() {
            return Poll::Ready(Ok(0));
        }
        let mut read_buf = ReadBuf::new(&mut bufs[0]);
        match self.socket.poll_recv_from(cx, &mut read_buf) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
            Poll::Ready(Ok(src)) => {
                let len = read_buf.filled().len();
                // The sender is bound to its own path; that path is its address.
                // An unnamed sender (autobind) has no path — we can't route a
                // reply, so drop it.
                let Some(src_path) = src.as_pathname() else {
                    return Poll::Pending;
                };
                recv_infos[0] =
                    RecvInfo::new(ipc_custom_addr(src_path), Some(self.local_addr.clone()));
                metas[0].len = len;
                metas[0].stride = len;
                Poll::Ready(Ok(1))
            }
        }
    }
}

/// Sends packets to a peer's IPC socket path.
#[derive(Debug)]
struct IpcSender {
    socket: Arc<UnixDatagram>,
}

impl CustomSender for IpcSender {
    fn is_valid_send_addr(&self, addr: &CustomAddr) -> bool {
        addr.id() == IPC_TRANSPORT_ID
    }

    fn poll_send(
        &self,
        cx: &mut Context<'_>,
        dst: &CustomAddr,
        _src: Option<&CustomAddr>,
        transmit: &Transmit<'_>,
    ) -> Poll<io::Result<()>> {
        let Some(path) = path_from_custom_addr(dst) else {
            return Poll::Ready(Err(io::Error::other("not an IPC custom address")));
        };
        // max_transmit_segments defaults to 1, so `contents` is a single
        // datagram — send it whole.
        match self.socket.poll_send_to(cx, transmit.contents, &path) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Ok(_sent)) => Poll::Ready(Ok(())),
            Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
        }
    }
}
