//! The bytes that frame one drawn frame, and the fence that paces the next
//! (DESIGN.md §4.7, ADR-0054).
//!
//! A frame is `SYNC_BEGIN` · text · image commands · `SYNC_END` · fence. The
//! bracket makes it atomic: a DEC 2026 terminal shows all of it or none of it.
//! The fence is a device-status request (`CSI 5 n`); the terminal answers
//! `CSI 0 n` once it has read every byte before it, because a terminal
//! answers in order. The loop writes no further frame until that answer, so
//! there is never more than one frame in the pipe.

use std::io::{self, Write};

use crate::detect::reply::{device_status_ok, replies};

/// Begin a synchronized update: the terminal buffers what follows and paints
/// it in one go (DEC 2026, DESIGN.md §4.1). Sent unconditionally: a terminal
/// without the mode ignores it.
pub const SYNC_BEGIN: &[u8] = b"\x1b[?2026h";

/// End a synchronized update, which is what makes the frame appear.
pub const SYNC_END: &[u8] = b"\x1b[?2026l";

/// The fence itself: device status report, "are you OK?".
const FENCE: &[u8] = b"\x1b[5n";

/// Ask the terminal to answer once it has read everything written so far
/// (`CSI 5 n`). Not flushed: the caller flushes the frame it ends.
///
/// # Errors
/// Whatever writing to `out` returns.
pub fn write_fence(out: &mut impl Write) -> io::Result<()> {
    out.write_all(FENCE)
}

/// How many fence answers (`CSI 0 n`, "terminal OK") are in `bytes`. Other
/// replies and keystrokes around them are stepped over; an answer cut off at
/// the end of `bytes` is not counted (the input decoder reassembles a reply
/// split across reads before anyone counts it).
#[must_use]
pub fn fence_answers(bytes: &[u8]) -> usize {
    replies(bytes)
        .filter(|reply| device_status_ok(*reply))
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_fence_is_a_device_status_request() {
        let mut out = Vec::new();
        write_fence(&mut out).expect("a Vec takes every byte");
        assert_eq!(out, b"\x1b[5n");
    }

    #[test]
    fn fence_answers_are_counted_amid_other_replies_and_keys() {
        let bytes =
            b"\x1b[0nj\x1b_Gi=7;ENOENT:gone\x1b\\\x1b[?62;22c\x1b[0nk\x1b[3n\x1b[6;16;8t\x1b[0n";
        assert_eq!(fence_answers(bytes), 3);
    }

    #[test]
    fn nothing_but_an_ok_status_is_an_answer() {
        // `CSI 3 n` is "malfunction"; `CSI ? 0 n` and `CSI 0 ; 1 n` are not
        // this answer either; `CSI 0 m` is a different final byte.
        assert_eq!(fence_answers(b"\x1b[3n\x1b[?0n\x1b[0;1n\x1b[0m"), 0);
        assert_eq!(fence_answers(b""), 0);
    }

    #[test]
    fn an_answer_cut_off_at_the_end_is_not_counted_until_it_is_whole() {
        let whole = b"\x1b[0n\x1b[0n";
        let (first, second) = whole.split_at(6);
        assert_eq!(fence_answers(first), 1, "the split one is not counted yet");
        let mut joined = first.to_vec();
        joined.extend_from_slice(second);
        assert_eq!(fence_answers(&joined), 2);
    }
}
