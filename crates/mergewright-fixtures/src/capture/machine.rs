//! Text that names the machine a file was written on, and the placeholders
//! that stand in for it.
//!
//! CLAUDE.md, Gates: *"Raw terminal streams never committed. The repo may be
//! published."* A capture is a picture of a running program, so it carries
//! whatever the program had on screen — and that included the author's home
//! directory, the scratch directory the fixture was copied into, and the port
//! the OS handed a test server. 124 tracked files named one person's account
//! and the layout of their disk (m5-history-and-ghost-preview/41).
//!
//! Three classes, and each is a different kind of wrong:
//!
//! * a **home directory** names a person — the reason this module exists;
//! * an **OS scratch directory** names one machine's private temp area, and
//!   its last segment is invented afresh every run, so a capture holding one
//!   can never be compared to anything;
//! * an **ephemeral port** is picked by the OS, so likewise.
//!
//! [`redacted`] replaces each with a placeholder of **exactly the same
//! width**, because a capture is a character grid: a replacement one column
//! short slides every following column left, and the grid the user judges by
//! eye stops being the grid the code drew. [`found`] reports the same runs
//! without changing them, which is what the repository-wide guard test uses.
//!
//! Same width has a price, and it is the honest one: a home directory two
//! characters longer redacts to a placeholder two characters longer, so this
//! alone does not make a capture identical on a machine whose paths are a
//! different length. That comes from the writer not drawing a real path at
//! all — `captures::CAPTURE_REPO_PATH` for the frames Rust draws — and this
//! module is the backstop under it. A capture taken through a pty shows the
//! scratch directory the app really opened, so it is reproducible on one
//! machine and not yet across machines; fixing that means giving the walk a
//! fixed scratch location, not a wider placeholder.
//!
//! This module is scanned by that guard like every other file, so it spells
//! the roots it forbids in two pieces (`concat!`) and assembles the ports its
//! own tests use, rather than carrying the literals it exists to ban. That is
//! the whole accommodation: the guard is never told to look away from a file,
//! because a guard with an exemption list is one entry away from being no
//! guard (m5-history-and-ghost-preview/41).

/// A class of machine-specific text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Machine {
    /// An absolute home directory: it names an account.
    Home,
    /// A path under the OS scratch directory: one machine's private temp
    /// area, with a name invented per run.
    Scratch,
    /// A port in the IANA dynamic range, which is one the OS picked.
    Port,
}

impl Machine {
    /// Why this class may not reach a tracked file.
    #[must_use]
    pub fn why(self) -> &'static str {
        match self {
            Self::Home => "a home directory names an account",
            Self::Scratch => "an OS scratch path names one machine, and changes every run",
            Self::Port => "an ephemeral port is the OS's choice, and changes every run",
        }
    }
}

/// One run of machine-specific text, as a byte range.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Found {
    /// Which class it is.
    pub machine: Machine,
    /// Where the run starts, in bytes.
    pub at: usize,
    /// How long the run is, in bytes — always equal to its width in columns,
    /// because a matched run is ASCII only.
    pub len: usize,
}

/// The macOS home root, in two pieces: this file is tracked, and [`found`]
/// reads it like any other (see the module note).
const MACOS_HOME: &str = concat!("/Us", "ers/");

/// The Linux home root; see [`MACOS_HOME`] for the spelling.
const LINUX_HOME: &str = concat!("/ho", "me/");

/// The macOS per-user scratch root; see [`MACOS_HOME`] for the spelling.
const MACOS_SCRATCH: &str = concat!("/var/fold", "ers/");

/// The same root as macOS reports it once resolved.
const MACOS_SCRATCH_REAL: &str = concat!("/private/var/fold", "ers/");

/// Hosts whose port a capture may show. Only a loopback address: a port after
/// anything else is a number in prose, not a socket this run opened.
const LOOPBACK: [&str; 5] = ["127.0.0.1:", "localhost:", "[::1]:", "0.0.0.0:", "[::]:"];

/// The IANA dynamic range — the ports an OS hands out when asked for any.
const EPHEMERAL: std::ops::RangeInclusive<u32> = 49152..=65535;

