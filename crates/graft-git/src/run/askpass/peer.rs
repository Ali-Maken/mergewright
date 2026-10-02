//! Who may be answered: only a peer running as the server's own uid.
//!
//! The `0700` directory already keeps other users out; this is the second
//! guard, checked against the kernel's record of the connecting process.

use std::fmt;
use std::io;
use std::os::unix::net::UnixStream;

/// Why a connection was turned away before its prompt was read.
#[derive(Debug)]
pub(super) enum Refusal {
    /// The peer runs as another user.
    OtherUser { peer: u32, owner: u32 },
    /// The peer's uid could not be read; refused rather than assumed ours.
    Unreadable(io::Error),
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Refusal::OtherUser { peer, owner } => write!(
                f,
                "askpass: refused a connection from uid {peer} (this server serves uid {owner})"
            ),
            Refusal::Unreadable(error) => write!(
                f,
                "askpass: refused a connection whose uid could not be read: {error}"
            ),
        }
    }
}

/// Whether a peer with uid `peer` may be served by a server owned by
/// `owner`. Fails closed: an unreadable uid is a refusal.
pub(super) fn admit(peer: io::Result<u32>, owner: u32) -> Result<(), Refusal> {
    match peer {
        Ok(peer) if peer == owner => Ok(()),
        Ok(peer) => Err(Refusal::OtherUser { peer, owner }),
        Err(error) => Err(Refusal::Unreadable(error)),
    }
}

/// The effective uid of the process at the other end of `stream`, from the
/// kernel: `getpeereid` on macOS and the BSDs.
#[cfg(any(
    target_os = "macos",
    target_os = "ios",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "dragonfly"
))]
pub(super) fn peer_uid(stream: &UnixStream) -> io::Result<u32> {
    let (uid, _gid) = nix::unistd::getpeereid(stream)?;
    Ok(uid.as_raw())
}

/// The effective uid of the process at the other end of `stream`, from the
/// kernel: `SO_PEERCRED` on Linux and Android.
#[cfg(any(target_os = "linux", target_os = "android"))]
pub(super) fn peer_uid(stream: &UnixStream) -> io::Result<u32> {
    let creds = nix::sys::socket::getsockopt(stream, nix::sys::socket::sockopt::PeerCredentials)?;
    Ok(creds.uid())
}
