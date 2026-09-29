//! A write that did not land, or was refused: the line it leaves.

use super::*;

/// A write that did not land: the line it leaves, and what it makes stale.
///
/// A failure is not proof the screen is still true — `ApplyError::Git` ran
/// part-way and journaled `Aborted` — so the job decides the scope, and
/// `None` is only the cases where nothing was written (m3/04 review M1, and
/// [`Operated::Failed`]'s own `refresh` field).
pub(super) fn operation_failed(
    state: &mut AppState,
    launched_op: Option<&Operation>,
    failed: Operated,
    next_id: &mut impl FnMut() -> JobId,
) -> Vec<Command> {
    let Operated::Failed {
        error,
        refresh: what,
        remote,
        refused,
        edits,
    } = failed
    else {
        // `operated` only ever hands this arm an `Operated::Failed`; the
        // match above is what guarantees it.
        return Vec::new();
    };
    // An instant switch refused for edits made since the last status read
    // lands on its preview, not on a refusal (the switch rule, 10b).
    if let (Some(op), Some(refusal)) = (launched_op, refused.as_ref())
        && let Some(commands) = crate::update::switch::instant_refused(state, op, refusal, next_id)
    {
        return commands;
    }
    let remote_op = launched_op.filter(|op| op.is_remote());
    state.status = match edits {
        Some(at) => edits_status(&error, at),
        None => failure_line(state, remote_op, remote.as_ref(), refused.as_ref(), error),
    };
    match what {
        Some(what) => refresh_scope(state, what, next_id),
        None => Vec::new(),
    }
}

/// The line for a set-aside that did not end in a clean put-back or a clash
/// (ADR-0045 §5 amended, m6-conflicts/84): sticky, the operation's own
/// failure first, then where `graft_ops` says the edits are. The clause is
/// the `keys` half, so at narrow widths the failure is cut and the clause
/// kept whole (`fit_status_line`). `S p` is named only when the edits are
/// not in the tree.
pub(crate) fn edits_status(failure: &str, at: graft_ops::EditsAt) -> Status {
    Status::Sticky {
        reason: failure.to_owned(),
        keys: Some(edits_clause(at)),
    }
}

/// Where a set-aside's edits are, in the app's words (keys included): the
/// clause of [`edits_status`], and of the relaunch notice when recovery
/// read where they were (`update::undo::interrupted_notice`, 30c). Same
/// state, same words.
pub(crate) fn edits_clause(at: graft_ops::EditsAt) -> String {
    use graft_ops::EditsAt;
    match at {
        EditsAt::Stash { n } => format!("your edits are in stash@{{{n}}} · S p puts them back"),
        EditsAt::Back { copy: Some(n) } => {
            format!("your edits are back · a copy stays in stash@{{{n}}}")
        }
        EditsAt::Back { copy: None } => "your edits are back · a copy may still be in S".to_owned(),
        EditsAt::Backup { copy: Some(n) } => {
            format!("u restores the backup · a copy is in stash@{{{n}}}")
        }
        EditsAt::Backup { copy: None } => "u restores the backup".to_owned(),
    }
}

/// The status line for a write that failed: the remote wording where the
/// failure came from a remote, the history wording where a precondition
/// refused it, and `graft_ops`' own sentence otherwise.
pub(super) fn failure_line(
    state: &mut AppState,
    remote_op: Option<&Operation>,
    remote: Option<&graft_git::remote::RemoteFailure>,
    refused: Option<&Refusal>,
    error: String,
) -> Status {
    match (remote_op, remote) {
        (Some(op), Some(failure)) => {
            crate::update::remote::remote_failed(state, op, failure, &error)
        }
        _ => history_refusal(state, refused).unwrap_or_else(|| Status::sticky(error)),
    }
}

