//! `Operation` queries: kind, describe and friends.

use super::{
    FactsScope, InProgress, OnEdits, Operation, OperationKind, PutBackWhy, RepoFacts, ResetTarget,
    SetAside, Tip, Touches, commit, conflict, finish_merge, history, opens_in_progress, recover,
    remote, rewrite, stage, stash,
};

impl Operation {
    /// Which journal kind this records as.
    #[must_use]
    pub fn kind(&self) -> OperationKind {
        match self {
            Self::Stage(_) => OperationKind::Stage,
            Self::Unstage(_) => OperationKind::Unstage,
            Self::Discard(_) => OperationKind::Discard,
            Self::Commit(_) => OperationKind::Commit,
            Self::Amend(_) => OperationKind::Amend,
            Self::StashPush(_) => OperationKind::StashPush,
            Self::StashApply { .. } => OperationKind::StashApply,
            Self::StashPop { .. } => OperationKind::StashPop,
            Self::StashDrop { .. } | Self::DropPutBack { .. } => OperationKind::StashDrop,
            Self::Fetch(_) => OperationKind::Fetch,
            Self::Pull(_) => OperationKind::Pull,
            Self::Push(_) => OperationKind::Push,
            Self::Merge(_) => OperationKind::Merge,
            Self::Rebase(_) => OperationKind::Rebase,
            Self::Rewrite(_) => OperationKind::Rewrite,
            Self::CherryPick(_) => OperationKind::CherryPick,
            Self::Revert(_) => OperationKind::Revert,
            Self::Reset(_) => OperationKind::Reset,
            Self::Checkout(_) => OperationKind::Checkout,
            Self::BranchCreate(_) => OperationKind::BranchCreate,
            Self::BranchDelete(_) => OperationKind::BranchDelete,
            Self::TagCreate(_) => OperationKind::TagCreate,
            Self::TagDelete(_) => OperationKind::TagDelete,
            Self::BranchFromStash(_) => OperationKind::BranchFromStash,
            Self::ResolveWholeFile(_) => OperationKind::ResolveConflict,
            Self::RestartConflict(_) => OperationKind::RestartConflict,
            Self::FinishMerge(_) => OperationKind::FinishMerge,
            Self::AbortMerge(_) => OperationKind::AbortMerge,
            Self::Recover(_) => OperationKind::Recover,
        }
    }

    /// The in-progress state this operation itself opens when git stops on
    /// a conflict: what the executor matches `in_progress` against to tell
    /// "stopped, still live" from "failed" (m5/07). `None` for every other
    /// operation.
    #[must_use]
    pub fn starts_in_progress(&self) -> Option<InProgress> {
        opens_in_progress(self.kind())
    }

    /// A history operation (m5/07): its preview is drawn from ghosts and
    /// ref moves, and the executor re-reads everything after it.
    #[must_use]
    pub fn is_history(&self) -> bool {
        matches!(
            self,
            Self::Merge(_)
                | Self::Rebase(_)
                | Self::Rewrite(_)
                | Self::CherryPick(_)
                | Self::Revert(_)
                | Self::Reset(_)
                | Self::Checkout(_)
                | Self::BranchCreate(_)
                | Self::BranchDelete(_)
                | Self::TagCreate(_)
                | Self::TagDelete(_)
                | Self::BranchFromStash(_)
        )
    }

    /// True when finishing this write moves `HEAD` — its ref or its commit —
    /// so the graph cursor lands on the new `HEAD`'s row after it (UX §3.1,
    /// m7-power-tools/03). False for everything that leaves `HEAD` where it
    /// was: stage, stash, tag, fetch, push, a branch moved or deleted that
    /// `HEAD` is not on, and the resolver's per-file writes. A put-back
    /// (`Recover`) is false: the journal screen owns its own landing.
    ///
    /// No `_` arm: a new kind must say which it is.
    #[must_use]
    pub fn moves_head(&self) -> bool {
        match self {
            Self::Commit(_)
            | Self::Amend(_)
            | Self::Merge(_)
            | Self::Rebase(_)
            | Self::Rewrite(_)
            | Self::CherryPick(_)
            | Self::Revert(_)
            | Self::Pull(_)
            | Self::Checkout(_)
            | Self::BranchFromStash(_)
            // The shell-begun merge, committed: a new merge commit on
            // `HEAD`. Sent by `apply::resume`, whose finish lands already.
            | Self::FinishMerge(_) => true,
            Self::Reset(params) => matches!(params.target, ResetTarget::Head { .. }),
            // A new branch switched to moves `HEAD`, as a checkout does.
            Self::BranchCreate(params) => params.checkout,
            Self::Stage(_)
            | Self::Unstage(_)
            | Self::Discard(_)
            | Self::StashPush(_)
            | Self::StashApply { .. }
            | Self::StashPop { .. }
            | Self::StashDrop { .. }
            | Self::Fetch(_)
            | Self::Push(_)
            | Self::BranchDelete(_)
            | Self::TagCreate(_)
            | Self::TagDelete(_)
            | Self::ResolveWholeFile(_)
            | Self::RestartConflict(_)
            // `merge --abort` puts the tree back to `HEAD`; `HEAD` stays.
            | Self::AbortMerge(_)
            | Self::DropPutBack { .. }
            | Self::Recover(_) => false,
        }
    }

