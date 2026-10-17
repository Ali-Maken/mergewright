//! `Operation` check, plan, preview, touches.

use super::{
    Invocation, Operation, PathBuf, Refusal, RemoteNames, RepoFacts, Target, Touches,
    amends_at_edit_stop, commit, conflict, finish_merge, history, recover, remote, rewrite, stage,
    stash,
};

impl Operation {
    /// The two gates every operation passes before its own rules: unmerged
    /// paths, and an operation git already has in progress.
    ///
    /// A stage or unstage of exactly the unmerged paths is "mark resolved"
    /// (m5/07), and the two conflict operations *are* the resolution, so
    /// their own `check` is the one that refuses. A stage of a path that
    /// still carries markers is `MarkersRemain`. Everything else waits for
    /// the operation in progress to finish — except an amend at a rebase's
    /// `edit` stop, on the stopped commit (P14's exception, 07a).
    fn check_conflict_state(&self, facts: &RepoFacts) -> Result<(), Refusal> {
        let unmerged: Vec<PathBuf> = facts
            .status
            .entries
            .iter()
            .filter(|entry| entry.unmerged)
            .map(|entry| entry.path.clone())
            .collect();
        if !unmerged.is_empty() {
            let resolving = match self {
                Self::Stage(Target::Paths(paths)) | Self::Unstage(Target::Paths(paths)) => {
                    paths.iter().all(|path| unmerged.contains(path))
                }
                // The two conflict operations are the resolution; aborting
                // is one of the merge's two ends, and a merge stopped on a
                // conflict is exactly what it ends.
                Self::ResolveWholeFile(_) | Self::RestartConflict(_) | Self::AbortMerge(_) => true,
                _ => false,
            };
            if !resolving {
                return Err(Refusal::Unmerged { paths: unmerged });
            }
            if let Self::Stage(Target::Paths(paths)) = self {
                history::markers_in(paths, facts)?;
            }
        }
        // Finishing or aborting the merge is what the in-progress state
        // waits *for*.
        let waits = !matches!(
            self,
            Self::Stage(_)
                | Self::Unstage(_)
                | Self::ResolveWholeFile(_)
                | Self::RestartConflict(_)
                | Self::FinishMerge(_)
                | Self::AbortMerge(_)
        );
        match facts
            .in_progress
            .filter(|_| waits && !amends_at_edit_stop(self, facts))
        {
            Some(kind) => Err(Refusal::InProgress(kind)),
            None => Ok(()),
        }
    }

    /// Property 1: the preconditions. `Ok` means `plan` will produce
    /// invocations git is expected to accept.
    ///
    /// # Errors
    /// The first [`Refusal`] found, in this order: a bare repository, unmerged
    /// paths (a stage or unstage of exactly those paths is exempt — that is
    /// "mark resolved", m5/07 — and a stage of one that still has conflict
    /// markers is `MarkersRemain`), an operation in progress (stage and
    /// unstage are allowed mid-rebase, and an amend at an `edit` stop on the
    /// stopped commit; everything else waits), then the
    /// kind's own rules. A fetch skips the first three: it changes no
    /// branch, index or file, so it runs in a bare repository and in the
    /// middle of a rebase.
    pub fn check(&self, facts: &RepoFacts) -> Result<(), Refusal> {
        if let Self::Fetch(params) = self {
            return remote::check_fetch(params, facts);
        }
        if facts.bare {
            return Err(Refusal::Bare);
        }
        self.check_conflict_state(facts)?;
        match self {
            Self::Stage(target) => stage::check(target, &facts.status, stage::State::Stageable),
            Self::Unstage(target) => stage::check(target, &facts.status, stage::State::Staged),
            Self::Discard(target) => stage::check(target, &facts.status, stage::State::Discardable),
            Self::Commit(params) => commit::check_commit(params, &facts.status),
            Self::Amend(params) => commit::check_amend(params, &facts.head),
            Self::StashPush(params) => stash::check_push(params, facts),
            Self::StashApply { index, commit } | Self::StashPop { index, commit } => {
                stash::check_apply(*index, *commit, facts)
            }
            Self::StashDrop { index, commit } => stash::find(*index, *commit, facts).map(|_| ()),
            Self::DropPutBack { stash, .. } => stash::position_of(*stash, facts).map(|_| ()),
            Self::Fetch(params) => remote::check_fetch(params, facts),
            Self::Pull(params) => remote::check_pull(params, facts),
            Self::Push(params) => remote::check_push(params, facts),
            Self::Merge(params) => history::check_merge(params, facts),
            Self::Rebase(params) => history::check_rebase(params, facts),
            Self::Rewrite(params) => rewrite::check(params, facts),
            Self::CherryPick(params) => history::check_pick(params, facts),
            Self::Revert(params) => history::check_revert(params, facts),
            Self::Reset(params) => history::check_reset(params, facts),
            Self::Checkout(params) => history::check_checkout(params, facts),
            Self::BranchCreate(params) => history::check_branch_create(params, facts),
            Self::BranchDelete(params) => history::check_branch_delete(params, facts),
            Self::TagCreate(params) => history::check_tag_create(params, facts),
            Self::TagDelete(params) => history::check_delete(params, facts),
            Self::BranchFromStash(params) => history::check_branch_from_stash(params, facts),
            Self::ResolveWholeFile(params) => conflict::check_resolve(params, facts),
            Self::RestartConflict(params) => conflict::check_restart(&params.path, facts),
            Self::FinishMerge(_) | Self::AbortMerge(_) => finish_merge::check(facts),
            Self::Recover(params) => recover::check(params, facts),
        }
    }