/// The two refusals a confirmed Ghost Preview can come back with, each with
/// the key that answers it (design note, hard choices 3 and 4).
///
/// `None` for every other refusal: those already read as one sentence from
/// `Refusal::explain`, and inventing a key hint for them would be a promise
/// the app cannot keep.
///
/// The verb is the first word of `Operation::describe`, which every
/// `describe_*` in `graft_ops::operation::history` begins with (`rebase …`,
/// `merge …`, `cherry-pick …`). Taking it from there rather than writing a
/// table here is what keeps this line and the Command Log describing the
/// same operation (`KNOWN_GAPS` row 229).
pub(super) fn history_refusal(state: &AppState, refused: Option<&Refusal>) -> Option<Status> {
    let running = state.remote.running.as_ref();
    match refused? {
        // The picture was drawn against a tip that has since moved, so the
        // operation the user confirmed is not the one that would run. The
        // answer is to look again — at the same key.
        Refusal::Moved { name } => Some(Status::Sticky {
            reason: format!("{} moved since the preview", short_ref(name)),
            keys: running
                .and_then(|running| running.opened_by)
                .map(|key| format!("{key} again")),
        }),
        // Hard choice 4: refuse before git runs rather than `--autostash`,
        // whose pop can conflict outside any journal entry. The refusal
        // costs one key, and that key is named.
        // The op it launched names the verb, so these words are the ones a
        // preview refused with before the confirm (m6-conflicts/67).
        refused @ Refusal::DirtyTree { .. } => running
            .and_then(|running| running.op.as_ref())
            .map(|op| refusal_status(op, refused)),
        // Marking a half-resolved file done is the one mistake that loses
        // work quietly: the markers go into the commit. The reason names
        // the file; the keys are the two ways out of it, which is exactly
        // what the user needs next.
        Refusal::MarkersRemain { path } => Some(Status::Sticky {
            reason: format!("{} still has conflict markers", path.display()),
            keys: Some("e edit  M mergetool".to_owned()),
        }),
        _ => None,
    }
}

/// `refs/heads/main` → `main`; anything else as it stands (`HEAD`).
pub(super) fn short_ref(full: &str) -> &str {
    full.rsplit_once('/').map_or(full, |(_, last)| last)
}

/// The one wording of a refusal of an operation he asked for, whether the
/// preview refused it before the box opened or the write refused it after
/// the confirm.
///
/// `DirtyTree` names the verb and the key that answers it (hard choice 4:
/// refuse before git runs rather than `--autostash`, whose pop can conflict
/// outside any journal entry; the refusal costs one key, and that key is
/// named). Every other refusal is `Refusal::explain`'s one sentence.
pub(crate) fn refusal_status(op: &Operation, refusal: &Refusal) -> Status {
    match refusal {
        Refusal::DirtyTree { count } => Status::Sticky {
            reason: format!("{} needs a clean tree", verb_of(&op.describe())),
            keys: Some(format!(
                "S stash {count} changed {}",
                if *count == 1 { "file" } else { "files" }
            )),
        },
        other => Status::sticky(other.explain()),
    }
}

/// `rebase feature onto main` → `rebase`.
pub(super) fn verb_of(description: &str) -> &str {
    description.split_whitespace().next().unwrap_or(description)
}

/// The status line for an operation git stopped part-way through **and
/// would not carry on**: `git <kind> --continue` said why, in its own words.
///
/// Only that case is a line now. A plain conflict stop is
/// [`AppState::stopped`] and the footer words it beside the banner's keys
/// (m6-conflicts/12); git's refusal is a real problem on top of the stop,
/// and a sticky line is what keeps its words on screen until `Esc`.
pub(super) fn stalled_line(description: &str, kind: InProgress, said: &str) -> String {
    let what = kind.name();
    format!(
        "{description} \u{2014} git would not continue: {said}; the {what} is still in progress"
    )
}

/// Re-read what the write changed.
///
/// [`Refresh::Everything`] means refs moved, which is exactly the state
/// `Ctrl+R` rebuilds — so it runs `Ctrl+R`'s own path rather than a
/// hand-written subset of it. The subset was missing the walk cancellation
/// (a second history walk started while the first still ran), the repository
/// re-open and the Mistake Radar's default-branch reset, which goes stale the
/// moment a commit moves the tip (m3/04 review M8).
pub(in crate::update) fn refresh_scope(
    state: &mut AppState,
    what: Refresh,
    next_id: &mut impl FnMut() -> JobId,
) -> Vec<Command> {
    match what {
        Refresh::WorkingTree => {
            // Every write journals an entry — an aborted one too — so the
            // browser and the interrupted notice are stale the moment one
            // finishes (m3-working-tree-undo-and-stash/06). The
            // `Everything` arm gets the same read through `refresh_key`.
            let mut commands = state.journal.read(next_id);
            // A stash push, apply, pop or drop moves `refs/stash`, and the
            // overlay is what the user is looking at when they press one
            // (m3-working-tree-undo-and-stash/06).
            crate::update::stash::refresh(state, &mut commands, next_id);
            let tree = next_id();
            commands.push(crate::update::status_read::ask(state, tree));
            commands.extend(state.detail.refresh(next_id));
            commands
        }
        Refresh::Everything => {
            let mut commands = Vec::new();
            crate::update::keys::refresh_key(state, &mut commands, next_id);
            commands
        }
    }
}