    /// What a rebase, rewrite, cherry-pick, merge, previewed switch or
    /// switching create away from `HEAD` would set aside before it runs and put back
    /// after (ADR-0045 §4): `Some` when tracked files are changed,
    /// staged or not; `None` on a clean tree, over untracked files only
    /// (they stay where they are), and for every other kind. Pure over the
    /// facts — the preview and `apply` ask the same question.
    ///
    /// A cherry-pick (m7-power-tools/11a) wraps any changed tracked file,
    /// as a rebase does, not only the paths the pick touches: those need a
    /// read of the commit, and `sets_aside` reads none. A file the pick
    /// leaves alone goes back as it was.
    ///
    /// A pull (m7-power-tools/03b) wraps only a fast-forward whose incoming
    /// paths meet a changed tracked file (a rename's original path counts),
    /// read from [`RepoFacts::incoming`] after its fetch; then it sets aside
    /// every changed tracked file, as a rebase does. Diverged, up to date,
    /// no overlap, or not fetched yet: `None`, and git's own words.
    ///
    /// A merge (m8-polish-and-ship/19d) wraps as a cherry-pick does: any
    /// changed tracked file, fast-forward or not. git refuses a true merge
    /// over any staged change, even one the merge does not touch, and the
    /// paths it touches need a read of the commits, which this makes none of.
    #[must_use]
    pub fn sets_aside(&self, facts: &RepoFacts) -> Option<SetAside> {
        let wraps = match self {
            Self::Rebase(_) | Self::Rewrite(_) | Self::CherryPick(_) | Self::Merge(_) => true,
            Self::Pull(_) => remote::pull_overlaps(facts),
            // A switch sets aside only when it was previewed (the switch
            // rule, m7-power-tools/10b); a new branch at `HEAD`'s own commit
            // leaves the edits where they are.
            Self::Checkout(params) => params.on_edits == OnEdits::SetAside,
            Self::BranchCreate(params) => {
                params.checkout
                    && params.on_edits == OnEdits::SetAside
                    && !history::creates_at_head(params, facts)
            }
            // A branch put back while checked out is a `reset --hard`.
            Self::Recover(params) => recover::moves_checked_out(params, facts),
            _ => false,
        };
        if !wraps {
            return None;
        }
        let files = facts
            .status
            .entries
            .iter()
            .filter(|entry| entry.is_staged() || entry.is_unstaged())
            .count();
        (files > 0).then_some(SetAside { files })
    }

    /// `refs/stash`, when this operation sets edits aside.
    ///
    /// Also a pull's (m7-power-tools/03b): its `touches` add it once the
    /// fetch has filled [`RepoFacts::incoming`].
    pub(super) fn stash_ref_if_set_aside(&self, facts: &RepoFacts) -> Option<String> {
        self.sets_aside(facts).map(|_| stash::STASH_REF.to_owned())
    }

    /// `touches`, with `refs/stash` added when this operation sets edits
    /// aside (a switch's).
    pub(super) fn and_stash_if_set_aside(
        &self,
        mut touches: Touches,
        facts: &RepoFacts,
    ) -> Touches {
        touches.refs.extend(self.stash_ref_if_set_aside(facts));
        touches
    }

    /// Every [`Tip`] this operation is pinned to: what the Ghost Preview was
    /// drawn against, and what `check` refuses [`Refusal::Moved`] over
    /// (hard choice 3). Empty for an operation with nothing pinned.
    #[must_use]
    pub fn tips(&self) -> Vec<Tip> {
        match self {
            Self::Merge(params) => vec![params.into.clone()],
            Self::Rebase(params) => history::tips_of_rebase(params),
            Self::Rewrite(params) => rewrite::tips(params),
            Self::CherryPick(params) => vec![params.head.clone()],
            Self::Revert(params) => vec![params.head.clone()],
            Self::Reset(params) => match &params.target {
                ResetTarget::Head { branch, .. } => vec![branch.clone()],
                ResetTarget::Branch(tip) => vec![tip.clone()],
            },
            Self::BranchDelete(params) | Self::TagDelete(params) => vec![Tip {
                name: params.name.clone(),
                oid: params.expected,
            }],
            _ => vec![],
        }
    }

