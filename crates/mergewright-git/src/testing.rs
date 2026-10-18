//! Test support; no production caller.
//!
//! What another crate's tests need to look inside this crate's types, kept
//! apart so the production surface stays what production uses.

use crate::run::GitInvocation;

impl GitInvocation {
    /// Whether [`stop_hooks_first`](GitInvocation::stop_hooks_first) marked
    /// this invocation: a caller's own test of which of its lines it marks.
    #[must_use]
    pub fn stops_hooks_first(&self) -> bool {
        self.marked_hooks_first()
    }
}
