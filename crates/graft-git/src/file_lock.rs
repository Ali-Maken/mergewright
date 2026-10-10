//! An advisory lock on a file: many readers share it, one writer holds it
//! alone (`flock(2)`).
//!
//! Advisory: it stops only processes that ask for it too. The kernel drops
//! it when the holder's file is closed — on drop, and when the process dies,
//! however it dies — so a crash never leaves a stale lock behind, which a
//! lock *file* (create-exclusive) would.
//!
//! Each [`FileLock`] opens the file afresh, so two locks in one process
//! contend exactly as two processes would (`flock` locks belong to the open
//! file, not the process). Nothing here spawns `git`.

use std::fs::{File, OpenOptions};
use std::io;
use std::path::Path;

use nix::errno::Errno;
use nix::fcntl::{Flock, FlockArg};

/// A held lock. Dropping it releases it.
#[derive(Debug)]
pub struct FileLock {
    _held: Flock<File>,
}

impl FileLock {
    /// The lock alone, without waiting: `Ok(None)` when anyone else holds
    /// it, shared or alone. Creates the file (not its folder) if missing.
    ///
    /// # Errors
    /// The file could not be opened or created, or `flock` failed for a
    /// reason other than someone else holding it.
    pub fn try_exclusive(path: &Path) -> io::Result<Option<FileLock>> {
        match Flock::lock(open(path)?, FlockArg::LockExclusiveNonblock) {
            Ok(held) => Ok(Some(FileLock { _held: held })),
            Err((_, Errno::EWOULDBLOCK)) => Ok(None),
            Err((_, errno)) => Err(io::Error::from(errno)),
        }
    }

    /// The lock shared with other readers, waiting while someone holds it
    /// alone — a kernel wait, not a poll. Creates the file if missing.
    ///
    /// # Errors
    /// As [`try_exclusive`](Self::try_exclusive).
    pub fn shared(path: &Path) -> io::Result<FileLock> {
        match Flock::lock(open(path)?, FlockArg::LockShared) {
            Ok(held) => Ok(FileLock { _held: held }),
            Err((_, errno)) => Err(io::Error::from(errno)),
        }
    }
}

fn open(path: &Path) -> io::Result<File> {
    OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)
}

#[cfg(test)]
mod tests {
    use super::FileLock;

    #[test]
    fn alone_excludes_everyone_and_shared_excludes_alone() {
        let dir = tempfile::tempdir().expect("a scratch dir");
        let path = dir.path().join("lock");
        let free = |path| FileLock::try_exclusive(path).expect("flock answers");

        let alone = free(&path).expect("nobody holds it yet");
        assert!(free(&path).is_none(), "one holder alone at a time");
        drop(alone);

        let reader = FileLock::shared(&path).expect("free to share");
        let second_reader = FileLock::shared(&path).expect("readers share");
        assert!(free(&path).is_none(), "readers keep a writer out");
        drop(reader);
        assert!(free(&path).is_none(), "one reader is enough");
        drop(second_reader);
        assert!(free(&path).is_some(), "dropped is released");
    }
}
