//! # iroh-ipc-transport
//!
//! An **inter-process (local-socket) custom transport** for [iroh]:
//! co-located peers (two processes on the same host) connect over a local
//! IPC socket — a Unix domain socket, or a message-mode named pipe on
//! Windows — as a **native iroh path**, instead of iroh's UDP/relay
//! transports.
//!
//! Because it plugs into iroh's custom-transport API, the socket becomes
//! just another path on the one iroh `Connection`:
//!
//! - **QUIC multiplexes** streams over it (many `open_bi()` substreams on
//!   one socket — no per-stream dial);
//! - **iroh TLS authenticates** the peer (no bespoke identity preamble);
//! - **iroh path selection** prefers it for same-host peers (lowest RTT)
//!   and **falls back to UDP/relay automatically** if the socket is absent.
//!
//! This replaces the common pattern of a hand-rolled same-host side channel
//! (a `.data.sock` with its own framing, multiplexer, and accept loop) with
//! one that *is* part of the iroh network.
//!
//! ## Why a datagram socket
//!
//! iroh's `CustomTransport` is a **datagram** interface (`poll_send` /
//! `poll_recv` of packets), because QUIC runs over an unreliable datagram
//! substrate. Each platform therefore uses a **message-boundary-preserving**
//! local socket: Unix `SOCK_DGRAM` (`UnixDatagram`), Windows
//! `PIPE_TYPE_MESSAGE` named pipes (each read/write is one message) — never
//! a byte-stream pipe.
//!
//! ## Status
//!
//! **Work in progress / experimental.** Depends on iroh's
//! `unstable-custom-transports` feature, which is itself experimental and may
//! shift between iroh releases. See `README.md` for the design and roadmap.
//!
//! [iroh]: https://iroh.computer

#![deny(missing_docs)]

#[cfg(unix)]
mod addr;
#[cfg(unix)]
mod endpoint;
mod selector;
#[cfg(unix)]
mod transport;

#[cfg(unix)]
pub use addr::{ipc_custom_addr, path_from_custom_addr};
pub use selector::PreferIpcTransport;
#[cfg(unix)]
pub use transport::IpcTransport;

/// iroh custom-transport type id for this local-IPC transport, mixed into
/// every [`iroh_base::CustomAddr`] it produces so iroh routes only its own
/// addresses to it.
///
/// A random-looking constant, distinct from other custom transports sharing
/// an endpoint.
pub const IPC_TRANSPORT_ID: u64 = 0x69_70_63_5f_74_72_00_00; // "ipc_tr\0\0"
