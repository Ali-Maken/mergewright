//! A write that did not land, or was refused: the line it leaves.

use graft_ui::keymap::{ActionId, Keymap};

use super::{AppState, Command, InProgress, JobId, Operated, Operation, Refresh, Refusal, Status};
use crate::update::op_keys::OpKeys;

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
        Some(at) => edits_status(&error, at, &state.keymap),
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
pub(crate) fn edits_status(failure: &str, at: graft_ops::EditsAt, keys: &Keymap) -> Status {
    Status::Sticky {
        reason: failure.to_owned(),
        keys: edits_clause(at, keys),
    }
}

/// Where a set-aside's edits are, in the app's words (keys included): the
/// clause of [`edits_status`], and of the relaunch notice when recovery
/// read where they were (`update::undo::interrupted_notice`, 30c). Same
/// state, same words.
///
/// Keys spelled from his keymap (m8/10f). `S p` is a sequence: `S` from the
/// graph's row, `p` from the stash list's (10b W5); either unbound drops
/// the clause. An unbound undo drops `u restores the backup`, and a clause
/// left with nothing is `None` (W3).
pub(crate) fn edits_clause(at: graft_ops::EditsAt, keys: &Keymap) -> Option<String> {
    use graft_ops::EditsAt;
    let restores = keys.key_verb(&[ActionId("app.undo")], "restores the backup");
    let clause = match at {
        EditsAt::Stash { n } => {
            let words = format!("your edits are in stash@{{{n}}}");
            match (
                keys.spell_keys(&[ActionId("graph.stash")]),
                keys.spell_keys(&[ActionId("stash.pop")]),
            ) {
                (Some(stash), Some(pop)) => format!("{words} · {stash} {pop} puts them back"),
                _ => words,
            }
        }
        EditsAt::Back { copy: Some(n) } => {
            format!("your edits are back · a copy stays in stash@{{{n}}}")
        }
        EditsAt::Back { copy: None } => "your edits are back · a copy may still be in S".to_owned(),
        EditsAt::Backup { copy: Some(n) } => {
            let copy = format!("a copy is in stash@{{{n}}}");
            match restores {
                Some(restores) => format!("{restores} · {copy}"),
                None => copy,
            }
        }
        EditsAt::Backup { copy: None } => return restores,
    };
    Some(clause)
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
            .map(|op| refusal_status(op, refused, state.size.columns, &state.keymap)),
        // The guard read again at `y` (ADR-0049): the same line the preview
        // refuses with, its next step kept whole (m7-power-tools/30t). It
        // reads no operation, so it needs none running.
        refused @ Refusal::InTheWay { .. } => Some(in_the_way_status(
            refused,
            state.size.columns,
            &state.keymap,
        )),
        // Marking a half-resolved file done is the one mistake that loses
        // work quietly: the markers go into the commit. The reason names
        // the file; the keys are the two ways out of it, which is exactly
        // what the user needs next.
        Refusal::MarkersRemain { path } => Some(Status::Sticky {
            reason: format!("{} still has conflict markers", path.display()),
            keys: Some(
                state
                    .keymap
                    .key_verbs(&[("changes.edit", "edit"), ("changes.mergetool", "mergetool")]),
            )
            .filter(|keys| !keys.is_empty()),
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
/// named). `InTheWay` keeps its next step whole as the line's keys (with
/// `esc dismiss` beside a step that names no key, ADR-0037 §4) and picks the
/// longest reason from `Refusal::explain_rungs` that fits a status row
/// `columns` wide (ADR-0049 amendment, m7-power-tools/30t); a resize after
/// the refusal keeps the rung chosen here. Every other refusal is
/// `Refusal::explain`'s one sentence.
pub(crate) fn refusal_status(
    op: &Operation,
    refusal: &Refusal,
    columns: u16,
    keys: &Keymap,
) -> Status {
    match refusal {
        Refusal::DirtyTree { count } => Status::Sticky {
            reason: format!("{} needs a clean tree", verb_of(&op.describe())),
            keys: keys.key_verb(
                &[ActionId("graph.stash")],
                &format!(
                    "stash {count} changed {}",
                    if *count == 1 { "file" } else { "files" }
                ),
            ),
        },
        refused @ Refusal::InTheWay { .. } => in_the_way_status(refused, columns, keys),
        other => Status::sticky(other.explain(&OpKeys(keys))),
    }
}

/// [`refusal_status`]'s `InTheWay` line. "Fits" is measured as
/// `fit_status_line` measures it, for the graph footer's text, which is one
/// cell narrower than the row (it indents one); J's row is the full width,
/// so a rung that fits here fits there too.
fn in_the_way_status(refusal: &Refusal, columns: u16, keymap: &Keymap) -> Status {
    use unicode_width::UnicodeWidthStr;
    // An ignored file's next step (`move it aside first`) names no key, and
    // a footer with no key is what ADR-0037 §4 rules out; `S stashes it
    // first` names its own.
    let names_no_key = matches!(refusal, Refusal::InTheWay { ignored, .. } if !ignored.is_empty());
    let keys = |next: Option<&str>| match (next, names_no_key) {
        (Some(next), true) => format!("{next} \u{b7} {}", crate::view::status::DISMISS),
        (Some(next), false) => next.to_owned(),
        (None, _) => crate::view::status::DISMISS.to_owned(),
    };
    let room = usize::from(columns.saturating_sub(1));
    let fits = |words: &&graft_ops::operation::RefusalWords| {
        let suffix = format!(" \u{b7} {}", keys(words.next.as_deref()));
        suffix.width() < room && words.reason.width() <= room - suffix.width()
    };
    let rungs = refusal.explain_rungs(&OpKeys(keymap));
    // `explain_rungs` is never empty; the fallback is its own sentence.
    let Some(picked) = rungs.iter().find(fits).or(rungs.last()) else {
        return Status::sticky(refusal.explain(&OpKeys(keymap)));
    };
    Status::Sticky {
        reason: picked.reason.clone(),
        keys: Some(keys(picked.next.as_deref())),
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
