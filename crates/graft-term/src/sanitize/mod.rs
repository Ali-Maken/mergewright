//! Making repository text safe to draw.
//!
//! A repository is untrusted input. A commit message, an author name, a ref
//! name or a path can carry escape sequences that repaint the screen, retitle
//! the window, open a hyperlink to somewhere else, or reorder what is drawn so
//! that the text on screen disagrees with the text git will act on. Graft
//! never writes such a string to the terminal: it writes a [`SanitizedText`],
//! and [`sanitize`] (with its byte and `OsStr` siblings) is the only way to
//! get one. Holding the type is proof the cleaning ran, so an unsanitised
//! `String` reaching `ui` is a compile error rather than a review catch
//! (ARCHITECTURE.md, DESIGN.md §8.2, PROMPT.md §6.3).
//!
//! # What is neutralised
//!
//! Nothing is ever deleted. Each hostile character is replaced by one visible
//! placeholder, so a spoofing attempt shows up as a run of odd glyphs in the
//! middle of a branch name instead of disappearing into whitespace:
//!
//! - `␛` (U+241B) for control characters — C0, `DEL` and the C1 block
//!   U+0080–U+009F. That single rule kills every CSI, OSC (OSC 8 hyperlinks
//!   included), DCS, APC and PM sequence in both their ESC-introduced and
//!   C1-introduced spellings.
//! - `�` (U+FFFD) for characters that are legal text but invisible or
//!   direction-changing: **all** of Unicode's format (`Cf`) category — which
//!   contains the bidi overrides and isolates (U+202A–U+202E, U+2066–U+2069)
//!   and the zero-width characters (U+200B–U+200F, U+2060, U+FEFF) that
//!   DESIGN.md §8.2 names, plus the soft hyphen, U+061C, the interlinear
//!   annotation characters and the U+E0000 tag block, which can carry a whole
//!   hidden sentence — and the line and paragraph separators U+2028/U+2029.
//!
//! Taking the whole `Cf` category rather than §8.2's five ranges is a
//! deliberate widening: the ranges §8.2 lists are a subset of it, and a rule
//! stated as a category survives the next Unicode revision, where an
//! enumerated list quietly stops being complete.
//!
//! # `\n` and `\t`, and which entry point to use
//!
//! [`sanitize`] lets `\n` and `\t` through, because a commit body without
//! them is unreadable and the widget that draws a body lays them out itself
//! (DESIGN.md §8.2). That makes it the wrong function for anything drawn on
//! **one** line: a file name may legally contain a newline or a tab, and drawn
//! raw into a status row or a diff header it scrolls the screen or jumps the
//! cursor to the next tab stop. Single-line sites call [`sanitize_line`],
//! which replaces those two like any other control. The path entry points
//! ([`sanitize_path`], [`sanitize_path_bytes`]) are single-line for the same
//! reason.
//!
//! # What is not
//!
//! Sanitising is not a width, a normalisation or a confusability check.
//! Combining-mark stacks ("Zalgo"), ambiguous-width and fullwidth characters,
//! and homoglyph lookalikes (Cyrillic `а` for Latin `a`) all survive: they
//! cannot make the terminal do anything, only make it look wrong, and
//! clamping them belongs to the width-aware rendering layer.
//!
//! Invisible characters that are *not* `Cf` survive too, and that is a
//! judgement call rather than an oversight: the variation selectors
//! (U+FE00–U+FE0F, U+E0100–U+E01EF) and the Hangul fillers are used as a
//! payload channel, but U+FE0F is also what makes an ordinary emoji render as
//! an emoji and U+E0100 selects a legitimate CJK glyph variant. Replacing
//! them would corrupt normal text to block a trick that can only mislead the
//! eye, never drive the terminal. Recorded in KNOWN_GAPS.md.
//!
//! # Reading the output
//!
//! `�` at the end of all this means one of three things — an invalid byte
//! that [`sanitize_path_bytes`] decoded lossily, a character replaced here, or
//! a U+FFFD that really was in the input (those pass through unchanged, which
//! is what makes sanitising idempotent). The type deliberately does not
//! distinguish them: the user is being told "this text is not what it looks
//! like", and which of the three it was does not change what they should do.

#[cfg(test)]
mod tests;

use std::fmt;
use std::ops::Deref;
use std::path::Path;

/// Stands in for a control character: `␛`, U+241B SYMBOL FOR ESCAPE.
const CONTROL_PLACEHOLDER: char = '\u{241B}';

/// Stands in for an invisible or direction-changing character: `�`, U+FFFD
/// REPLACEMENT CHARACTER.
const INVISIBLE_PLACEHOLDER: char = '\u{FFFD}';

/// Text that has been through [`sanitize`] and is safe to write to the
/// terminal.
///
/// The field is private and this module holds the only constructors, so a
/// value of this type cannot exist without the cleaning having run. Every
/// view-model field carrying repo-derived text is typed `SanitizedText` for
/// exactly that reason (DESIGN.md §8.2).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SanitizedText(String);

impl SanitizedText {
    /// The cleaned text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Deref for SanitizedText {
    type Target = str;

