//! What the status line has to say.

/// What the status line has to say.
///
/// Modelled as a state rather than as a string with an `error:` prefix: a
/// field that always looks like an error teaches the user to ignore it, which
/// is how a real failure gets missed.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Status {
    /// Nothing to report.
    #[default]
    Ready,
    /// A background job failed, and this is what it said — a read, not
    /// something the user asked for. Shown until a later job succeeds
    /// (`JobFinished`) or `Esc` dismisses it; a failure is never dropped on
    /// the floor.
    Failed(String),
    /// A line that stays until `Esc` dismisses it; `kind` says whether it is
    /// a failure (drawn `⊗`) or a notice. Dismissed by hand, never by a
    /// later job.
    ///
    /// The failures it carries must outlive the refresh they trigger: a hook
    /// that rejected a commit, a patch git would not apply, an amend whose
    /// `HEAD` message could not be read, an `$EDITOR` that would not start.
    /// Apart from [`Status::Failed`] because the two are stale for different
    /// reasons: a read that failed on its own is superseded by the next read
    /// that works, so `JobFinished` clears it; the refresh a write failure
    /// itself triggers finishes a moment later, and clearing on that wiped a
    /// rejected hook's message off the screen before it could be read (m3/04
    /// review, `KNOWN_GAPS.md`).
    ///
    /// The rest are notices: a refusal already worded, the leftovers of a
    /// cancel he asked for, an offer with keys. They live as long, but are
    /// not failures, so they draw no mark (ease-pass 28, Q1). Default
    /// [`Status::sticky`]: an unaudited site never shows a false `⊗`.
    ///
    /// Split into `reason` and `keys` (`hint-bar-redesign`, ADR-0037 §4) so
    /// the graph footer can keep `keys` whole and cut `reason` first when
    /// the line does not fit (`push rejected: … · p pull · P force` at 80
    /// columns, `m4-07-acceptance` M2). `keys` is `None` for the (common)
    /// message with no key to keep.
    Sticky {
        /// What went wrong, in words.
        reason: String,
        /// The key(s) that answer it, worded as they'd read in the hint bar
        /// (`"p pull · P force"`). Only `remote_failed` builds `Some` today
        /// — it parses `ops`' "reason · keys" convention out of the failure
        /// line it is handed, since `mergewright-ops` is out of this ticket's
        /// crate set.
        keys: Option<String>,
        /// Whether this sticky line reports a failure (drawn `⊗`) or a notice.
        kind: mergewright_ui::graph_screen::ProblemKind,
    },
    /// `y` sent the selected commit's SHA to the terminal's clipboard
    /// request (OSC 52); `short` is its first seven hex digits. Says what
    /// Mergewright sent, not that the clipboard has it: nothing comes back to
    /// tell (ADR-0023). Cleared by the next key.
    Copied {
        /// The SHA's first seven hex digits.
        short: String,
    },
    /// A key could not act yet because a read it needs is still in flight
    /// (`still reading the refs — try again in a moment`). Nothing is
    /// wrong, so it is drawn as a notice, never as a failure (CLAUDE.md: a
    /// status must never read as an error when nothing is wrong; review
    /// 47-49, m1). Cleared by the next key, like `Copied`.
    Waiting(String),
    /// A working-tree write is running: `<description>…`
    /// (m3-working-tree-undo-and-stash/04).
    Busy(String),
    /// It finished: a working-tree write in the journal's own words (`stage
    /// src/a.rs`), or a verdict (`every file done`). Drawn `✓ <what>`, then
    /// ` · <also>` when something else is true after it (`your edits are
    /// back`), then two spaces and `<keys>` when there is a next key to
    /// name (`c finishes the merge`), by `view::status::done_line` — the one place
    /// that draws a `Done`. Cleared by the next key, like `Copied`.
    ///
    /// The keys are kept apart, as `Sticky`'s are, because the tick is the
    /// renderer's: glued into `what`, it landed after the key words
    /// (`… · c finishes the merge ✓`, ticket 41). `what` never holds `✓`.
    Done {
        /// What finished, without the tick.
        what: String,
        /// What else is true once it finished, drawn after the tick and
        /// before the keys (`your edits are back`, m6-conflicts/84). Never
        /// a key: a caller that re-words the keys (`undo::land_after_undo`)
        /// leaves it alone.
        also: Option<String>,
        /// The next key, worded as it reads in the hint bar, if any.
        keys: Option<String>,
    },
}

impl Status {
    /// A sticky notice with no keys to keep whole: a refusal, a cancel's
    /// leftovers, an offer. Draws no failure mark.
    pub fn sticky(reason: impl Into<String>) -> Status {
        Status::Sticky {
            reason: reason.into(),
            keys: None,
            kind: mergewright_ui::graph_screen::ProblemKind::Notice,
        }
    }

    /// [`Status::sticky`] with the keys that answer it.
    pub fn sticky_keys(reason: impl Into<String>, keys: Option<String>) -> Status {
        Status::Sticky {
            reason: reason.into(),
            keys,
            kind: mergewright_ui::graph_screen::ProblemKind::Notice,
        }
    }

    /// [`Status::sticky_failure`] with the keys that answer it.
    pub fn sticky_failure_keys(reason: impl Into<String>, keys: Option<String>) -> Status {
        Status::Sticky {
            reason: reason.into(),
            keys,
            kind: mergewright_ui::graph_screen::ProblemKind::Failure,
        }
    }

    /// A sticky failure: what he asked for did not happen. Outlives its
    /// refresh; drawn `⊗`.
    pub fn sticky_failure(reason: impl Into<String>) -> Status {
        Status::Sticky {
            reason: reason.into(),
            keys: None,
            kind: mergewright_ui::graph_screen::ProblemKind::Failure,
        }
    }

    /// `Done` with no keys: drawn `✓ <what>`.
    pub fn done(what: impl Into<String>) -> Status {
        Status::Done {
            what: what.into(),
            also: None,
            keys: None,
        }
    }

    /// `Done` whose line names the next key: drawn `✓ <what>  <keys>`.
    pub fn done_with_keys(what: impl Into<String>, keys: impl Into<String>) -> Status {
        Status::Done {
            what: what.into(),
            also: None,
            keys: Some(keys.into()),
        }
    }

    /// A sticky line's words and keys, whatever its kind; `None` for any other status.
    #[must_use]
    pub fn sticky_parts(&self) -> Option<(&String, &Option<String>)> {
        match self {
            Status::Sticky { reason, keys, .. } => Some((reason, keys)),
            _ => None,
        }
    }

    /// A failure — `Failed` or `Sticky` — the kind that stays until `Esc`
    /// dismisses it, and so the kind a box must not let go unseen
    /// (m6-conflicts/78).
    #[must_use]
    pub(crate) fn is_failure(&self) -> bool {
        matches!(self, Status::Failed(_) | Status::Sticky { .. })
    }
}
