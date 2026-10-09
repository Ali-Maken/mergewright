//! Refusal explanations (`Refusal::explain`).

use super::{
    KeyNames, NoPutBack, OpKey, Refusal, RefusalWords, clause, commits_them, continue_or_abort,
    dashed, history, in_the_way_rungs, in_the_way_words, name_paths, ref_words, short, then,
};

impl Refusal {
    /// One plain sentence saying what stands in the way and what to do, naming
    /// the shell command where a later milestone owns the fix. Keys are
    /// spelled through `keys`; an unbound one's clause is left out (W3).
    #[must_use]
    #[expect(
        clippy::too_many_lines,
        reason = "one arm per refusal, matched exhaustively here: splitting the remote \
                  ones out needed a catch-all that re-entered this function (m4/04 review \
                  Minor 7)"
    )]
    pub fn explain(&self, keys: &dyn KeyNames) -> String {
        match self {
            Self::Bare => "This repository has no working tree, so there is nothing here to \
                           stage, commit, discard or stash."
                .to_owned(),
            Self::InProgress(kind) => format!(
                "A {} is in progress; finish it with `{}` or abandon it with `{}` first.",
                kind.name(),
                kind.continue_command(),
                kind.abort_command()
            ),
            // ADR-0045 / m6-conflicts/72: Graft resolves them now, so the
            // words say what to do here, not in the shell.
            Self::Unmerged { paths } => match paths.split_first() {
                Some((first, rest)) if !rest.is_empty() => format!(
                    "{} +{} still has conflicts · resolve it first",
                    first.display(),
                    rest.len()
                ),
                _ => format!(
                    "{} still has conflicts · resolve it first",
                    name_paths(paths)
                ),
            },
            Self::PathNotInState { path } => format!(
                "{} has no change this can apply to; the status may have moved on, so \
                 refresh and try again.",
                path.display()
            ),
            Self::PatchPathMismatch => "The patch is for a different file than the one \
                                        selected; refresh and pick the hunk again."
                .to_owned(),
            Self::PatchSideMismatch => "The hunk comes from the other diff (a stage takes an \
                                        unstaged hunk, an unstage a staged one); refresh and \
                                        pick it again."
                .to_owned(),
            Self::NothingStaged => {
                "Nothing is staged; stage something first, or allow an empty commit.".to_owned()
            }
            Self::EmptySubject => "The commit message needs a subject line.".to_owned(),
            Self::UnbornHead => "There is no commit yet; amend and stash both need one.".to_owned(),
            Self::NothingToStash => "There are no changes to stash: untracked files count only \
                                     when they are included, and a staged-only stash needs \
                                     something staged."
                .to_owned(),
            Self::NeedsGit { feature, minimum } => {
                format!("`git {feature}` needs git {minimum} or newer; the system git is older.")
            }
            Self::NoSuchStash => "That stash is no longer in the list; look again.".to_owned(),
            Self::StashListChanged => {
                "The stash list changed since it was read; look again before choosing.".to_owned()
            }
            Self::TouchesChangedFiles { paths } => format!(
                "The stash changes {}, which {} uncommitted changes now; stage, stash or \
                 discard those first, since applying over them could conflict.",
                name_paths(paths),
                if paths.len() == 1 { "has" } else { "have" }
            ),
            Self::StashInTheWay {
                unsaved,
                staged,
                untracked,
                ignored,
            } => in_the_way_words(unsaved, staged, untracked, ignored),
            Self::StashIndexNoLongerApplies { index } => {
                format!("the staged part of stash@{{{index}}} no longer applies")
            }
            // The remote refusals, in the M4 design note's words.
            Self::DetachedHead => "HEAD is detached — check out a branch first".to_owned(),
            Self::NoRemote => {
                "no remote — add one in the shell: git remote add origin <url>".to_owned()
            }
            Self::NoSuchRemote { name } => format!("no remote named '{name}'"),
            Self::NoUpstream { branch } => dashed(
                format!("{branch} has no upstream"),
                clause(keys, OpKey::Push, "pushes and sets one"),
            ),
            Self::UpstreamChanged => dashed(
                "the branch's upstream changed since it was read".to_owned(),
                keys.label(OpKey::Pull)
                    .map(|key| format!("press {key} again")),
            ),
            Self::BranchChanged => {
                "HEAD moved to another branch since it was read — look again".to_owned()
            }
            Self::HeadUnborn => then(
                "HEAD has no commit yet".to_owned(),
                [clause(keys, OpKey::Commit, "makes the first one")],
            ),
            Self::LeaseStale { remote_side } => then(
                format!(
                    "force-push refused: {remote_side} is not where the box said \
                     (fetched since, or never fetched)"
                ),
                [clause(keys, OpKey::Fetch, "to fetch, then look")],
            ),
            // The history refusals, in the M5 design note's words.
            Self::Moved { name } => format!(
                "{} moved since the preview was drawn — press the key again",
                history::short_ref(name)
            ),
            Self::DirtyTree { count } => then(
                "needs a clean tree".to_owned(),
                [clause(
                    keys,
                    OpKey::Stash,
                    &format!(
                        "stashes {count} changed {} first",
                        if *count == 1 { "file" } else { "files" }
                    ),
                )],
            ),
            Self::BadRefName { name, rule } => format!("bad name '{name}': {rule}"),
            Self::RefExists { name } => format!("{name} exists"),
            Self::MarkersRemain { path } => format!(
                "{} still has conflict markers (<<<<<<<) — e edits, M opens the mergetool",
                path.display()
            ),
            Self::NotInProgress => {
                "nothing Graft started is in progress; an operation begun in the shell is \
                 finished there"
                    .to_owned()
            }
            Self::NotOnBranch { commit, branch } => format!(
                "{} is not on {branch}; there is no old tip to re-push",
                history::short(*commit)
            ),
            Self::MainlineOnNonMerge => {
                "-m names a parent this commit has not got; only a merge takes a mainline"
                    .to_owned()
            }
            Self::MergeNeedsMainline => {
                "reverting a merge needs a mainline: Fix… offers -m 1".to_owned()
            }
            Self::NoCommits => "nothing to cherry-pick".to_owned(),
            Self::AlreadyOnBranch { commit, branch } => {
                format!("{} is already on {branch}", history::short(*commit))
            }
            Self::PickMerge { commit } => format!(
                "{} is a merge · taking a merge isn't in Graft yet",
                history::short(*commit)
            ),
            Self::NoSuchRef { name } => format!("{} does not exist", history::short_ref(name)),
            Self::CheckedOut { branch } => {
                format!("{branch} is checked out; switch away before deleting it")
            }
            Self::NotPreviewable(kind) => {
                format!("{} has no preview", crate::apply::kind_name(*kind))
            }
            Self::CannotRevertTo(why) => format!("cannot revert to that point: {why}"),
            Self::RepushNeedsLease => {
                "re-pushing an old tip rewrites the remote branch; it runs only with a lease"
                    .to_owned()
            }
            Self::CannotSkip(kind) => then(
                format!("a {} has nothing to skip", kind.name()),
                [continue_or_abort(keys)],
            ),
            Self::MessageNotHere { kind } => then(
                format!("a {} keeps each commit's own message", kind.name()),
                [clause(keys, OpKey::Continue, "continues")],
            ),
            Self::OctopusMerge { heads, end } => format!(
                "this merges {heads} branches at once (an octopus merge); {}",
                end.in_the_shell()
            ),
            Self::AutostashWaits { end } => format!(
                "git set your changes aside for this merge (autostash); {}",
                end.in_the_shell()
            ),
            Self::NotFastForward { branch } => dashed(
                format!("{branch} has diverged; a fast-forward is not possible"),
                clause(keys, OpKey::Merge, "merges instead"),
            ),
            // The M6 conflict refusals, in the design note's words.
            Self::NotUnmerged { path } => format!("{} is not conflicted", path.display()),
            Self::ConflictsOpen { path, open } => {
                let noun = if *open == 1 { "conflict" } else { "conflicts" };
                format!("{open} {noun} still open in {}", path.display())
            }
            Self::NotMarkerShape { path, how } => format!(
                "{}: {} — no markers to take from",
                path.display(),
                how.words()
            ),
            Self::NoStagesKept { path } => {
                format!("{}: the conflict's sides were not kept", path.display())
            }
            Self::RestartChanged { path } => {
                format!(
                    "{} changed since it was opened · open it again",
                    path.display()
                )
            }
            Self::NotOnYourBranch { commit, by } => {
                let isnt = format!("{} isn't on it", short(*commit));
                match by.what_it_does(keys) {
                    Some(what) => format!("{what} · {isnt}"),
                    None => isnt,
                }
            }
            Self::MergeInRange { merge } => {
                format!("can't rewrite past a merge ({})", short(*merge))
            }
            Self::ForkedDependents { a, b } => format!(
                "{} and {} are both built on yours · Graft moves one line",
                ref_words(a),
                ref_words(b)
            ),
            Self::DependentElsewhere { branch, worktree } => format!(
                "{} is checked out in another worktree ({}) · it can't follow",
                ref_words(branch),
                worktree.display()
            ),
            Self::NothingStagedFor { target } => then(
                format!("nothing staged to put into {}", short(*target)),
                [clause(keys, OpKey::Stage, "stages")],
            ),
            Self::NotOnABranch => then(
                "you're not on a branch".to_owned(),
                [keys
                    .label(OpKey::Switch)
                    .map(|key| format!("switch first ({key})"))],
            ),
            Self::GitTooOld { need } => {
                format!("branches built on yours can follow only with git {need} or newer")
            }
            Self::PlainAmend => {
                "that's the newest commit · a plain amend puts the changes in".to_owned()
            }
            Self::RootCommit { commit } => format!(
                "{} is the first commit · there is nothing below it to replay onto",
                short(*commit)
            ),
            Self::BadTodo(problem) => problem.explain(),
            Self::NothingToAbsorb => then(
                "none of your staged lines belong to a commit on your branch".to_owned(),
                [commits_them(keys)],
            ),
            Self::AllPushed { branch } => then(
                format!(
                    "every commit on {branch} is already pushed · absorb only changes unpushed \
                     commits"
                ),
                [commits_them(keys)],
            ),
            Self::NoCommitsOfItsOwn { branch } => then(
                format!("{branch} has no commits of its own yet"),
                [commits_them(keys)],
            ),
            Self::AlreadyPushed { commit } => then(
                format!("{} is already pushed", short(*commit)),
                [clause(
                    keys,
                    OpKey::Amend,
                    "only changes older commits you haven't pushed",
                )],
            ),
            Self::BeyondReach { commit, branch } => then(
                format!(
                    "{} is more than {} commits back",
                    short(*commit),
                    crate::rewrite::ABSORB_FALLBACK_DEPTH,
                ),
                [keys.label(OpKey::Amend).map(|key| {
                    format!(
                        "on {branch} {key} reaches the last {} unpushed commits",
                        crate::rewrite::ABSORB_FALLBACK_DEPTH
                    )
                })],
            ),
            Self::MovedOnSince { name, now } => {
                format!("{name} moved on since (now {})", short(*now))
            }
            Self::HeldAgain { commit, by } => {
                format!("{} is on {by} again · nothing to put back", short(*commit))
            }
            Self::NoPutBack(NoPutBack::Graft) => {
                "done in Graft · revert to before it instead".to_owned()
            }
            Self::NoPutBack(NoPutBack::Switch { back_to }) => {
                format!("a switch moves no branch · switch back to {back_to} instead")
            }
            Self::NoPutBack(NoPutBack::Created { name }) => {
                format!("this step made {name} · there was nothing before it to go back to")
            }
            Self::NoPutBack(NoPutBack::Nothing) => "nothing here to put back".to_owned(),
            Self::InTheWay {
                doing,
                ignored,
                untracked,
            } => {
                let words = in_the_way_rungs(doing, ignored, untracked, keys).swap_remove(0);
                match words.next {
                    Some(next) => format!("{} · {next}", words.reason),
                    None => words.reason,
                }
            }
        }
    }

    /// Every wording of this refusal, longest first. `[0]` joined as
    /// `reason · next` is exactly [`Refusal::explain`]. Every variant but
    /// `InTheWay` has one rung, with no `next`. `InTheWay` has ADR-0049's
    /// ladder (amendment, m7-power-tools/30t): `of <doing>` goes first, then
    /// the path's middle (`<top>/…/<name>`), then all but its name
    /// (`…/<name>`); `next` is the same on every rung, so a caller can keep
    /// it whole as the line's keys and pick the reason that fits.
    #[must_use]
    pub fn explain_rungs(&self, keys: &dyn KeyNames) -> Vec<RefusalWords> {
        match self {
            Self::InTheWay {
                doing,
                ignored,
                untracked,
            } => in_the_way_rungs(doing, ignored, untracked, keys),
            other => vec![RefusalWords {
                reason: other.explain(keys),
                next: None,
            }],
        }
    }
}
