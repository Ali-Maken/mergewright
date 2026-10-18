//! A live operation stopped, and a stash put back that clashed.

/// A live operation stopped: what the footer's reason is worded from
/// (`banner::stop_reason`). The live working-tree read overrides `kind`'s
/// step and `paths_at_stop` while it names the same kind.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Stop {
    /// Whose operation this is: this run's, a previous run's, or one begun
    /// in the shell.
    pub origin: crate::update::relaunch::LiveOrigin,
    /// A rebase that stopped on a later step after a `c` (`stopped again`).
    pub again: bool,
    /// The operation git is part-way through.
    pub kind: mergewright_git::state::InProgress,
    /// Unmerged paths when it stopped — the count until a read says more.
    pub paths_at_stop: usize,
    /// A rewrite's stop, read on the job thread (m7-power-tools/07b): its
    /// step, rows and chain for the banner, and whether it is an `edit` stop.
    /// `None` outside a `rebase -i`; `Err` is the failed read, shown, never
    /// dropped.
    pub rewrite: Option<Result<crate::update::banner::RewriteStop, String>>,
}

/// A stash put back that clashed, and whose it is (m6-conflicts/72b):
/// [`AppState::clash`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClashState {
    /// The entry, the stash, its position now and why it was put back.
    pub clash: mergewright_ops::clash::StashClash,
    /// This run's clash, or one the launch found (`PreviousRun`).
    pub origin: crate::update::relaunch::LiveOrigin,
}