    /// What the facts must carry for this operation: the remotes for a
    /// remote one, the named refs a history one is pinned to.
    #[must_use]
    pub fn facts_scope(&self) -> FactsScope {
        let refs = match self {
            Self::Merge(params) => history::refs_of_merge(params),
            Self::Rebase(params) => history::refs_of_rebase(params),
            Self::Rewrite(params) => rewrite::tips(params)
                .into_iter()
                .map(|tip| tip.name)
                .collect(),
            Self::CherryPick(params) => vec![params.head.name.clone()],
            Self::Revert(params) => vec![params.head.name.clone()],
            Self::Reset(params) => history::refs_of_reset(params),
            Self::Checkout(params) => history::refs_of_checkout(params),
            Self::BranchCreate(params) => history::refs_of_branch_create(params),
            Self::BranchDelete(params) | Self::TagDelete(params) => vec![params.name.clone()],
            Self::TagCreate(params) => vec![history::tag_ref(&params.name)],
            // `HEAD` as well: the preview reaches the stash commit by
            // asking for what is on it and not on `HEAD`, and the branch
            // starts at the stash's base, not at `HEAD` (m5/07).
            Self::BranchFromStash(params) => {
                vec![history::branch_ref(&params.name), "HEAD".to_owned()]
            }
            // The branch being pushed: a re-push has to prove its commit is
            // on it before it will draw a force-push (ADR-0038 §4).
            Self::Push(params) => vec![params.branch.clone()],
            Self::Recover(params) => recover::refs(params),
            _ => vec![],
        };
        FactsScope {
            remotes: self.is_remote(),
            refs,
        }
    }

    /// [`Operation::describe`], with what only the facts know: a
    /// `DropPutBack` is `drop stash@{n}`, `n` the position `check` found by
    /// commit — as a `StashDrop` reads (review 72+72b m6). `apply` writes
    /// this on the journal line.
    #[must_use]
    pub fn describe_in(&self, facts: &RepoFacts) -> String {
        match self {
            Self::DropPutBack { stash, why } => match stash::position_of(*stash, facts) {
                Ok(index) if *why == PutBackWhy::SetAside => {
                    format!("drop the set-aside copy (stash@{{{index}}})")
                }
                Ok(index) => format!("drop stash@{{{index}}}"),
                // Not listed: `check` refuses before anything is written,
                // so this line is never journaled.
                Err(_) => self.describe(),
            },
            _ => self.describe(),
        }
    }

    /// The journal's and the browser's one-line text: `stage src/a.rs`,
    /// `commit "fix the thing"`, `pop stash@{0}`. A `DropPutBack` has no
    /// position without the facts, so it is named by its commit here; the
    /// journal line uses [`Operation::describe_in`].
    #[must_use]
    pub fn describe(&self) -> String {
        match self {
            Self::Stage(target) => stage::describe("stage", target),
            Self::Unstage(target) => stage::describe("unstage", target),
            Self::Discard(target) => stage::describe("discard", target),
            Self::Commit(params) => format!("commit \"{}\"", commit::subject(&params.message)),
            Self::Amend(params) => format!("amend \"{}\"", commit::subject(&params.message)),
            Self::StashPush(params) => stash::describe_push(params),
            Self::StashApply { index, .. } => format!("apply stash@{{{index}}}"),
            Self::StashPop { index, .. } => format!("pop stash@{{{index}}}"),
            Self::StashDrop { index, .. } => format!("drop stash@{{{index}}}"),
            Self::DropPutBack {
                stash,
                why: PutBackWhy::SetAside,
            } => format!("drop the set-aside copy ({})", history::short(*stash)),
            Self::DropPutBack { stash, .. } => format!("drop stash {}", history::short(*stash)),
            Self::Fetch(params) => remote::describe_fetch(params),
            Self::Pull(params) => remote::describe_pull(params),
            Self::Push(params) => remote::describe_push(params),
            Self::Merge(params) => history::describe_merge(params),
            Self::Rebase(params) => history::describe_rebase(params),
            Self::Rewrite(params) => rewrite::describe(params),
            Self::CherryPick(params) => history::describe_pick(params),
            Self::Revert(params) => history::describe_revert(params),
            Self::Reset(params) => history::describe_reset(params),
            Self::Checkout(params) => history::describe_checkout(params),
            Self::BranchCreate(params) => history::describe_branch_create(params),
            Self::BranchDelete(params) => history::describe_delete("branch", params),
            Self::TagCreate(params) => history::describe_tag_create(params),
            Self::TagDelete(params) => history::describe_delete("tag", params),
            Self::BranchFromStash(params) => history::describe_branch_from_stash(params),
            Self::ResolveWholeFile(params) => conflict::describe_resolve(params),
            Self::RestartConflict(params) => conflict::describe_restart(params),
            Self::FinishMerge(params) => finish_merge::describe(params),
            Self::AbortMerge(params) => finish_merge::describe_abort(params),
            Self::Recover(params) => recover::describe(params),
        }
    }
}