/// Every run of machine-specific text in `bytes`, in the order it appears.
///
/// Byte oriented so that a capture which is not UTF-8 — an escape stream, an
/// image — is read on the same terms as a text one. Every run it reports is
/// ASCII only, so its byte length is also its width in columns.
#[must_use]
pub fn found(bytes: &[u8]) -> Vec<Found> {
    let mut out = Vec::new();
    let mut at = 0;
    while at < bytes.len() {
        match matched(bytes, at) {
            Some(found) => {
                at = found.at + found.len;
                out.push(found);
            }
            None => at += 1,
        }
    }
    out
}

/// `text` with every run [`found`] reports replaced by a placeholder of the
/// same width.
/// Sliced rather than rebuilt from bytes: a run starts at `/` and ends where
/// a path segment does, both ASCII, so every index is a character boundary
/// and no fallible re-decode is needed.
#[must_use]
pub fn redacted(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut from = 0;
    for hit in found(text.as_bytes()) {
        out.push_str(&text[from..hit.at]);
        out.push_str(&placeholder(hit.machine, hit.len));
        from = hit.at + hit.len;
    }
    out.push_str(&text[from..]);
    out
}

/// [`redacted`] for a capture that is not text.
#[must_use]
pub fn redacted_bytes(bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(bytes.len());
    let mut from = 0;
    for hit in found(bytes) {
        out.extend_from_slice(&bytes[from..hit.at]);
        out.extend_from_slice(placeholder(hit.machine, hit.len).as_bytes());
        from = hit.at + hit.len;
    }
    out.extend_from_slice(&bytes[from..]);
    out
}

/// What stands in for one run of `width` columns.
///
/// A bracketed word, padded to the width it replaces so the grid does not
/// move: `<home____________>`. Too narrow to bracket, it is underscores —
/// still the right width, which is the property that matters.
fn placeholder(machine: Machine, width: usize) -> String {
    let stem = match machine {
        Machine::Home => "home",
        Machine::Scratch => "scratch",
        // A port is digits, and reads as one: 0 is not a port an OS assigns.
        Machine::Port => return "0".repeat(width),
    };
    if width >= stem.len() + 2 {
        format!("<{stem}{}>", "_".repeat(width - stem.len() - 2))
    } else if width >= 3 {
        format!("<{}>", &stem[..width - 2])
    } else {
        "_".repeat(width)
    }
}

/// The run starting at `at`, if one starts there.
fn matched(bytes: &[u8], at: usize) -> Option<Found> {
    if let Some(len) = home_at(bytes, at) {
        return Some(Found {
            machine: Machine::Home,
            at,
            len,
        });
    }
    if let Some(len) = scratch_at(bytes, at) {
        return Some(Found {
            machine: Machine::Scratch,
            at,
            len,
        });
    }
    port_at(bytes, at).map(|(at, len)| Found {
        machine: Machine::Port,
        at,
        len,
    })
}

/// A home root at `at`, with the account name after it, as a byte length.
///
/// The root must start a path — `left/right/home/end` is a list of key names,
/// not a directory — and the account name is taken as far as a path segment
/// runs. The tail is left alone: `…/dev/sandbox/mwr` is the repository, and
/// keeping it is what makes the redacted capture still say where it was
/// taken.
fn home_at(bytes: &[u8], at: usize) -> Option<usize> {
    let root = [MACOS_HOME, LINUX_HOME]
        .into_iter()
        .find(|root| bytes[at..].starts_with(root.as_bytes()))?;
    if !starts_a_path(bytes, at) {
        return None;
    }
    let mut len = root.len();
    while segment_byte(bytes.get(at + len).copied()) {
        len += 1;
    }
    Some(len)
}

/// A scratch path at `at`, whole, as a byte length.
///
/// Whole, unlike a home path, because every segment under the root is
/// invented per run — the temp directory's own name, then the fixture's.
/// There is nothing under it worth keeping and nothing under it that would be
/// the same twice.
///
/// The root must be followed by a real segment: `/var/fold` + `ers/…` with an
/// ellipsis is a document naming the class, which is exactly how this rule is
/// written about.
fn scratch_at(bytes: &[u8], at: usize) -> Option<usize> {
    let root = [MACOS_SCRATCH_REAL, MACOS_SCRATCH]
        .into_iter()
        .find(|root| bytes[at..].starts_with(root.as_bytes()))?;
    if !starts_a_path(bytes, at) || !segment_byte(bytes.get(at + root.len()).copied()) {
        return None;
    }
    let mut len = root.len();
    while segment_byte(bytes.get(at + len).copied()) || bytes.get(at + len) == Some(&b'/') {
        len += 1;
    }
    Some(len)
}

