//! Mapping between a local socket path and an iroh [`CustomAddr`].
//!
//! The address data is simply the raw socket-path bytes: an iroh transport
//! address is opaque (peer *identity* is authenticated by QUIC/TLS, not by the
//! address), so the path is a complete, self-contained transport address.

#![cfg(unix)]

use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};

use iroh_base::CustomAddr;

use crate::IPC_TRANSPORT_ID;

/// The iroh [`CustomAddr`] for an IPC socket at `path`.
pub fn ipc_custom_addr(path: &Path) -> CustomAddr {
    CustomAddr::from_parts(IPC_TRANSPORT_ID, path.as_os_str().as_bytes())
}

/// The socket path encoded in `addr`, or `None` when `addr` is not an IPC
/// transport address (its [`CustomAddr::id`] is not [`IPC_TRANSPORT_ID`]).
pub fn path_from_custom_addr(addr: &CustomAddr) -> Option<PathBuf> {
    if addr.id() != IPC_TRANSPORT_ID {
        return None;
    }
    Some(PathBuf::from(OsStr::from_bytes(addr.data())))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn path_round_trips_through_custom_addr() {
        let path = Path::new("/tmp/wwkg/z6Mk.host.ipc.sock");
        let addr = ipc_custom_addr(path);
        assert_eq!(addr.id(), IPC_TRANSPORT_ID);
        assert_eq!(path_from_custom_addr(&addr).as_deref(), Some(path));
    }

    #[test]
    fn foreign_transport_id_is_rejected() {
        let foreign = CustomAddr::from_parts(0xdead_beef, b"/whatever");
        assert_eq!(path_from_custom_addr(&foreign), None);
    }
}
