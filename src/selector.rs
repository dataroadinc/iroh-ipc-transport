//! A [`PathSelector`] that prefers the IPC transport for co-located peers.

use iroh::endpoint::transports::{Addr, PathSelection, PathSelectionContext, PathSelector};

use crate::IPC_TRANSPORT_ID;

/// A [`PathSelector`] that selects the IPC (local-socket) path whenever one is
/// open to the peer, and otherwise picks the lowest-RTT path.
///
/// So a co-located peer (reachable over the shared Unix socket) rides the
/// socket, while every other peer uses UDP/relay exactly as before — and if
/// the socket goes away, iroh falls back automatically.
///
/// Install via
/// [`Builder::path_selector`](iroh::endpoint::Builder::path_selector).
#[derive(Debug, Default, Clone, Copy)]
pub struct PreferIpcTransport;

impl PathSelector for PreferIpcTransport {
    fn select(&self, ctx: &PathSelectionContext<'_>) -> PathSelection {
        let mut selection = PathSelection::none();

        // First preference: any open path on the IPC custom transport.
        if let Some(path) = ctx.paths().find(
            |p| matches!(p.network_path().remote(), Addr::Custom(c) if c.id() == IPC_TRANSPORT_ID),
        ) {
            selection.set(&path);
            return selection;
        }

        // Otherwise: lowest RTT wins. Paths whose stats can't be read (closed
        // concurrently with selection) are skipped.
        if let Some(path) = ctx
            .paths()
            .filter_map(|p| p.stats().map(|s| (p, s.rtt)))
            .min_by_key(|(_, rtt)| *rtt)
            .map(|(p, _)| p)
        {
            selection.set(&path);
        }
        selection
    }
}