    fn deref(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for SanitizedText {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for SanitizedText {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Cleans text that may legitimately span several lines — a commit message
/// body, a diff hunk, the text of a conflict.
///
/// Every control, bidi and invisible character becomes a visible placeholder;
/// `\n` and `\t` are left for the widget to lay out. The result has exactly
/// as many characters as the input, and sanitising it again changes nothing.
///
/// For anything drawn on a single line use [`sanitize_line`] instead.
#[must_use]
pub fn sanitize(text: &str) -> SanitizedText {
    clean(text, Newlines::Keep)
}

/// Cleans text that will be drawn on one line — a commit subject, an author
/// name, a ref badge, a status row.
///
/// As [`sanitize`], except that `\n` and `\t` are replaced too: on a
/// single-line widget a raw newline scrolls the screen and a raw tab jumps the
/// cursor, both of them straight from repository content.
#[must_use]
pub fn sanitize_line(text: &str) -> SanitizedText {
    clean(text, Newlines::Replace)
}

/// Cleans a path or file name as the operating system gave it to us — on Unix
/// arbitrary bytes, on Windows possibly ill-formed UTF-16.
///
/// Lossy: anything that is not valid UTF-8 becomes U+FFFD before the cleaning
/// runs, so an escape sequence hidden in the valid part of a broken path is
/// still caught. Single-line, like [`sanitize_line`], because file names are
/// drawn in rows and headers (PROMPT.md §6.3).
#[must_use]
pub fn sanitize_path(path: &Path) -> SanitizedText {
    clean(&path.to_string_lossy(), Newlines::Replace)
}

/// Cleans a path that arrived as raw bytes, which is how git reports one.
///
/// Identical to [`sanitize_path`]; it exists because a path out of git is
/// bytes long before it is an `OsStr`, and routing it through a
/// platform-specific conversion first would be a second place to get the
/// lossy step wrong.
#[must_use]
pub fn sanitize_path_bytes(bytes: &[u8]) -> SanitizedText {
    clean(&String::from_utf8_lossy(bytes), Newlines::Replace)
}

/// Whether `\n` and `\t` are structural text or two more controls.
#[derive(Clone, Copy)]
enum Newlines {
    /// Multi-line text: the widget lays them out (DESIGN.md §8.2).
    Keep,
    /// Single-line text: they would move the cursor, so they are replaced.
    Replace,
}

/// The one loop. Every entry point above is this plus a mode.
fn clean(text: &str, newlines: Newlines) -> SanitizedText {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match classify(c, newlines) {
            Verdict::Keep => out.push(c),
            Verdict::Control => out.push(CONTROL_PLACEHOLDER),
            Verdict::Invisible => out.push(INVISIBLE_PLACEHOLDER),
        }
    }
    SanitizedText(out)
}

/// What sanitising does to one character.
enum Verdict {
    /// Harmless: it goes through unchanged.
    Keep,
    /// A control character; it becomes [`CONTROL_PLACEHOLDER`].
    Control,
    /// Legal text, but invisible or direction-changing; it becomes
    /// [`INVISIBLE_PLACEHOLDER`].
    Invisible,
}

/// The single decision this module makes. Everything above is plumbing.
fn classify(c: char, newlines: Newlines) -> Verdict {
    match c {
        // Structural whitespace, or two more controls, depending on where the
        // text is going. Listed first so the control arm below doesn't have to
        // carve them out.
        '\n' | '\t' => match newlines {
            Newlines::Keep => Verdict::Keep,
            Newlines::Replace => Verdict::Control,
        },

        // C0, DEL and C1. ESC (U+001B) is in here, which is what makes every
        // CSI/OSC/DCS/APC/PM sequence inert; the C1 block covers the
        // single-byte spellings of the same introducers (0x9B CSI, 0x9D OSC).
        '\u{0}'..='\u{1F}' | '\u{7F}' | '\u{80}'..='\u{9F}' => Verdict::Control,

        // Unicode's format (`Cf`) category, as of Unicode 16.0. DESIGN.md
        // §8.2 names the bidi overrides and isolates (U+202A–U+202E,
        // U+2066–U+2069) and the zero-width characters (U+200B–U+200F,
        // U+2060, U+FEFF); the rest are here because they are invisible by
        // the same argument — U+00AD soft hyphen, U+061C Arabic letter mark
        // (a bidi control §8.2's list misses), U+180E Mongolian vowel
        // separator, the interlinear annotation characters, and the U+E0000
        // tag block, which can carry a whole hidden sentence. U+2065 sits
        // unassigned inside one of those runs and is neutralised with them:
        // an unassigned code point in a ref name is not text anyone typed.
        '\u{AD}'
        | '\u{600}'..='\u{605}'
        | '\u{61C}'
        | '\u{6DD}'
        | '\u{70F}'
        | '\u{890}'..='\u{891}'
        | '\u{8E2}'
        | '\u{180E}'
        | '\u{200B}'..='\u{200F}'
        | '\u{202A}'..='\u{202E}'
        | '\u{2060}'..='\u{206F}'
        | '\u{FEFF}'
        | '\u{FFF9}'..='\u{FFFB}'
        | '\u{110BD}'
        | '\u{110CD}'
        | '\u{13430}'..='\u{1343F}'
        | '\u{1BCA0}'..='\u{1BCA3}'
        | '\u{1D173}'..='\u{1D17A}'
        | '\u{E0001}'
        | '\u{E0020}'..='\u{E007F}' => Verdict::Invisible,

        // Line and paragraph separators: not `Cf`, but they end a line in
        // anything that understands them, which no widget expects from the
        // middle of a branch name.
        '\u{2028}' | '\u{2029}' => Verdict::Invisible,

        _ => Verdict::Keep,
    }
}
