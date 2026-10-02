//! Splitting whatever the terminal sent back into the replies Graft asked for.
//!
//! A probe round writes six queries and then reads one buffer. What comes back
//! is not six tidy answers: it is the answers a terminal happened to implement,
//! in whatever order, possibly interleaved with keystrokes the user typed
//! during the probe window, and possibly cut off when the deadline hit
//! mid-sequence. So this module scans rather than parses: it walks the buffer
//! for well-formed escape sequences, hands each one to the caller, and steps
//! over everything it does not recognise.
//!
//! Two rules make that safe. A sequence that is merely *unfinished* at the end
//! of the buffer ends the scan (there is nothing after it to misread), and a
//! sequence that is *malformed* — one that a new `ESC` interrupts — is
//! abandoned at the interruption, so the interrupting sequence is still found.
//! Both cases are real: a terminal that answers slowly gets truncated by the
//! deadline, and a stray `ESC` keypress lands in the middle of a reply.

use graft_core::capabilities::Rgb;

const ESCAPE: u8 = 0x1b;
const BELL: u8 = 0x07;

/// One well-formed escape sequence from the terminal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Reply<'a> {
    /// `ESC [ <params> <intermediates> <final>`. `params` keeps its leading
    /// `?` when there is one: `CSI ? … c` (a `DA1` answer) and `CSI … t` (a
    /// window report) are told apart by exactly that.
    Csi {
        params: &'a [u8],
        intermediates: &'a [u8],
        final_byte: u8,
    },
    /// `ESC ] <body> <terminator>`, the body without its terminator.
    Osc(&'a [u8]),
    /// `ESC _ <body> ESC \`, the body without its terminator. The Kitty
    /// graphics protocol answers here.
    Apc(&'a [u8]),
    /// `ESC P <body> ESC \`, the body without its terminator.
    Dcs(&'a [u8]),
}

/// Every reply in a buffer, in the order the terminal sent them.
pub(crate) fn replies(bytes: &[u8]) -> Replies<'_> {
    Replies { bytes, position: 0 }
}

pub(crate) struct Replies<'a> {
    bytes: &'a [u8],
    position: usize,
}

/// What one attempt at reading a sequence found.
enum Scan<'a> {
    /// A complete sequence, and where the next one could start.
    Found(Reply<'a>, usize),
    /// Not a sequence after all. Resume the scan here — always strictly after
    /// the `ESC` that started the attempt, so the walk cannot stall.
    Malformed(usize),
    /// The buffer ended mid-sequence. Nothing further can be read.
    Truncated,
}

impl<'a> Iterator for Replies<'a> {
    type Item = Reply<'a>;

    fn next(&mut self) -> Option<Reply<'a>> {
        loop {
            let offset = self.bytes[self.position..]
                .iter()
                .position(|&byte| byte == ESCAPE)?;
            let escape = self.position + offset;
            // `ESC` as the last byte: a sequence the deadline cut off.
            let introducer = *self.bytes.get(escape + 1)?;
            let body = escape + 2;

            let scan = match introducer {
                b'[' => csi(self.bytes, body),
                b']' => string_sequence(self.bytes, body, Terminator::BellOrSt, Reply::Osc),
                b'_' => string_sequence(self.bytes, body, Terminator::St, Reply::Apc),
                b'P' => string_sequence(self.bytes, body, Terminator::St, Reply::Dcs),
                // A two-byte escape, or a keystroke: skip past it and keep
                // looking. Not an error — the buffer legitimately carries
                // things Graft never asked for.
                _ => Scan::Malformed(body),
            };

            match scan {
                Scan::Found(reply, next) => {
                    self.position = next;
                    return Some(reply);
                }
                Scan::Malformed(resume) => self.position = resume,
                Scan::Truncated => {
                    self.position = self.bytes.len();
                    return None;
                }
            }
        }
    }
}

