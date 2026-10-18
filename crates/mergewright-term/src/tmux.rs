//! Getting escape sequences through tmux to the terminal behind it
//! (DESIGN.md §4.3).
//!
//! tmux draws its panes itself and swallows any sequence it does not
//! understand, image protocols included. The one way past it is the DCS
//! passthrough: `ESC P tmux; <payload> ESC \`, with every `ESC` inside the
//! payload doubled so tmux can tell the payload's own terminators from the
//! wrapper's. tmux ≥ 3.3 forwards the payload, unescaped, when
//! `allow-passthrough` is on; otherwise it drops the whole thing, which is
//! why detection re-probes through this same wrapper before any image is
//! sent (ADR-0014 §4).
//!
//! Wrap **one sequence at a time**, never a whole frame. Cursor moves and
//! save/restore must reach tmux itself, which owns the cursor; and a single
//! passthrough holding a frame's worth of chunked uploads would be one
//! unbounded DCS string instead of many small ones.

use mergewright_core::capabilities::TmuxStatus;

/// How an image sequence reaches the terminal that draws it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Transport {
    /// Written as is: Mergewright is talking to the terminal directly.
    #[default]
    Direct,
    /// Each sequence wrapped in tmux's passthrough, one wrapper apiece.
    TmuxPassthrough,
}

impl Transport {
    /// The transport detection's tmux answer calls for. Anything but
    /// [`TmuxStatus::Outside`] wraps: when detection settled on pixels inside
    /// tmux the passthrough answered, and when `graph.renderer = "pixel"`
    /// forces them anyway, an unwrapped sequence is one tmux certainly eats.
    #[must_use]
    pub fn for_tmux(status: &TmuxStatus) -> Self {
        match status {
            TmuxStatus::Outside => Self::Direct,
            _ => Self::TmuxPassthrough,
        }
    }

    /// Append one complete escape sequence to `out`, wrapped if need be.
    pub fn write(self, sequence: &[u8], out: &mut Vec<u8>) {
        match self {
            Self::Direct => out.extend_from_slice(sequence),
            Self::TmuxPassthrough => tmux_passthrough(sequence, out),
        }
    }
}

/// Append `sequence` to `out` wrapped in tmux's passthrough: `ESC P tmux;`,
/// the sequence with every `ESC` doubled, then `ESC \`.
pub fn tmux_passthrough(sequence: &[u8], out: &mut Vec<u8>) {
    out.reserve(sequence.len() + 9);
    out.extend_from_slice(b"\x1bPtmux;");
    for &byte in sequence {
        if byte == 0x1b {
            out.push(0x1b);
        }
        out.push(byte);
    }
    out.extend_from_slice(b"\x1b\\");
}

/// Undo every passthrough wrapper in `bytes` the way tmux does, leaving the
/// bytes outside wrappers alone. Returns the stream the outer terminal would
/// receive, and how many wrappers there were. Panics on a malformed wrapper.
#[cfg(test)]
pub(crate) fn unwrap_all(bytes: &[u8]) -> (Vec<u8>, usize) {
    const OPEN: &[u8] = b"\x1bPtmux;";
    let mut out = Vec::new();
    let mut wrappers = 0;
    let mut index = 0;
    while index < bytes.len() {
        if !bytes[index..].starts_with(OPEN) {
            out.push(bytes[index]);
            index += 1;
            continue;
        }
        wrappers += 1;
        index += OPEN.len();
        loop {
            match (bytes.get(index), bytes.get(index + 1)) {
                (Some(0x1b), Some(0x1b)) => {
                    out.push(0x1b);
                    index += 2;
                }
                (Some(0x1b), Some(b'\\')) => {
                    index += 2;
                    break;
                }
                (Some(0x1b), _) => panic!("a lone ESC inside a wrapper at {index}"),
                (Some(&byte), _) => {
                    out.push(byte);
                    index += 1;
                }
                (None, _) => panic!("an unterminated wrapper"),
            }
        }
    }
    (out, wrappers)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wrapped(sequence: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        tmux_passthrough(sequence, &mut out);
        out
    }

    /// tmux eats one level of escaping, so what it forwards is the original
    /// sequence byte for byte.
    #[test]
    fn every_escape_inside_is_doubled_and_the_wrapper_is_not() {
        assert_eq!(
            wrapped(b"\x1b_Ga=d,i=7,q=1\x1b\\"),
            b"\x1bPtmux;\x1b\x1b_Ga=d,i=7,q=1\x1b\x1b\\\x1b\\"
        );
    }

    #[test]
    fn a_sequence_with_no_escape_is_only_wrapped() {
        assert_eq!(wrapped(b"abc"), b"\x1bPtmux;abc\x1b\\");
    }

    #[test]
    fn it_appends_rather_than_replacing() {
        let mut out = b"before".to_vec();
        tmux_passthrough(b"\x1b[c", &mut out);
        assert_eq!(out, b"before\x1bPtmux;\x1b\x1b[c\x1b\\");
    }

    #[test]
    fn only_a_session_outside_tmux_writes_directly() {
        assert_eq!(Transport::for_tmux(&TmuxStatus::Outside), Transport::Direct);
        for status in [
            TmuxStatus::Passthrough {
                version: "3.4".to_owned(),
            },
            TmuxStatus::PassthroughSilent {
                version: "3.4".to_owned(),
            },
            TmuxStatus::TooOld {
                version: "3.2".to_owned(),
            },
            TmuxStatus::VersionUnknown,
        ] {
            assert_eq!(
                Transport::for_tmux(&status),
                Transport::TmuxPassthrough,
                "{status:?}"
            );
        }
    }

    #[test]
    fn a_direct_transport_leaves_the_sequence_alone() {
        let mut out = Vec::new();
        Transport::Direct.write(b"\x1b[c", &mut out);
        assert_eq!(out, b"\x1b[c");
    }

    /// Undoing the wrapper the way tmux does gives back the input exactly,
    /// whatever bytes it held.
    #[test]
    fn unwrapping_returns_the_original() {
        let original: Vec<u8> = (0u8..=255).chain([0x1b, 0x1b, b'\\']).collect();
        assert_eq!(unwrap_all(&wrapped(&original)), (original, 1));
    }
}
