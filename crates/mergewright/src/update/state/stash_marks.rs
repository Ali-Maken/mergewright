//! Which commits only a stash reaches.

use std::sync::Arc;

use mergewright_core::job::JobId;

/// Which commits only a stash reaches (`mergewright_git::history_reads::only_in_stash`,
/// m7-power-tools/16): the graph draws those rows `□`, dimmed, badged
/// `only in stash@{n}` (`view::rows`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StashOnlyMarks {
    /// The read in flight, set when a refs read with a stash answers. An
    /// answer under any other id is stale (the refs moved again) and dropped.
    pub job: Option<JobId>,
    /// The newest answer: each commit and the newest entry `n` (`stash@{n}`)
    /// reaching it. Kept while the next read runs — the refs a read follows
    /// rarely change what only a stash holds, and dropping the marks for a
    /// moment on every refresh would flicker them. Emptied when a refs read
    /// lists no stash, and when a read fails. Shared: the state is cloned
    /// per event.
    pub commits: Arc<std::collections::HashMap<mergewright_core::oid::Oid, usize>>,
}

impl StashOnlyMarks {
    /// `stash@{n}` for `id`, when only a stash reaches it.
    #[must_use]
    pub fn entry(&self, id: mergewright_core::oid::Oid) -> Option<usize> {
        self.commits.get(&id).copied()
    }
}