    /// Property 2: the typed invocations, in the order they run. Exact
    /// arguments are the rules in the group spec; each is verified against
    /// real git in a throwaway repository before it was written down.
    #[must_use]
    pub fn plan(&self, facts: &RepoFacts) -> Vec<Invocation> {
        match self {
            Self::Stage(target) => stage::plan_stage(target, &facts.status),
            Self::Unstage(target) => stage::plan_unstage(target, facts),
            Self::Discard(target) => stage::plan_discard(target, facts),
            Self::Commit(params) => commit::plan(params, false, facts),
            Self::Amend(params) => commit::plan(params, true, facts),
            Self::StashPush(params) => vec![stash::plan_push(params)],
            Self::StashApply { index, .. } => vec![stash::plan_at("apply", *index, true)],
            Self::StashPop { index, .. } => vec![stash::plan_at("pop", *index, true)],
            Self::StashDrop { index, .. } => vec![stash::plan_at("drop", *index, false)],
            Self::DropPutBack { stash, .. } => vec![stash::plan_drop_by_commit(*stash, facts)],
            Self::Fetch(params) => remote::plan_fetch(params),
            Self::Pull(params) => remote::plan_pull(params),
            Self::Push(params) => remote::plan_push(params),
            Self::Merge(params) => history::plan_merge(params),
            Self::Rebase(params) => history::plan_rebase(params, facts.git),
            Self::Rewrite(params) => rewrite::plan(params, facts.git),
            Self::CherryPick(params) => history::plan_pick(params),
            Self::Revert(params) => history::plan_revert(params),
            Self::Reset(params) => history::plan_reset(params),
            Self::Checkout(params) => history::plan_checkout(params),
            Self::BranchCreate(params) => history::plan_branch_create(params),
            Self::BranchDelete(params) | Self::TagDelete(params) => history::plan_delete(params),
            Self::TagCreate(params) => history::plan_tag_create(params),
            Self::BranchFromStash(params) => history::plan_branch_from_stash(params),
            Self::ResolveWholeFile(params) => conflict::plan_resolve(params),
            Self::RestartConflict(params) => conflict::plan_restart(params),
            Self::FinishMerge(params) => finish_merge::plan(params),
            Self::AbortMerge(_) => finish_merge::plan_abort(),
            Self::Recover(params) => recover::plan(params, facts),
        }
    }

    /// Fetch, pull or push: an operation that talks to a remote. The
    /// executor re-reads everything after one (`Refresh::Everything`).
    #[must_use]
    pub fn is_remote(&self) -> bool {
        matches!(self, Self::Fetch(_) | Self::Pull(_) | Self::Push(_))
    }

    /// The branch and remote names a remote operation's failure line uses,
    /// from its params; `None` for a local operation.
    #[must_use]
    pub fn remote_names(&self) -> Option<RemoteNames> {
        match self {
            Self::Fetch(params) => Some(remote::names_of_fetch(params)),
            Self::Pull(params) => Some(remote::names_of_pull(params)),
            Self::Push(params) => Some(remote::names_of_push(params)),
            _ => None,
        }
    }

