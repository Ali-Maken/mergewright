//! Sends a step's bytes, fences the terminal's replies behind a DA1 query,
//! and decodes a capture into a human-readable summary. See `driver` for the
//! commands themselves.

use std::fmt::Write as _;
use std::io::{self, Read, Write};
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

use graft_term::kitty;

/// How long a step may wait for the DA1 answer before counting as silent.
const FENCE_DEADLINE: Duration = Duration::from_secs(3);

/// One step's name, the bytes it sent, and what the terminal answered.
pub(crate) struct Step {
    pub(crate) name: &'static str,
    pub(crate) transmits: usize,
    pub(crate) placements: usize,
    pub(crate) replies: Vec<kitty::GraphicsReply>,
    pub(crate) fenced: bool,
}

/// Send `out`, fence it with DA1, and collect the image replies before the
/// DA1 answer.
pub(crate) fn run_step(
    name: &'static str,
    out: &[u8],
    stdout: &mut impl Write,
    input: &Receiver<Vec<u8>>,
    capture: &mut Vec<u8>,
) -> io::Result<Step> {
    let text = String::from_utf8_lossy(out);
    let transmits = text.matches("a=t,").count();
    let placements = text.matches("a=p,").count();
    stdout.write_all(out)?;
    stdout.write_all(b"\x1b[c")?;
    stdout.flush()?;
    capture.extend_from_slice(out);
    capture.extend_from_slice(b"\x1b[c");

    let deadline = Instant::now() + FENCE_DEADLINE;
    let mut heard = Vec::new();
    let fenced = loop {
        if da1_answer(&heard) {
            break true;
        }
        let Some(left) = deadline.checked_duration_since(Instant::now()) else {
            break false;
        };
        match input.recv_timeout(left) {
            Ok(bytes) => heard.extend_from_slice(&bytes),
            Err(mpsc::RecvTimeoutError::Timeout) => break false,
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "stdin closed before the DA1 answer",
                ));
            }
        }
    };
    Ok(Step {
        name,
        transmits,
        placements,
        replies: kitty::replies(&heard),
        fenced,
    })
}

/// Whether `bytes` hold a DA1 answer, `CSI ? … c`.
fn da1_answer(bytes: &[u8]) -> bool {
    bytes.windows(3).enumerate().any(|(start, window)| {
        window == b"\x1b[?"
            && bytes[start + 3..]
                .iter()
                .find(|byte| !(byte.is_ascii_digit() || **byte == b';'))
                == Some(&b'c')
    })
}

/// Read stdin on a thread, handing each chunk over as it arrives.
pub(crate) fn spawn_stdin() -> Receiver<Vec<u8>> {
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let mut stdin = io::stdin();
        let mut chunk = [0u8; 4096];
        loop {
            match stdin.read(&mut chunk) {
                Ok(count) if count > 0 => {
                    if sender.send(chunk[..count].to_vec()).is_err() {
                        // The main thread is gone: nobody left to hand to.
                        return;
                    }
                }
                // End of input or a read error: dropping the sender is what
                // reports it, as the waiting step's `Disconnected` error.
                Ok(_) | Err(_) => return,
            }
        }
    });
    receiver
}

/// The capture made readable: one line per escape sequence, image payloads
/// replaced by their length.
pub(crate) fn decode(bytes: &[u8]) -> String {
    let mut text = String::new();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index..].starts_with(b"\x1b_G") {
            let end = find(bytes, index, b"\x1b\\").unwrap_or(bytes.len());
            let body = String::from_utf8_lossy(&bytes[index + 3..end]);
            let (control, payload) = body.split_once(';').unwrap_or((&body, ""));
            writeln!(text, "APC G {control}  [payload {} bytes]", payload.len()).ok();
            index = (end + 2).min(bytes.len());
        } else if bytes[index] == 0x1b {
            let end = sequence_end(bytes, index);
            let shown: String = bytes[index + 1..end]
                .iter()
                .map(|&byte| char::from(byte))
                .collect();
            writeln!(text, "ESC {shown}").ok();
            index = end;
        } else {
            let end = find(bytes, index, b"\x1b").unwrap_or(bytes.len());
            writeln!(
                text,
                "text {:?}",
                String::from_utf8_lossy(&bytes[index..end])
            )
            .ok();
            index = end;
        }
    }
    text
}

fn find(bytes: &[u8], from: usize, needle: &[u8]) -> Option<usize> {
    bytes[from..]
        .windows(needle.len())
        .position(|window| window == needle)
        .map(|at| from + at)
}

/// One past the end of the non-APC escape at `start`: CSI up to its final
/// byte, anything else two bytes.
fn sequence_end(bytes: &[u8], start: usize) -> usize {
    if bytes.get(start + 1) == Some(&b'[') {
        bytes[start + 2..]
            .iter()
            .position(|byte| (0x40..=0x7e).contains(byte))
            .map_or(bytes.len(), |at| start + 3 + at)
    } else {
        (start + 2).min(bytes.len())
    }
}
