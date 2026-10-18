//! Test-only: a hint's key as the shipped table spells it (ticket m8/10).
//!
//! A [`Hint`] names actions, not text, so its key is only text once a
//! keymap spells it. The app's tests run on the shipped table, so this reads
//! every hint against [`Keymap::shipped`] and lets a test still compare a
//! hint's key to the letter it expects.

use mergewright_ui::graph_screen::Hint;
use mergewright_ui::keymap::Keymap;

/// [`Hint::key_text`] against the shipped table.
pub(crate) trait Spelled {
    /// The hint's key as the shipped table spells it; `—` when the table
    /// leaves it unbound, so a test asserting a letter fails on the dash.
    /// Leaked — a test-only handful of short strings — so `(key, verb)`
    /// tuples still compare as `&str`.
    fn key(&self) -> &'static str;

    /// The hint's whole verb, leaked as [`Spelled::key`] is: a verb that
    /// names a branch or a count is built per bar (ease-pass/03b), so
    /// `Hint::verb` is a `Cow` and a `(key, verb)` tuple needs this.
    fn said(&self) -> &'static str;
}

impl Spelled for Hint {
    fn said(&self) -> &'static str {
        match &self.verb {
            std::borrow::Cow::Borrowed(verb) => verb,
            std::borrow::Cow::Owned(verb) => Box::leak(verb.clone().into_boxed_str()),
        }
    }

    fn key(&self) -> &'static str {
        let text = self
            .key_text(Keymap::shipped())
            .unwrap_or_else(|| "—".to_owned());
        Box::leak(text.into_boxed_str())
    }
}