    /// The command lines a remote operation will run, known before any
    /// facts are read — the push box shows them. Equal to `plan`'s lines,
    /// because a remote operation's plan reads nothing from the facts.
    /// `None` for a local operation, whose plan depends on the status.
    #[must_use]
    pub fn preview(&self) -> Option<Vec<String>> {
        let plan = match self {
            Self::Fetch(params) => remote::plan_fetch(params),
            Self::Pull(params) => remote::plan_pull(params),
            Self::Push(params) => remote::plan_push(params),
            _ => return None,
        };
        Some(plan.iter().map(Invocation::command_line).collect())
    }

    /// Property 3: what the operation changes — the snapshot's scope and the
    /// generic inverse's steps. Refs are full names; a detached `HEAD` is
    /// `HEAD` itself.
    #[must_use]
    pub fn touches(&self, facts: &RepoFacts) -> Touches {
        let (refs, index, worktree) = match self {
            Self::Stage(_) | Self::Unstage(_) => (vec![], true, false),
            // A discard, and the two conflict operations: the index (three
            // stages collapsing to one entry, or going) and the working tree
            // (a side written out, or the file removed). No ref moves for
            // either conflict operation — the operation the conflict belongs
            // to is what moves one, when it finishes.
            Self::Discard(_) | Self::ResolveWholeFile(_) | Self::RestartConflict(_) => {
                (vec![], true, true)
            }
            Self::Commit(_) | Self::Amend(_) => {
                (vec![commit::head_ref_name(&facts.head)], true, false)
            }
            Self::StashPush(_) | Self::StashApply { .. } | Self::StashPop { .. } => {
                (vec![stash::STASH_REF.to_owned()], true, true)
            }
            Self::StashDrop { .. } | Self::DropPutBack { .. } => {
                (vec![stash::STASH_REF.to_owned()], false, false)
            }
            // Tracking refs are not recorded: a fetch can move thousands.
            Self::Fetch(_) => (vec![], false, false),
            // For the browser's `origin/main a1b2c3d → 9f8e7d6`, not for undo.
            Self::Push(params) => (remote::push_tracking_ref(params, facts), false, false),
            // The branch (or a detached `HEAD`), the index and the tree.
            // Finishing a shell-begun merge commits the resolved index, so `u` has the
            // resolved tree to put back. Aborting one moves no ref, but the
            // branch is recorded all the same: a commit made on it since is
            // drift, and putting the merge back onto another `HEAD` would
            // be a merge nobody began.
            Self::Revert(_) | Self::FinishMerge(_) | Self::AbortMerge(_) => {
                (vec![commit::head_ref_name(&facts.head)], true, true)
            }
            // A set-aside records `refs/stash` too: its before and after
            // say whether `S` was left in the list, which is what the
            // inverse's stash step reads (ADR-0045 §4 step 2, §6).
            //
            // A rebase's branches built on his join these in `apply`, which
            // finds them when it runs (`apply::engine`). A cherry-pick sets
            // aside as a rebase does (m7-power-tools/11a), and so do a pull
            // (m7-power-tools/03b) and a merge (m8-polish-and-ship/19d). A
            // pull never records the tracking ref: undo must not move it
            // back, since the remote still has those commits.
            Self::Rebase(_) | Self::CherryPick(_) | Self::Pull(_) | Self::Merge(_) => {
                let mut refs = vec![commit::head_ref_name(&facts.head)];
                refs.extend(self.stash_ref_if_set_aside(facts));
                (refs, true, true)
            }
            // Every branch of the line, his first (ADR-0047 §2).
            Self::Rewrite(params) => {
                let mut refs: Vec<String> = rewrite::tips(params)
                    .into_iter()
                    .map(|tip| tip.name)
                    .collect();
                refs.extend(self.stash_ref_if_set_aside(facts));
                (refs, true, true)
            }
            Self::Reset(params) => return history::touches_reset(params),
            Self::Checkout(params) => {
                return self.and_stash_if_set_aside(history::touches_checkout(params), facts);
            }
            Self::BranchCreate(params) => {
                return self.and_stash_if_set_aside(history::touches_branch_create(params), facts);
            }
            Self::BranchDelete(params) | Self::TagDelete(params) => {
                (vec![params.name.clone()], false, false)
            }
            Self::TagCreate(params) => (vec![history::tag_ref(&params.name)], false, false),
            Self::BranchFromStash(params) => return history::touches_branch_from_stash(params),
            Self::Recover(params) => {
                let mut touches = recover::touches(params, facts);
                touches.refs.extend(self.stash_ref_if_set_aside(facts));
                return touches;
            }
        };
        Touches {
            refs,
            index,
            worktree,
        }
    }
}
