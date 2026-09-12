//! `GRAPH_SPIKE_DEBUG=1`: dump every escape sequence the spike sends (and
//! every probe reply it reads) to `graph-spike-debug.log` in the working
//! directory.
//!
//! Exists because of the step-3 failure: the spike drew nothing in Ghostty
//! and there was no way to see what had been sent. With this log, the next
//! "blank screen" starts from the exact byte stream instead of guesses.
//! Replies stay visible too — Ghostty answers `q=0` graphics commands with
//! `OK` or an error, and those errors are the whole diagnosis.

use std::env;
use std::fmt::Write as _;
use std::fs::OpenOptions;
use std::io::{self, Write};
use std::sync::OnceLock;

const LOG_PATH: &str = "graph-spike-debug.log";

/// Keep at most this many bytes of any single write in the log. Image
/// payload chunks are 4 KB of base64 each and say nothing; command headers
/// fit comfortably.
const MAX_LOGGED: usize = 160;

pub fn enabled() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| env::var_os("GRAPH_SPIKE_DEBUG").is_some_and(|v| v != "0" && !v.is_empty()))
}

/// Append one labelled line, e.g. a probe reply or a frame marker.
pub fn note(label: &str, bytes: &[u8]) {
    if !enabled() {
        return;
    }
    append(&format!("\n-- {label}: {}\n", escape(bytes, bytes.len())));
}

/// Append raw sent bytes, escaped, truncated. No framing: consecutive
/// writes read as the stream they were.
pub fn sent(bytes: &[u8]) {
    if !enabled() {
        return;
    }
    append(&escape(bytes, MAX_LOGGED));
}

fn append(text: &str) {
    // Ignored result, deliberately: this *is* the diagnostic channel. If
    // the log itself can't be written, failing the spike over it would
    // trade the bug being debugged for a logging bug.
    let _ = OpenOptions::new()
        .create(true)
        .append(true)
        .open(LOG_PATH)
        .and_then(|mut f| f.write_all(text.as_bytes()));
}

/// Printable-safe rendering: `ESC` for 0x1b, `<hex>` for other controls.
/// (The ignored `write!` results are `fmt::Write` into a `String`, which
/// cannot fail.)
fn escape(bytes: &[u8], keep: usize) -> String {
    let mut out = String::new();
    for &b in &bytes[..bytes.len().min(keep)] {
        match b {
            0x1b => out.push_str("ESC"),
            0x20..=0x7e => out.push(b as char),
            _ => {
                let _ = write!(out, "<{b:02x}>");
            }
        }
    }
    if bytes.len() > keep {
        let _ = write!(out, "(+{} more bytes)", bytes.len() - keep);
    }
    out
}

/// A writer that forwards to `inner` and mirrors everything into the debug
/// log when `GRAPH_SPIKE_DEBUG=1`. The whole frame stream goes through one
/// of these, so the log is exactly what the terminal received.
pub struct Tee<W: Write> {
    inner: W,
}

impl<W: Write> Tee<W> {
    pub fn new(inner: W) -> Self {
        Tee { inner }
    }
}

impl<W: Write> Write for Tee<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let n = self.inner.write(buf)?;
        sent(&buf[..n]);
        Ok(n)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}