/// An ephemeral port at `at`, as the range of its digits — the host stays, so
/// the line still says it is loopback.
fn port_at(bytes: &[u8], at: usize) -> Option<(usize, usize)> {
    let host = LOOPBACK
        .into_iter()
        .find(|host| bytes[at..].starts_with(host.as_bytes()))?;
    let digits_at = at + host.len();
    let mut len = 0;
    while bytes.get(digits_at + len).is_some_and(u8::is_ascii_digit) {
        len += 1;
    }
    if !(4..=5).contains(&len) {
        return None;
    }
    let number: u32 = std::str::from_utf8(&bytes[digits_at..digits_at + len])
        .ok()?
        .parse()
        .ok()?;
    EPHEMERAL.contains(&number).then_some((digits_at, len))
}

/// Could a path start at `at` — or is this the middle of a longer one?
///
/// Only the middle of a path disqualifies it, which is one step back from
/// what precedes it: `left/right/home/end` is a list of key names because the
/// word before the root is itself preceded by a separator. A capture is often
/// a raw escape stream, where the root follows the `m` that ends a colour
/// sequence with no space between them — that is a path, and reading only the
/// single byte before missed all four of those files.
fn starts_a_path(bytes: &[u8], at: usize) -> bool {
    let Some(mut before) = at.checked_sub(1) else {
        return true;
    };
    if bytes[before] == b'/' {
        return false;
    }
    while before > 0 && segment_byte(Some(bytes[before])) {
        before -= 1;
    }
    !(segment_byte(Some(bytes[at - 1])) && bytes[before] == b'/')
}

