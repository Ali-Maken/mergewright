//! Errors from `core`'s own fallible constructors.
//!
//! This is deliberately *not* a shared error type other crates wrap: each
//! crate keeps its own (PROMPT.md §6.2). A constructor here rejects a value
//! that would break one of `core`'s own invariants — an object id of the
//! wrong length, a ref name git itself would refuse — which is the newtype
//! enforcing its shape, not an algorithm acting on domain data (DESIGN.md
//! §8.2, ADR-0007).

use std::fmt;

/// A value offered to a `core` constructor that the type cannot represent.
// `thiserror::Error` spelled out: this crate's own error type is called
// `Error` too, and importing the derive would shadow it.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    /// An object id was not one of git's two hash sizes.
    #[error("a git object id is 20 bytes (SHA-1) or 32 (SHA-256), not {len}")]
    OidLength {
        /// How many bytes were offered.
        len: usize,
    },
    /// An object id's hex text was not `2 × hash size` hex digits.
    #[error("not a git object id in hex: \"{text}\"")]
    OidHex {
        /// The offered text, escaped for display.
        text: String,
    },
    /// A ref name git's own `check-ref-format` rules reject.
    #[error("not a valid git ref name: \"{name}\" ({reason})")]
    RefName {
        /// The offered name, escaped for display.
        name: String,
        /// Which rule it broke.
        reason: RefNameRule,
    },
}

/// The `git check-ref-format` rule a rejected ref name broke.
///
/// Kept as an enum rather than a message so a caller can tell "empty" from
/// "contains a control character" — the second is a sign of a hostile
/// repository, the first of a bug in our own code.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RefNameRule {
    /// The name was empty.
    Empty,
    /// A control character, DEL or a space appeared in the name.
    ControlOrSpace,
    /// One of `~ ^ : ? * [ \` appeared in the name.
    ForbiddenCharacter,
    /// `..`, `@{` or a lone `@` appeared in the name.
    ForbiddenSequence,
    /// A `/`-separated component was empty, started with `.`, or ended with
    /// `.lock` — including the leading and trailing edges of the whole name.
    BadComponent,
    /// The name ended with a `.`.
    TrailingDot,
}

/// Not an error type of its own — it is the parenthesis in [`Error::RefName`]'s
/// message — so this stays a plain `Display`, hand-written like any other.
impl fmt::Display for RefNameRule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let reason = match self {
            RefNameRule::Empty => "it is empty",
            RefNameRule::ControlOrSpace => "it contains a control character or a space",
            RefNameRule::ForbiddenCharacter => "it contains one of ~ ^ : ? * [ \\",
            RefNameRule::ForbiddenSequence => "it contains .. or @{, or is a lone @",
            RefNameRule::BadComponent => {
                "a path component is empty, starts with a dot, or ends with .lock"
            }
            RefNameRule::TrailingDot => "it ends with a dot",
        };
        f.write_str(reason)
    }
}