/// `ESC [` has been seen; read parameters, intermediates and the final byte.
fn csi(bytes: &[u8], start: usize) -> Scan<'_> {
    let mut end = start;
    while matches!(bytes.get(end), Some(0x30..=0x3f)) {
        end += 1;
    }
    let params = &bytes[start..end];

    let intermediates_start = end;
    while matches!(bytes.get(end), Some(0x20..=0x2f)) {
        end += 1;
    }
    let intermediates = &bytes[intermediates_start..end];

    match bytes.get(end) {
        None => Scan::Truncated,
        Some(&final_byte @ 0x40..=0x7e) => Scan::Found(
            Reply::Csi {
                params,
                intermediates,
                final_byte,
            },
            end + 1,
        ),
        // A control byte where the final byte belongs: this was never a CSI.
        Some(_) => Scan::Malformed(end),
    }
}

/// Which terminators end a string sequence. OSC accepts the older `BEL` as
/// well as `ST`; APC and DCS take `ST` only.
#[derive(Clone, Copy)]
enum Terminator {
    St,
    BellOrSt,
}

/// `ESC ]`, `ESC _` or `ESC P` has been seen; read the body up to its
/// terminator.
fn string_sequence<'a>(
    bytes: &'a [u8],
    start: usize,
    terminator: Terminator,
    wrap: fn(&'a [u8]) -> Reply<'a>,
) -> Scan<'a> {
    let mut end = start;
    while let Some(&byte) = bytes.get(end) {
        match byte {
            BELL if matches!(terminator, Terminator::BellOrSt) => {
                return Scan::Found(wrap(&bytes[start..end]), end + 1);
            }
            ESCAPE => {
                return match bytes.get(end + 1) {
                    None => Scan::Truncated,
                    Some(b'\\') => Scan::Found(wrap(&bytes[start..end]), end + 2),
                    // Another sequence started before this one closed. Give up
                    // on the body and rescan from the interruption.
                    Some(_) => Scan::Malformed(end),
                };
            }
            _ => end += 1,
        }
    }
    Scan::Truncated
}

/// The parameters of a primary device attributes reply (`CSI ? … c`), or
/// `None` for any other sequence. Empty and non-numeric fields are dropped:
/// only the presence of a specific number ever matters here.
pub(crate) fn device_attributes(reply: Reply<'_>) -> Option<Vec<u16>> {
    let Reply::Csi {
        params, final_byte, ..
    } = reply
    else {
        return None;
    };
    if final_byte != b'c' {
        return None;
    }
    let body = params.strip_prefix(b"?")?;
    Some(
        body.split(|&byte| byte == b';')
            .filter_map(number::<u16>)
            .collect(),
    )
}

/// Cell size in pixels from a `CSI 6 ; height ; width t` reply — height first,
/// which is the field order the spike got wrong once and the ADR records
/// (ADR-0012). Returns `(width, height)`; a zero in either field is no answer
/// at all, not a cell of size zero.
pub(crate) fn cell_size(reply: Reply<'_>) -> Option<(u32, u32)> {
    let Reply::Csi {
        params, final_byte, ..
    } = reply
    else {
        return None;
    };
    if final_byte != b't' {
        return None;
    }
    let mut fields = params.split(|&byte| byte == b';');
    if number::<u16>(fields.next()?) != Some(6) {
        return None;
    }
    let height = number::<u32>(fields.next()?)?;
    let width = number::<u32>(fields.next()?)?;
    if height == 0 || width == 0 {
        return None;
    }
    Some((width, height))
}

/// True for a Kitty keyboard protocol reply (`CSI ? <flags> u`). The flags
/// themselves say which enhancements are on *right now*, which is not what is
/// being asked: any answer at all means the protocol exists.
pub(crate) fn kitty_keyboard(reply: Reply<'_>) -> bool {
    matches!(
        reply,
        Reply::Csi {
            params,
            final_byte: b'u',
            ..
        } if params.starts_with(b"?")
    )
}

/// True for the "terminal OK" device status report, `CSI 0 n` — the answer
/// to `CSI 5 n`, and to the per-frame fence (ADR-0054).
pub(crate) fn device_status_ok(reply: Reply<'_>) -> bool {
    matches!(
        reply,
        Reply::Csi {
            params: b"0",
            intermediates: b"",
            final_byte: b'n',
        }
    )
}

