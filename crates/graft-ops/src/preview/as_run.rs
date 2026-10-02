//! The operation as `y` will run it: a rebase completed once, then planned.

#[allow(clippy::wildcard_imports)]
use super::*;

/// The operation as `y` will run it, and its picture: the preview job's one
/// entry point for an [`Operation`] (05b).
///
/// A rebase (`r`) is completed here, once, and never again at `y`:
/// 1. its line of branches (`rewrite::line_of`, `R`'s own read):
///    a fork, a branch of it checked out in another worktree, or git older
///    than 2.38 with branches following is [`PreviewError::Refused`], in
///    `R`'s words and order;
/// 2. [`suggest_drops`] over that line;
/// 3. `line` written, and `drops` = the suggestion's commits git would
///    replay (one git already leaves out as "already upstream" has no row
///    to mark, so it is not written);
/// 4. [`plan`] over the completed operation, with `already_in` from step 2.
///
/// Every other operation comes back unchanged, with [`plan`]'s picture.
///
/// The default branch is git's guess (`None`), as `apply::Context::open`
/// passes today (02b's open seam).
///
/// # Errors
/// As [`plan`]; for a rebase also the line's refusals above.
pub fn plan_as_run(
    repo: &RepoHandle,
    git: &GitVersion,
    op: Operation,
    facts: &RepoFacts,
    stop: &dyn Stop,
) -> Result<(Operation, PreviewPlan), PreviewError> {
    if !matches!(op, Operation::Rebase(_)) {
        let drawn = plan(repo, git, &op, facts, stop)?;
        return Ok((op, drawn));
    }
    if stop.should_stop() {
        return Err(PreviewError::Stopped);
    }
    // `plan`'s own gate first, so a branch switched or moved since the key
    // says so before the line is read.
    checked(&op, facts)?;
    let Operation::Rebase(mut params) = op else {
        // Matched above: only a rebase reaches here.
        return Err(PreviewError::Refused(Refusal::NotPreviewable(op.kind())));
    };
    let line =
        crate::rewrite::line_of(repo, git, &params.branch, None, stop).map_err(
            |why| match why {
                crate::rewrite::RewriteError::Refused(refusal) => PreviewError::Refused(refusal),
                crate::rewrite::RewriteError::Facts(error) => PreviewError::Facts(error),
                crate::rewrite::RewriteError::Stopped => PreviewError::Stopped,
            },
        )?;
    let suggestion = suggest_drops(repo, git, &line, params.onto_tip, stop).map_err(facts_error)?;
    params.drops = match &suggestion {
        Some(DropSuggestion::Drop(already_in)) => {
            let rows = crate::rewrite::rebase_rows(repo, params.onto_tip, params.branch.oid, stop)
                .map_err(facts_error)?;
            already_in
                .commits
                .iter()
                .copied()
                .filter(|commit| rows.iter().any(|row| row.commit == *commit))
                .collect()
        }
        Some(DropSuggestion::NotChecked(_)) | None => Vec::new(),
    };
    // The facts `check` holds the completed rebase to: the tips of the line
    // just read, which is what the picture is drawn against.
    let mut facts = facts.clone();
    for tip in line.dependents() {
        facts.refs.insert(tip.name.clone(), Some(tip.oid));
    }
    params.line = line;
    let op = Operation::Rebase(params);
    let mut drawn = plan(repo, git, &op, &facts, stop)?;
    drawn.already_in = suggestion;
    Ok((op, drawn))
}

/// The same gate `apply` runs before it writes: a picture of a write that
/// would be refused is not a preview (m6-conflicts/67).
pub(super) fn checked(op: &Operation, facts: &RepoFacts) -> Result<(), PreviewError> {
    op.check(facts).map_err(|refused| match (op, refused) {
        // A stash put-back refused over changed files says which of them
        // git itself would refuse over, in the stash preview's words
        // (m6-conflicts/86). Refused either way; only the words differ.
        (
            Operation::StashApply { commit, .. } | Operation::StashPop { commit, .. },
            Refusal::TouchesChangedFiles { paths },
        ) => PreviewError::Refused(stash::in_the_way(*commit, facts, paths)),
        (_, refused) => PreviewError::Refused(refused),
    })
}

pub(super) fn facts_error(error: FactsError) -> PreviewError {
    match error {
        FactsError::Stopped => PreviewError::Stopped,
        other => PreviewError::Facts(other),
    }
}

/// [`crate::in_the_way::blocking`], the read `apply` repeats at `y`, as the
/// preview's refusal.
pub(super) fn refuse_in_the_way(
    repo: &RepoHandle,
    op: &Operation,
    facts: &RepoFacts,
    stop: &dyn Stop,
) -> Result<(), PreviewError> {
    let blocking = crate::in_the_way::blocking(repo, op, facts, stop).map_err(|why| match why {
        FactsError::Stopped | FactsError::Snapshot(crate::snapshot::SnapshotError::Stopped) => {
            PreviewError::Stopped
        }
        other => PreviewError::Facts(other),
    })?;
    match crate::in_the_way::refused(op, blocking) {
        Some(refused) => Err(PreviewError::Refused(refused)),
        None => Ok(()),
    }
}

/// Fold another commit's forecast into the one so far.
///
/// `NotChecked` wins over everything — a forecast with a hole in it is not
/// a clean bill (PROMPT.md §1.4) — then `Likely`, whose paths accumulate in
/// the order git named them, without repeats.
pub(super) fn fold(sofar: ConflictForecast, next: ConflictForecast) -> ConflictForecast {
    match (sofar, next) {
        (ConflictForecast::NotChecked(why), _) | (_, ConflictForecast::NotChecked(why)) => {
            ConflictForecast::NotChecked(why)
        }
        // Only a chained replay stops at a step, and it is never folded; if
        // one ever is, the first stop is the one git reaches.
        (stop @ ConflictForecast::StopsAt { .. }, _)
        | (_, stop @ ConflictForecast::StopsAt { .. }) => stop,
        (ConflictForecast::Likely { mut paths }, ConflictForecast::Likely { paths: more }) => {
            for path in more {
                if !paths.contains(&path) {
                    paths.push(path);
                }
            }
            ConflictForecast::Likely { paths }
        }
        (ConflictForecast::Likely { paths }, ConflictForecast::Clean)
        | (ConflictForecast::Clean, ConflictForecast::Likely { paths }) => {
            ConflictForecast::Likely { paths }
        }
        (ConflictForecast::Clean, ConflictForecast::Clean) => ConflictForecast::Clean,
    }
}
