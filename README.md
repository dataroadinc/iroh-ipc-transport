# iroh-ipc-transport

A **local-socket custom transport** for [iroh](https://iroh.computer): two
processes on the same host connect over a **Unix domain socket** (or, on
Windows, a **message-mode named pipe**) as a **native iroh path** — instead of
iroh's UDP/relay transports.

## Why

The usual way to give co-located peers a fast local path is a hand-rolled side
channel: a `.data.sock`, its own framing, its own multiplexer, its own accept
loop, its own identity handshake, its own relay fallback. That connection lives
*outside* the iroh network — a whole parallel transport masquerading as an
optimization.

This crate instead plugs the local socket into iroh's **custom-transport API**,
so it becomes just another path on the one iroh `Connection`:

- **QUIC multiplexes** streams over it — many `open_bi()` substreams on one
  socket, no per-stream dial, real per-stream flow control (no head-of-line
  blocking).
- **iroh TLS authenticates** the peer — no bespoke "claimed, not authenticated"
  identity preamble.
- **iroh path selection** prefers it for same-host peers (lowest RTT) and
  **falls back to UDP/relay automatically** when the socket is absent — no
  bespoke fallback logic.
- Everything collapses back to `conn.open_bi()` over the one connection.

## How it works

iroh's `CustomTransport` is a **datagram** interface (`poll_send` / `poll_recv`
of packets), because QUIC runs over an unreliable datagram substrate. So each
platform uses a **message-boundary-preserving** local socket:

| Platform | Socket | Why |
|---|---|---|
| Unix | `SOCK_DGRAM` `UnixDatagram` | natively datagram |
| Windows | `PIPE_TYPE_MESSAGE` named pipe | each read/write is one discrete message — datagram-like |

A **byte-stream** pipe/socket would *not* fit (you'd have to frame QUIC packets
yourself); the message primitives above map directly onto `poll_recv`/
`poll_send`. Both live behind a private `ipc` seam so the iroh glue on top is
platform-agnostic.

Addressing: a peer's `CustomAddr` is `(IPC_TRANSPORT_ID, key_bytes)`; the
transport resolves the peer's `EndpointId` to its local socket path / pipe name
by convention. A `PathSelector` (`PreferIpcTransport`) tells iroh to prefer
this path whenever a same-host candidate exists.

## Crate layout (roadmap)

- `socket` — the platform seam: `#[cfg(unix)]` `UnixDatagram`,
  `#[cfg(windows)]` message-mode named pipe, behind one datagram trait.
- `addr` — `CustomAddr` ⇆ socket address mapping + `IPC_TRANSPORT_ID`.
- `endpoint` — `CustomEndpoint` (`watch_local_addrs`, `poll_recv`) +
  `CustomSender` (`poll_send`, `is_valid_send_addr`).
- `transport` — `IpcTransport` implementing `CustomTransport`.
- `selector` — `PreferIpcTransport: PathSelector`.

## Status

**Work in progress / experimental.** Depends on iroh's
`unstable-custom-transports` feature — experimental and may change between iroh
releases. Not yet published.

Milestones:

- [ ] Unix `SOCK_DGRAM` transport + two-endpoint echo test (native `is_relay=false` path)
- [ ] `PreferIpcTransport` path selector
- [ ] Windows message-mode named-pipe backend
- [ ] Publish to crates.io

## License

MIT OR Apache-2.0.

## Prior art

Modeled on iroh's own custom transports:
[iroh-tor-transport](https://github.com/n0-computer/iroh-tor-transport),
[iroh-ble-transport](https://github.com/mcginty/iroh-ble-transport), and iroh's
in-tree `test_utils::test_transport` + `examples/custom-transport.rs`.
