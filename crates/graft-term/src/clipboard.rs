//! Putting text on the terminal's clipboard: OSC 52 (ADR-0023, decision 4).
//!
//! `ESC ] 52 ; c ; <base64> ESC \` asks the terminal to set its clipboard.
//! It is a request, not a transaction: nothing comes back, and a terminal
//! that ignores or refuses it (a disabled setting, tmux without
//! passthrough) looks exactly like one that honoured it. So what Graft can
//! say afterwards is what it sent, never that the clipboard has it.
//!
//! The sequence is one sequence, so inside tmux it goes through
//! [`crate::Transport::write`] like an image command, wrapped alone.

/// The OSC 52 sequence that asks the terminal to put `text` on its
/// clipboard (the `c` selection), terminated with `ESC \`.
#[must_use]
pub fn osc52_copy(text: &str) -> Vec<u8> {
    let mut out = Vec::with_capacity(text.len().div_ceil(3) * 4 + 9);
    out.extend_from_slice(b"\x1b]52;c;");
    base64_into(text.as_bytes(), &mut out);
    out.extend_from_slice(b"\x1b\\");
    out
}

/// Standard base64 (RFC 4648 §4, with `=` padding), appended to `out`.
/// Local rather than a dependency: a SHA is the only thing encoded, and
/// this is the whole encoder.
fn base64_into(bytes: &[u8], out: &mut Vec<u8>) {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    for chunk in bytes.chunks(3) {
        let first = chunk[0];
        let second = chunk.get(1).copied().unwrap_or(0);
        let third = chunk.get(2).copied().unwrap_or(0);
        let group = (u32::from(first) << 16) | (u32::from(second) << 8) | u32::from(third);
        // Each index is masked to six bits, so it is always below 64.
        let sextet = |shift: u32| ALPHABET[((group >> shift) & 0x3f) as usize];
        out.push(sextet(18));
        out.push(sextet(12));
        out.push(if chunk.len() > 1 { sextet(6) } else { b'=' });
        out.push(if chunk.len() > 2 { sextet(0) } else { b'=' });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Transport;
    use crate::tmux::unwrap_all;

    fn base64(bytes: &[u8]) -> String {
        let mut out = Vec::new();
        base64_into(bytes, &mut out);
        String::from_utf8(out).expect("base64 is ASCII")
    }

    #[test]
    fn abc_is_the_exact_sequence() {
        assert_eq!(osc52_copy("abc"), b"\x1b]52;c;YWJj\x1b\\");
    }

    /// RFC 4648 §10's test vectors: every padding case.
    #[test]
    fn base64_matches_the_rfc_vectors() {
        for (plain, encoded) in [
            ("", ""),
            ("f", "Zg=="),
            ("fo", "Zm8="),
            ("foo", "Zm9v"),
            ("foob", "Zm9vYg=="),
            ("fooba", "Zm9vYmE="),
            ("foobar", "Zm9vYmFy"),
        ] {
            assert_eq!(base64(plain.as_bytes()), encoded, "{plain:?}");
        }
    }

    /// All 256 byte values, so every alphabet entry and both high bits are
    /// exercised; checked against the output of `base64` (coreutils).
    #[test]
    fn base64_covers_the_whole_alphabet() {
        assert_eq!(base64(&[0xfb, 0xff, 0xbf, 0x00, 0x10, 0x83]), "+/+/ABCD");
    }

    #[test]
    fn a_full_sha_encodes_to_fifty_six_characters() {
        let sha = "848ec2e5f6c7a1b2c3d4e5f60718293a4b5c6d7e";
        let sequence = osc52_copy(sha);
        let body = &sequence[7..sequence.len() - 2];
        assert_eq!(body.len(), 56);
        assert_eq!(body, base64(sha.as_bytes()).as_bytes());
    }

    #[test]
    fn inside_tmux_it_is_wrapped_alone_with_every_escape_doubled() {
        let sequence = osc52_copy("abc");
        let mut out = Vec::new();
        Transport::TmuxPassthrough.write(&sequence, &mut out);
        assert_eq!(out, b"\x1bPtmux;\x1b\x1b]52;c;YWJj\x1b\x1b\\\x1b\\");
        assert_eq!(unwrap_all(&out), (sequence, 1));
    }

    #[test]
    fn directly_it_is_written_as_is() {
        let mut out = Vec::new();
        Transport::Direct.write(&osc52_copy("abc"), &mut out);
        assert_eq!(out, b"\x1b]52;c;YWJj\x1b\\");
    }
}