/// The reported state of DEC mode 2026 (synchronized output) from a `DECRPM`
/// reply, `CSI ? 2026 ; <state> $ y`.
///
/// `0` means the terminal does not know the mode; `1`–`4` are set, reset,
/// permanently set and permanently reset — all four of which mean it does.
pub(crate) fn synchronized_output(reply: Reply<'_>) -> Option<bool> {
    let Reply::Csi {
        params,
        intermediates,
        final_byte,
    } = reply
    else {
        return None;
    };
    if final_byte != b'y' || intermediates != b"$" {
        return None;
    }
    let mut fields = params.strip_prefix(b"?")?.split(|&byte| byte == b';');
    if number::<u16>(fields.next()?) != Some(2026) {
        return None;
    }
    Some(number::<u16>(fields.next()?)? != 0)
}

/// A Kitty graphics protocol reply (`ESC _ G i=<id>… ; message ESC \`) about
/// image `id`: `Some(true)` for `OK`, `Some(false)` for any error, `None` for
/// anything else, a reply about another id included.
///
/// For the graphics query any answer counts, exactly as ADR-0012 found: only
/// a terminal that implements the protocol answers `a=q` at all. For the
/// deflate query only `OK` does. Matching by id keeps the two apart, since
/// both go out in the same round.
pub(crate) fn kitty_graphics_reply(reply: Reply<'_>, id: u32) -> Option<bool> {
    let Reply::Apc(body) = reply else {
        return None;
    };
    let body = body.strip_prefix(b"G")?;
    let (control, message) = match body.iter().position(|&byte| byte == b';') {
        Some(at) => (&body[..at], &body[at + 1..]),
        None => (body, &b""[..]),
    };
    let named = control
        .split(|&byte| byte == b',')
        .find_map(|key| number::<u32>(key.strip_prefix(b"i=")?))?;
    (named == id).then_some(message == b"OK")
}

/// The background colour from an `OSC 11` reply.
///
/// Bodies look like `11;rgb:1c1c/1c1c/1c1c` — one to four hex digits per
/// channel, scaled to eight bits — and, from a few terminals, `11;#1c1c1c`.
pub(crate) fn background_color(reply: Reply<'_>) -> Option<Rgb> {
    let Reply::Osc(body) = reply else {
        return None;
    };
    let spec = body.strip_prefix(b"11;")?;
    if let Some(hex) = spec.strip_prefix(b"#") {
        return hash_color(hex);
    }
    let channels = spec
        .strip_prefix(b"rgb:")
        .or_else(|| spec.strip_prefix(b"rgba:"))?;

    let mut fields = channels.split(|&byte| byte == b'/');
    let red = channel(fields.next()?)?;
    let green = channel(fields.next()?)?;
    let blue = channel(fields.next()?)?;
    Some(Rgb { red, green, blue })
}

/// One `rgb:` channel: 1–4 hex digits, scaled so that the widest value of any
/// width becomes 255 (`f` and `ffff` are both full brightness).
fn channel(field: &[u8]) -> Option<u8> {
    // A `u32` from the start: it is a bit count below, never a length.
    let digits = u32::try_from(field.len()).ok()?;
    if digits == 0 || digits > 4 {
        return None;
    }
    let value = hex(field)?;
    let full = (1u32 << (4 * digits)) - 1;
    // Rounds to nearest rather than truncating: `8/f` is 136, not 128.
    u8::try_from((value * 255 + full / 2) / full).ok()
}

/// `#RGB`, `#RRGGBB`, `#RRRGGGBBB` or `#RRRRGGGGBBBB`: three equal-width
/// channels with no separators.
fn hash_color(hex_digits: &[u8]) -> Option<Rgb> {
    let width = hex_digits.len() / 3;
    if width == 0 || width > 4 || !hex_digits.len().is_multiple_of(3) {
        return None;
    }
    Some(Rgb {
        red: channel(&hex_digits[..width])?,
        green: channel(&hex_digits[width..2 * width])?,
        blue: channel(&hex_digits[2 * width..])?,
    })
}

fn hex(digits: &[u8]) -> Option<u32> {
    digits.iter().try_fold(0u32, |value, &digit| {
        Some(value * 16 + char::from(digit).to_digit(16)?)
    })
}

