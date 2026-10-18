//! A named pipe a shell script can announce itself through.
//!
//! A test that has to know a hook is *inside* the command it hooks has two
//! choices: touch a file and poll for it, or give the hook something the test
//! can block on. Polling is a timer, and CLAUDE.md rule 2 bans those; this is
//! the event.
//!
//! ```sh
//! #!/bin/sh
//! echo started > "$MARKER"   # the test's `wait` returns here
//! sleep 5
//! ```
//!
//! Unix only: there is no `mkfifo` elsewhere, and the hooks that use it are
//! `/bin/sh` scripts anyway.

use std::fs::{File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use tempfile::TempDir;

/// A FIFO in a temporary directory, with a thread already blocked on reading
/// it. [`wait`](Fifo::wait) is that read; a writer opening the path and
/// writing anything ends it.
///
/// The pipe is held open read-write by this fixture for its whole life, which
/// is what makes it usable from a shell script: the script's
/// `echo … > path` never blocks waiting for a reader to show up, whether the
/// test is already waiting or not. It also means the reading thread can never
/// see end-of-file, so [`Drop`] wakes it by writing a byte itself.
///
/// The directory and the FIFO are removed when this drops.
#[derive(Debug)]
pub struct Fifo {
    path: PathBuf,
    /// Kept for the drop that removes the directory the FIFO lives in.
    _dir: TempDir,
    /// The write end this fixture holds. Two jobs: it keeps the pipe from
    /// ever reaching end-of-file, and `Drop` writes through it to release the
    /// reading thread.
    waker: File,
    written: Receiver<io::Result<()>>,
    reader: Option<JoinHandle<()>>,
}

impl Fifo {
    /// Makes a FIFO and starts reading it.
    ///
    /// # Errors
    /// If the temporary directory, the FIFO or the reading thread cannot be
    /// made.
    pub fn create() -> io::Result<Fifo> {
        let dir = TempDir::new()?;
        let path = dir.path().join("marker");
        make_fifo(&path)?;

        // Read-write, not read-only: opening a FIFO for reading alone blocks
        // until a writer arrives, which would be this call hanging until the
        // hook ran — exactly backwards.
        let read = OpenOptions::new().read(true).write(true).open(&path)?;
        let waker = OpenOptions::new().read(true).write(true).open(&path)?;

        let (sender, written) = mpsc::channel();
        let reader = thread::Builder::new()
            .name("fifo-marker".to_owned())
            .spawn(move || {
                let mut read = read;
                let mut byte = [0u8; 1];
                let result = read.read(&mut byte).map(drop);
                // The receiver is gone only if the fixture was dropped
                // without waiting, which is allowed; there is no one left to
                // tell, and nothing was lost.
                drop(sender.send(result));
            })?;

        Ok(Fifo {
            path,
            _dir: dir,
            waker,
            written,
            reader: Some(reader),
        })
    }

    /// The path to put in the script.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Blocks until something has been written to the FIFO, or `within`
    /// elapses; `true` if something was.
    ///
    /// `within` is a deadline for the writer that never comes, so a broken
    /// test fails instead of hanging the run. It is not a poll interval:
    /// nothing wakes between the write and this returning.
    pub fn wait(&self, within: Duration) -> bool {
        match self.written.recv_timeout(within) {
            Ok(Ok(())) => true,
            Ok(Err(error)) => {
                eprintln!("Fifo at {}: read failed: {error}", self.path.display());
                false
            }
            Err(RecvTimeoutError::Timeout) => false,
            Err(RecvTimeoutError::Disconnected) => {
                // The reading thread ended without sending, which it can only
                // do by panicking. Say so: "nothing was written" would send
                // the caller looking at its script instead of at this.
                eprintln!(
                    "Fifo at {}: the reading thread died without answering",
                    self.path.display()
                );
                false
            }
        }
    }
}

impl Drop for Fifo {
    fn drop(&mut self) {
        let Some(reader) = self.reader.take() else {
            return;
        };
        // The thread is blocked on a read that can never see end-of-file
        // (this fixture holds a write end). One byte releases it. If the hook
        // already wrote, the byte goes into the pipe buffer and is discarded
        // with it.
        if let Err(error) = self.waker.write_all(b"x") {
            let message = format!(
                "Fifo at {}: could not wake the reader ({error}); its thread is still running",
                self.path.display()
            );
            if thread::panicking() {
                eprintln!("{message}");
                return;
            }
            panic!("{message}");
        }
        if reader.join().is_err() {
            eprintln!("Fifo at {}: reading thread panicked", self.path.display());
        }
    }
}

/// `mkfifo -m 600 <path>`, through the POSIX binary.
///
/// `libc::mkfifo` would be one call instead of one process, but it is
/// `unsafe`, and this crate's `#![deny(unsafe_code)]` admits only the sites
/// ADR-0015 names — widening that is a decision, not a test fixture's to
/// take. The standard library has no `mkfifo`, so the binary it is. It runs
/// once, at setup, and its exit status is checked.
fn make_fifo(path: &Path) -> io::Result<()> {
    let made = Command::new("mkfifo")
        .args(["-m", "600"])
        .arg(path)
        .status()?;
    if made.success() {
        Ok(())
    } else {
        Err(io::Error::other(format!(
            "mkfifo {} failed: {made}",
            path.display()
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A writer that arrives after the wait has begun ends it.
    #[test]
    fn a_write_ends_the_wait() {
        let fifo = Fifo::create().expect("the fifo is made");
        let path = fifo.path().to_path_buf();
        let writer = thread::spawn(move || {
            std::fs::write(&path, b"started\n").expect("the fifo takes a write");
        });
        assert!(fifo.wait(Duration::from_secs(30)), "the write arrived");
        writer.join().expect("the writer finished");
    }

    /// A writer that arrived first is not missed: the pipe holds the byte.
    #[test]
    fn a_write_before_the_wait_is_not_missed() {
        let fifo = Fifo::create().expect("the fifo is made");
        std::fs::write(fifo.path(), b"started\n").expect("the fifo takes a write");
        assert!(fifo.wait(Duration::from_secs(30)));
    }

    /// Nothing written, and the wait gives up rather than hanging.
    #[test]
    fn a_wait_with_no_writer_times_out() {
        let fifo = Fifo::create().expect("the fifo is made");
        assert!(!fifo.wait(Duration::from_millis(50)));
    }
}