/// Is this a byte a path segment is made of? ASCII only, so that a matched
/// run is as many columns as it is bytes.
fn segment_byte(byte: Option<u8>) -> bool {
    byte.is_some_and(|byte| {
        byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-' | b'+')
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The roots, spelled once, for tests that need the literal.
    fn home(name: &str) -> String {
        format!("{MACOS_HOME}{name}")
    }

    fn scratch(rest: &str) -> String {
        format!("{MACOS_SCRATCH}{rest}")
    }

    /// A loopback address on an ephemeral port, assembled rather than
    /// spelled, for the reason the roots are (see the module note): the guard
    /// reads this file like any other, and a literal here is a literal it
    /// would have to find.
    fn loopback(port: u32) -> String {
        format!("{}{port}", LOOPBACK[0])
    }

    #[test]
    fn a_home_directory_is_found_and_its_tail_is_kept() {
        let line = format!("mwr {}/dev/sandbox/mwr  73 commits", home("ada"));
        let out = redacted(&line);
        assert_eq!(out, "mwr <home____>/dev/sandbox/mwr  73 commits");
        assert!(found(out.as_bytes()).is_empty(), "{out}");
    }

    #[test]
    fn a_placeholder_is_as_wide_as_what_it_replaces() {
        for name in ["a", "ada", "devuser", "a-very-long-account-name"] {
            let line = format!("mwr {}/dev  x", home(name));
            let out = redacted(&line);
            assert_eq!(
                out.chars().count(),
                line.chars().count(),
                "{name}: the grid moved"
            );
        }
    }

    #[test]
    fn both_home_roots_are_found() {
        for root in [MACOS_HOME, LINUX_HOME] {
            let line = format!("{root}someone/.ssh");
            assert!(!found(line.as_bytes()).is_empty(), "{line} was allowed");
        }
    }

    #[test]
    fn a_home_root_that_does_not_start_a_path_is_not_one() {
        // `SecretField::left/right/home/end` — key names, not a directory.
        let line = "SecretField::left/right/home/end do not pulse";
        assert_eq!(found(line.as_bytes()), vec![]);
        assert_eq!(redacted(line), line);
    }

    #[test]
    fn a_root_that_follows_an_escape_sequence_is_still_a_path() {
        // Four committed captures are raw escape streams, where the colour
        // sequence's `m` abuts the path.
        let line = format!("\u{1b}[38;2;215;219;224;49m{MACOS_HOME}ada/dev");
        let out = redacted(&line);
        assert!(found(out.as_bytes()).is_empty(), "{out}");
        assert!(out.ends_with("49m<home____>/dev"), "{out}");
    }

    #[test]
    fn a_root_in_the_middle_of_a_longer_path_is_not_a_root() {
        let line = format!("/mnt{MACOS_HOME}bob");
        assert_eq!(found(line.as_bytes()), vec![]);
    }

    #[test]
    fn a_scratch_path_is_replaced_whole() {
        let line = format!(
            "mwr {}  47 commits",
            scratch("z4/tqdvc9557mv3f1/T/mwr-remote-idICFn/work")
        );
        let out = redacted(&line);
        assert!(out.contains("<scratch"), "{out}");
        assert!(out.ends_with(">  47 commits"), "{out}");
        assert_eq!(out.chars().count(), line.chars().count());
        assert!(found(out.as_bytes()).is_empty(), "{out}");
    }

    #[test]
    fn a_scratch_path_stops_at_the_column_the_next_text_starts() {
        let line = format!("{} rows", scratch("z4/h/T/x"));
        let out = redacted(&line);
        assert!(out.ends_with("> rows"), "{out}");
    }

    #[test]
    fn an_elided_scratch_root_is_prose_about_the_rule_not_a_leak() {
        let line = format!("headers carry {MACOS_SCRATCH}…/mwr-remote");
        assert_eq!(found(line.as_bytes()), vec![]);
    }

    #[test]
    fn the_resolved_scratch_root_is_found_too() {
        let line = format!("{MACOS_SCRATCH_REAL}z4/h/T/x");
        let out = redacted(&line);
        assert!(found(out.as_bytes()).is_empty(), "{out}");
        assert_eq!(out.chars().count(), line.chars().count());
    }

    #[test]
    fn an_ephemeral_port_is_zeroed_and_the_host_stays() {
        let host = LOOPBACK[0];
        assert_eq!(
            redacted(&format!("url http://{}/bare", loopback(54372))),
            format!("url http://{host}00000/bare")
        );
    }

    #[test]
    fn a_chosen_port_is_not_an_ephemeral_one() {
        for kept in ["127.0.0.1:8443", "localhost:8765", "127.0.0.1:443"] {
            assert_eq!(redacted(kept), kept, "{kept} is a port somebody chose");
        }
    }

    #[test]
    fn a_port_after_something_that_is_not_loopback_is_a_number_in_prose() {
        let line = "deps/it-8f3aee7a:54372 exited";
        assert_eq!(found(line.as_bytes()), vec![]);
    }

    #[test]
    fn every_class_in_one_line_is_found_once_each() {
        let line = format!(
            "{} {} {}",
            home("ada"),
            scratch("z4/h/T/x"),
            loopback(51000)
        );
        let classes: Vec<Machine> = found(line.as_bytes())
            .into_iter()
            .map(|hit| hit.machine)
            .collect();
        assert_eq!(
            classes,
            vec![Machine::Home, Machine::Scratch, Machine::Port]
        );
    }

    #[test]
    fn redacting_twice_changes_nothing_the_first_pass_left() {
        let line = format!(
            "{}/dev {} {}",
            home("ada"),
            scratch("z4/h/T/x"),
            loopback(51000)
        );
        let once = redacted(&line);
        assert_eq!(redacted(&once), once);
    }

    #[test]
    fn bytes_that_are_not_utf8_are_redacted_around() {
        let mut bytes = b"\x1b]0;".to_vec();
        bytes.extend_from_slice(home("ada").as_bytes());
        bytes.extend_from_slice(&[0xff, 0xfe]);
        let out = redacted_bytes(&bytes);
        assert_eq!(out.len(), bytes.len());
        assert!(found(&out).is_empty());
        assert_eq!(&out[out.len() - 2..], &[0xff, 0xfe]);
    }

    #[test]
    fn a_placeholder_too_narrow_to_bracket_is_still_the_right_width() {
        assert_eq!(placeholder(Machine::Home, 2), "__");
        assert_eq!(placeholder(Machine::Home, 4), "<ho>");
        assert_eq!(placeholder(Machine::Scratch, 9), "<scratch>");
        assert_eq!(placeholder(Machine::Port, 5), "00000");
    }

    #[test]
    fn a_home_root_with_no_account_after_it_is_still_a_leak() {
        // Otherwise the rule is one grep away from being untrue again.
        assert_eq!(found(MACOS_HOME.as_bytes()).len(), 1);
    }
}