/// A decimal parameter field. `None` when it is empty or not a number, which
/// is how a malformed reply stops being read rather than becoming a wrong
/// number.
fn number<T: std::str::FromStr>(field: &[u8]) -> Option<T> {
    std::str::from_utf8(field).ok()?.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Captured from a real Ghostty 1.3.1 answering the probe round
    /// (ADR-0012's verification run), in the order the bytes arrived.
    const GHOSTTY_ROUND: &[u8] = b"\x1b_Gi=31;OK\x1b\\\x1b[?0u\x1b[6;37;17t\x1b[?2026;2$y\x1b]11;rgb:1d1d/1f1f/2121\x1b\\\x1b[?62;22c";

    fn all(bytes: &[u8]) -> Vec<Reply<'_>> {
        replies(bytes).collect()
    }

    #[test]
    fn a_full_probe_round_yields_every_reply_in_order() {
        let found = all(GHOSTTY_ROUND);

        assert_eq!(found.len(), 6, "{found:?}");
        assert_eq!(kitty_graphics_reply(found[0], 31), Some(true));
        assert_eq!(kitty_graphics_reply(found[0], 32), None);
        assert!(kitty_keyboard(found[1]));
        assert_eq!(cell_size(found[2]), Some((17, 37)));
        assert_eq!(synchronized_output(found[3]), Some(true));
        assert_eq!(
            background_color(found[4]),
            Some(Rgb {
                red: 0x1d,
                green: 0x1f,
                blue: 0x21
            })
        );
        assert_eq!(device_attributes(found[5]), Some(vec![62, 22]));
    }

    #[test]
    fn sixel_shows_up_as_device_attributes_parameter_four() {
        let found = all(b"\x1b[?63;1;2;4;6;9;22c");
        assert_eq!(
            device_attributes(found[0]),
            Some(vec![63, 1, 2, 4, 6, 9, 22])
        );
    }

    #[test]
    fn an_osc_reply_may_end_with_a_bell_instead_of_st() {
        let found = all(b"\x1b]11;rgb:ffff/ffff/ffff\x07");
        assert_eq!(
            background_color(found[0]),
            Some(Rgb {
                red: 255,
                green: 255,
                blue: 255
            })
        );
    }

    #[test]
    fn short_and_long_colour_channels_scale_to_the_same_eight_bit_value() {
        let short = all(b"\x1b]11;rgb:f/0/8\x1b\\");
        assert_eq!(
            background_color(short[0]),
            Some(Rgb {
                red: 255,
                green: 0,
                blue: 136
            })
        );

        let hash = all(b"\x1b]11;#1d1f21\x1b\\");
        assert_eq!(
            background_color(hash[0]),
            Some(Rgb {
                red: 0x1d,
                green: 0x1f,
                blue: 0x21
            })
        );
    }

    #[test]
    fn a_terminal_without_the_mode_reports_synchronized_output_as_zero() {
        let found = all(b"\x1b[?2026;0$y");
        assert_eq!(synchronized_output(found[0]), Some(false));
    }

    #[test]
    fn a_decrpm_reply_about_another_mode_is_not_read_as_2026() {
        let found = all(b"\x1b[?1049;1$y");
        assert_eq!(synchronized_output(found[0]), None);
    }

    /// The one pair that a sloppy scanner confuses: both are `CSI ?` with a
    /// numeric parameter, and only the final byte tells them apart.
    #[test]
    fn a_device_attributes_reply_is_not_a_keyboard_reply() {
        let found = all(b"\x1b[?62;22c");
        assert!(!kitty_keyboard(found[0]));
        assert!(device_attributes(found[0]).is_some());

        let found = all(b"\x1b[?5u");
        assert!(kitty_keyboard(found[0]));
        assert!(device_attributes(found[0]).is_none());
    }

    /// A window report that is not the cell-size one. `CSI 8 ; rows ; cols t`
    /// is the text-area size, and reading it as pixels would silently halve
    /// every cell.
    #[test]
    fn only_the_cell_size_window_report_is_read_as_a_cell_size() {
        let found = all(b"\x1b[8;40;122t");
        assert_eq!(cell_size(found[0]), None);
    }

    #[test]
    fn a_zero_cell_size_is_no_answer_rather_than_a_zero_sized_cell() {
        let found = all(b"\x1b[6;0;0t");
        assert_eq!(cell_size(found[0]), None);
    }

    #[test]
    fn a_truncated_reply_at_the_end_of_the_buffer_is_dropped() {
        // Each of these is the tail of a sequence the deadline cut short.
        for truncated in [
            &b"\x1b"[..],
            b"\x1b[",
            b"\x1b[6;37;",
            b"\x1b_Gi=31;OK",
            b"\x1b_Gi=31;OK\x1b",
            b"\x1b]11;rgb:1d1d/1f1f/2121",
        ] {
            assert!(all(truncated).is_empty(), "{truncated:?}");
        }
    }

    /// The reply before a truncated one still counts: a cut-off tail must not
    /// throw away what already arrived.
    #[test]
    fn replies_before_a_truncated_one_survive() {
        let found = all(b"\x1b[?62;22c\x1b[6;37;");
        assert_eq!(found.len(), 1);
        assert_eq!(device_attributes(found[0]), Some(vec![62, 22]));
    }

    /// A lone `Esc` keypress lands inside the reply that was arriving. The
    /// half-finished reply is lost — but the scan must recover, not stop.
    #[test]
    fn a_sequence_interrupted_by_another_one_is_abandoned_not_fatal() {
        let found = all(b"\x1b]11;rgb:1d1d\x1b[?62;22c");
        assert_eq!(found.len(), 1);
        assert_eq!(device_attributes(found[0]), Some(vec![62, 22]));

        let found = all(b"\x1b[6;\x1b[?62;22c");
        assert_eq!(found.len(), 1);
        assert_eq!(device_attributes(found[0]), Some(vec![62, 22]));
    }

    /// Typeahead: keys pressed during the probe window arrive mixed into the
    /// replies. They are not replies and must not become one.
    #[test]
    fn keystrokes_between_replies_are_stepped_over() {
        let found = all(b"jjk\x1b[?62;22c\x1bOPq\x1b[?0u");
        assert_eq!(found.len(), 2);
        assert_eq!(device_attributes(found[0]), Some(vec![62, 22]));
        assert!(kitty_keyboard(found[1]));
    }

    #[test]
    fn nothing_is_read_out_of_an_empty_or_reply_free_buffer() {
        assert!(all(b"").is_empty());
        assert!(all(b"hello").is_empty());
    }

    /// Garbage in the numeric fields must fail to parse rather than parse into
    /// something plausible.
    #[test]
    fn malformed_parameters_yield_no_value() {
        assert_eq!(cell_size(all(b"\x1b[6;xx;17t")[0]), None);
        assert_eq!(cell_size(all(b"\x1b[6;37t")[0]), None);
        assert_eq!(
            background_color(all(b"\x1b]11;rgb:zz/00/00\x1b\\")[0]),
            None
        );
        assert_eq!(
            background_color(all(b"\x1b]11;rgb:1d1d/1f1f\x1b\\")[0]),
            None
        );
        assert_eq!(background_color(all(b"\x1b]10;rgb:0/0/0\x1b\\")[0]), None);
        assert_eq!(background_color(all(b"\x1b]11;#12345\x1b\\")[0]), None);
    }

    /// A terminal that answers `CSI c` without the `?` (there are a few) is
    /// not answering `DA1`, and its parameters mean something else.
    #[test]
    fn a_device_attributes_reply_without_the_private_marker_is_ignored() {
        assert_eq!(device_attributes(all(b"\x1b[62;22c")[0]), None);
    }

    /// The scan must terminate on any input at all, including bytes chosen to
    /// look like the start of something.
    #[test]
    fn every_buffer_scans_to_an_end() {
        for bytes in [
            &b"\x1b\x1b\x1b\x1b"[..],
            b"\x1b[\x1b[\x1b[",
            b"\x1b]\x1b]\x1b]",
            b"\x1b[6;\x1b[6;\x1b[6;",
            b"\x1b_\x1b_\x1b_",
        ] {
            // None of these hold a complete sequence; a scan that failed to
            // advance would hang here rather than fail.
            assert_eq!(replies(bytes).count(), 0, "{bytes:?}");
        }
    }
}
