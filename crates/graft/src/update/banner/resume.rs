//! Resuming a stopped operation — mergetool, edit, and the line a resume leaves.

use super::{
    AppState, Command, JobId, Operated, Path, PathBuf, Resume, Status, edit_stop_lands_on_head,
    finished_lands_on_head, land_in_resolver, live, stopped_again,
};
use crate::update::op_keys::OpKeys;
use graft_ui::keymap::{ActionId, ContextId, KeyChord, KeyCode, Keymap};

/// Queue a `--continue`/`--skip`/`--abort`.
///
/// Unstoppable, like every other history write: `Esc` killing git partway
/// through a `--continue` would leave git's state on disk with the entry
/// still `Pending` and no outcome line, which is the shape `graft_ops` does
/// not model ([`crate::update::preview::HISTORY_CANNOT_STOP`]).
pub fn resume(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    what: Resume,
    next_id: &mut impl FnMut() -> JobId,
) -> bool {
    if crate::update::operate::operation_in_flight(state) {
        return false;
    }
    let Some(kind) = live(state).map(|detail| detail.kind) else {
        return false;
    };
    let verb = match what {
        Resume::Continue | Resume::ContinueWith { .. } => "continue",
        Resume::Skip => "skip",
        Resume::Abort => "abort",
    };
    let description = format!("{verb} {}", kind.name());
    let id = next_id();
    let claimed = crate::update::operate::claim_write(state, id);
    if claimed.is_err() {
        // Unreachable: `operation_in_flight` returned above.
        crate::update::operate::say_if_refused(state, claimed);
        return true;
    }
    state.resuming = Some((id, what.clone()));
    state.status = Status::Busy(description.clone());
    commands.extend(crate::update::remote::launched(
        state,
        id,
        description,
        None,
        crate::update::remote::Stoppable::No(crate::update::preview::HISTORY_CANNOT_STOP),
        None,
    ));
    commands.push(Command::Resume(
        id,
        what,
        std::sync::Arc::clone(&state.keymap),
    ));
    true
}

/// `M`: hand the terminal to the configured mergetool for `path`.
///
/// Refused in words when there is no `merge.tool`: `git mergetool` with none
/// configured asks "which tool?" on stdin, and stdin at that moment belongs
/// to a terminal Graft has just given away, so the question would have no
/// way back to the user.
pub fn open_mergetool(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    path: Option<&Path>,
) -> bool {
    let Some(path) = path else {
        return false;
    };
    let Some(tool) = state.mergetool.as_deref() else {
        let place = crate::update::edge::context_of(state);
        state.status = Status::Sticky {
            reason: "no merge.tool configured".to_owned(),
            keys: mergetool_fallback(place, path, &state.keymap),
        };
        return true;
    };
    state.status = Status::Busy(format!("{tool} on {}", path.display()));
    commands.push(Command::OpenMergetool {
        path: path.to_path_buf(),
    });
    true
}

/// The rows a press of `e` may answer with "open this file in `$EDITOR`".
const EDIT_ROWS: [&str; 3] = ["operation.edit", "resolver.edit", "changes.edit"];

/// The rows a press of `M` may answer with "hand this file to the mergetool".
const MERGETOOL_ROWS: [&str; 3] = [
    "operation.mergetool",
    "resolver.mergetool",
    "changes.mergetool",
];

/// The row the shipped `key` acts as at `place`, when it is one of `rows`
/// (m8/10j ruling 1). Read off the shipped table, not his: the words name
/// the *action* that answers there, then spell it from his keymap — so a
/// remap moves the label, never the action the clause promises.
fn row_at(place: ContextId, key: char, rows: &[&str]) -> Option<ActionId> {
    Keymap::shipped()
        .acting(place, KeyChord::key(key))
        .filter(|id| rows.contains(&id.0))
}

/// The one place → edit-row lookup both stop lines share, so the two can
/// never name different `e` keys in one place (m8/10j ruling 4).
fn edit_row(place: ContextId) -> Option<ActionId> {
    row_at(place, 'e', &EDIT_ROWS)
}

/// Two clauses two spaces apart, either dropped; `None` when both are.
fn joined(first: Option<String>, second: Option<String>) -> Option<String> {
    match (first, second) {
        (Some(a), Some(b)) => Some(format!("{a}  {b}")),
        (one, other) => one.or(other),
    }
}

/// The fallback clause after `no merge.tool configured · `: `e edits {path}
/// a abort`, each key from the row answering it at `place` (m8/10j ruling
/// 1); a clause with no row there or an unbound key dropped; nothing left →
/// `None`. `a` is kept only where it is `operation.abort` (the banner): the
/// resolver is modal and the changes pane refuses `a` while an operation
/// is live.
pub(crate) fn mergetool_fallback(place: ContextId, path: &Path, keys: &Keymap) -> Option<String> {
    let edit =
        edit_row(place).and_then(|row| keys.key_verb(&[row], &format!("edits {}", path.display())));
    let abort =
        row_at(place, 'a', &["operation.abort"]).and_then(|row| keys.key_verb(&[row], "abort"));
    joined(edit, abort)
}

/// `e edit  M mergetool` as said at `place` (m8/10j ruling 4): `e` from
/// the edit row answering it there, `M` only where it is a mergetool row
/// (on the graph `M` is `graph.map`); a clause with no row or an unbound
/// key dropped; nothing left → `None`.
pub(crate) fn edit_or_mergetool(place: ContextId, keys: &Keymap) -> Option<String> {
    let edit = edit_row(place).and_then(|row| keys.key_verb(&[row], "edit"));
    let tool =
        row_at(place, 'M', &MERGETOOL_ROWS).and_then(|row| keys.key_verb(&[row], "mergetool"));
    joined(edit, tool)
}

/// `⏎ resolve  M mergetool  e edit` — the keys `c`'s refusal offers — as
/// said at `place` (m8/10l, on 10j ruling 1's place rule): `⏎` only where
/// it is `app.open` (on the banner it opens the resolver on the first
/// conflicted file), `M`, `e` from their rows there; a clause with no row
/// or an unbound key dropped; nothing left → `None`. The order is the
/// banner bar's, which leads with `⏎ resolve`.
pub(crate) fn resolve_keys(place: ContextId, keys: &Keymap) -> Option<String> {
    let resolve = (place == ContextId::Banner)
        .then(|| Keymap::shipped().acting(place, KeyChord::plain(KeyCode::Enter)))
        .flatten()
        .filter(|id| id.0 == "app.open")
        .and_then(|row| keys.key_verb(&[row], "resolve"));
    let tool =
        row_at(place, 'M', &MERGETOOL_ROWS).and_then(|row| keys.key_verb(&[row], "mergetool"));
    let edit = edit_row(place).and_then(|row| keys.key_verb(&[row], "edit"));
    joined(joined(resolve, tool), edit)
}

/// `e`: open `$EDITOR` on `path` at its first conflict marker.
///
/// The marker's line number is **not** looked up here. `update` is pure —
/// it reads no clock and opens no file (ARCHITECTURE.md) — and finding the
/// first `<<<<<<< ` means reading the file. So the command carries the path
/// and the loop, which is about to open the file anyway, does the read on
/// its way past ([`Command::OpenConflictEditor`]).
pub fn edit_conflict(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    path: Option<&Path>,
) -> bool {
    let Some(path) = path else {
        return false;
    };
    state.conflict_editing = Some(path.to_path_buf());
    commands.push(Command::OpenConflictEditor {
        path: path.to_path_buf(),
    });
    true
}

/// The path under the changes-pane cursor, when that file is unmerged.
///
/// The file list is git's diff output and the status entries are git's
/// porcelain output; only the path joins them, which is why this matches on
/// bytes rather than on an index into either.
#[must_use]
pub fn cursor_conflict(state: &AppState) -> Option<PathBuf> {
    use std::ffi::OsStr;

    let list = state.detail.list.as_ref()?;
    let entry = state.detail.selected_entry()?;
    let file = usize::try_from(entry)
        .ok()
        .and_then(|index| list.files.get(index))?;
    state.working_tree.as_deref().and_then(|status| {
        status
            .entries
            .iter()
            .find(|candidate| {
                candidate.unmerged
                    && OsStr::as_encoded_bytes(candidate.path.as_os_str()) == file.path.as_slice()
            })
            .map(|candidate| candidate.path.clone())
    })
}

/// A `--continue`/`--skip`/`--abort` came back.
pub fn resumed(
    state: &mut AppState,
    id: JobId,
    outcome: Operated,
    next_id: &mut impl FnMut() -> JobId,
) -> Vec<Command> {
    // One guard: the answer is the job `operating` waits for **and** the
    // resume `resuming` kept. `resume` always sets both together.
    let Some((_, how)) = state
        .resuming
        .take_if(|(resume, _)| state.operating == Some(id) && *resume == id)
    else {
        return Vec::new();
    };
    state.operating = None;
    state.remote.running = None;
    // The finish summary's own `--continue` first: a refusal keeps the
    // summary open with git's words, and nothing below runs for it.
    let finish = crate::update::finish::resume_answered(state, id, &outcome);
    let kept = finish == Some(crate::update::finish::FinishAnswer::Kept);
    let done = !kept && matches!(outcome, Operated::Done(_));
    let commands = if kept {
        kept_open(state, outcome, next_id)
    } else {
        resumed_answer(state, &how, outcome, finish, next_id)
    };
    // Last: a `q y` waiting on this resume quits only once the answer is in
    // place, and only when it ended done (m8/13b).
    crate::update::quit::write_ended(state, id, done);
    commands
}

/// [`resumed`]'s answer once the finish summary has had its say.
fn resumed_answer(
    state: &mut AppState,
    how: &Resume,
    outcome: Operated,
    finish: Option<crate::update::finish::FinishAnswer>,
    next_id: &mut impl FnMut() -> JobId,
) -> Vec<Command> {
    match outcome {
        Operated::Done(applied) => resumed_done(state, how, &applied, finish, next_id),
        Operated::StagingNotKept { applied, paths } => {
            resumed_staging_not_kept(state, how, &applied, &paths, finish, next_id)
        }
        Operated::Incomplete { applied, error } => {
            resumed_incomplete(state, how, &applied, &error, next_id)
        }
        // A rebase stopping on the *next* commit is the same entry and the
        // same banner: it stays focused, and the line says this is a second
        // stop rather than the first one all over again.
        Operated::Conflicted {
            kind,
            paths,
            stalled,
            entry,
            rewrite,
            ..
        } => {
            state.live_entries = vec![entry];
            stopped_again(state, kind, paths.len(), stalled.as_deref(), rewrite);
            let everything = graft_ops::apply::Refresh::Everything;
            let mut commands = crate::update::operate::refresh_scope(state, everything, next_id);
            land_in_resolver(state, &mut commands, &paths, next_id);
            edit_stop_lands_on_head(state, &commands);
            commands
        }
        // Over, but its set-aside clashed putting back (ADR-0045 §4, m6-conflicts/74).
        Operated::Clashed { entry, clash } => ended_in_clash(state, entry, &clash, next_id),
        Operated::Failed {
            error,
            edits: Some(at),
            refresh,
            ..
        } => edits_left_aside(state, &error, at, refresh, next_id),
        Operated::Failed {
            error,
            refresh,
            refused,
            ..
        } => {
            // The operation is still live and `a` still ends it, whatever
            // this was. `graft_ops`' own words, with `a abort` as the key
            // rather than a sentence the app invented.
            let reason = refused.map_or(error, |refusal| refusal.explain(&OpKeys(&state.keymap)));
            state.status = Status::Sticky {
                reason,
                keys: super::abort_key(&state.keymap),
            };
            match refresh {
                Some(what) => crate::update::operate::refresh_scope(state, what, next_id),
                None => Vec::new(),
            }
        }
    }
}

/// `resumed`'s answer when the rebase closed — continued or aborted — and
/// the edits set aside around it did not come back cleanly
/// (m6-conflicts/74, /84): the operation is over, and the line is its
/// failure, then where `graft_ops` says the edits are.
pub(super) fn edits_left_aside(
    state: &mut AppState,
    failure: &str,
    at: graft_ops::EditsAt,
    refresh: Option<graft_ops::apply::Refresh>,
    next_id: &mut impl FnMut() -> JobId,
) -> Vec<Command> {
    crate::update::undo::operation_ended(state);
    state.status = crate::update::operate::edits_status(failure, at, &state.keymap);
    match refresh {
        Some(what) => crate::update::operate::refresh_scope(state, what, next_id),
        None => Vec::new(),
    }
}

/// How a resume that closed its entry ended, in words: finished (the
/// renderer adds the tick) or not.
pub(super) enum Closing {
    Finished(String),
    Unfinished(String),
}

/// The one wording of a resume's close. Keys on the `Resume` sent **and**
/// the entry's status: a failed continue closes `Aborted` exactly as an
/// abort does (`graft_ops::apply::resume::close_resume`), so neither alone
/// can say which happened (m6-conflicts/68).
pub(super) fn closing(how: &Resume, entry: &graft_core::journal::JournalEntry) -> Closing {
    use graft_core::journal::EntryStatus;

    let what = &entry.description;
    match (&entry.status, how) {
        // A shell-begun merge's abort is its own `AbortMerge` entry, which
        // commits and already reads `abort merge of …`: no second word.
        (
            EntryStatus::Committed,
            Resume::Continue | Resume::ContinueWith { .. } | Resume::Abort,
        ) => Closing::Finished(what.clone()),
        // The skip finished the rebase, but which commit it left out is not
        // known here: a stop on the first of three skips the first, and the
        // two after apply cleanly (m6-conflicts/78, review Minor 1).
        (EntryStatus::Committed, Resume::Skip) => {
            Closing::Finished(format!("{what}, skipped a commit"))
        }
        // The abort the user asked for worked: it keeps its tick, and the
        // prefix makes the line true.
        (EntryStatus::Aborted { .. }, Resume::Abort) => {
            Closing::Finished(format!("aborted: {what}"))
        }
        (
            EntryStatus::Aborted { reason },
            Resume::Continue | Resume::ContinueWith { .. } | Resume::Skip,
        ) => Closing::Unfinished(format!("{what} did not finish: {reason}")),
        // Not expected on a closing answer; here so nothing says `✓` over it.
        (EntryStatus::Pending, _) => {
            Closing::Unfinished(format!("{what} did not finish: still open"))
        }
    }
}

/// What the status line says once a `--continue`/`--skip`/`--abort` closed
/// its entry. A `Status`, not a string: the tick is the renderer's, and a
/// close that did not finish must not be a `Done`. Nothing else builds
/// these words.
#[must_use]
pub(crate) fn resumed_words(how: &Resume, applied: &graft_ops::apply::Applied) -> Status {
    match closing(how, &applied.entry) {
        // A rebase that set edits aside put them back as it closed: a
        // closing answer means they came back clean, and the line says so
        // after the tick — `aborted: rebase … ✓ · your edits are back`
        // (m6-conflicts/74). Nothing after it for any other entry.
        Closing::Finished(what) => Status::Done {
            what,
            also: crate::update::clash::edits_back(applied),
            keys: None,
        },
        Closing::Unfinished(reason) => Status::sticky(reason),
    }
}

/// The operation finished: the entry ends, the scope re-reads and the
/// cursor lands on HEAD. A merge committed from the finish summary says so
/// in its own words (`merged feature into main`).
/// `resumed`'s `Incomplete`: it ran; only its record is short. The re-read
/// confirms the end the same way it does for `Done`.
pub(super) fn resumed_incomplete(
    state: &mut AppState,
    how: &Resume,
    applied: &graft_ops::apply::Applied,
    error: &str,
    next_id: &mut impl FnMut() -> JobId,
) -> Vec<Command> {
    crate::update::undo::operation_ended(state);
    let (Closing::Finished(what) | Closing::Unfinished(what)) = closing(how, &applied.entry);
    state.status = Status::sticky(format!(
        "{what} \u{2014} could not confirm it fully: {error}"
    ));
    crate::update::operate::refresh_scope(state, applied.refresh, next_id)
}

/// `resumed`'s `Clashed`: the operation is over, and its put-back clashed.
pub(super) fn ended_in_clash(
    state: &mut AppState,
    entry: graft_core::journal::EntryId,
    clash: &graft_ops::clash::Clash,
    next_id: &mut impl FnMut() -> JobId,
) -> Vec<Command> {
    crate::update::undo::operation_ended(state);
    crate::update::clash::clashed(state, None, entry, clash, next_id)
}

/// `resumed`'s `StagingNotKept`: closed as `Done` closes (banner, scope,
/// landing), and the line is the fallback's sticky ladder, led by what
/// `resumed_done` would put before its tick. A close that did not finish
/// keeps its own reason, with the staging news after it.
pub(super) fn resumed_staging_not_kept(
    state: &mut AppState,
    how: &Resume,
    applied: &graft_ops::apply::Applied,
    paths: &[PathBuf],
    finish: Option<crate::update::finish::FinishAnswer>,
    next_id: &mut impl FnMut() -> JobId,
) -> Vec<Command> {
    let commands = resumed_done(state, how, applied, finish, next_id);
    state.status = match &state.status {
        Status::Done { what, .. } => crate::update::operate::staging_not_kept_status(
            what,
            paths,
            &applied.came_back,
            state.size.columns,
            &state.keymap,
        ),
        // Anything but `Done` (an `Unfinished` sticky) is left as it is: a
        // put-back runs only on a close that finished, so no such line
        // carries staging news (review m7-30-06d Minor 7).
        other => other.clone(),
    };
    commands
}

pub(super) fn resumed_done(
    state: &mut AppState,
    how: &Resume,
    applied: &graft_ops::apply::Applied,
    finish: Option<crate::update::finish::FinishAnswer>,
    next_id: &mut impl FnMut() -> JobId,
) -> Vec<Command> {
    crate::update::undo::operation_ended(state);
    state.status = resumed_words(how, applied);
    let commands = crate::update::operate::refresh_scope(state, applied.refresh, next_id);
    finished_lands_on_head(state, &commands);
    if let Some(crate::update::finish::FinishAnswer::Merged(Some(words))) = finish {
        state.status = Status::done(words);
    }
    commands
}

/// A `--continue` the finish summary started, refused with nothing moved
/// (a hook said no): the summary already shows why, so the status line
/// stays quiet and the banner does not re-announce the stop. What might
/// have been written is re-read all the same.
pub(super) fn kept_open(
    state: &mut AppState,
    outcome: Operated,
    next_id: &mut impl FnMut() -> JobId,
) -> Vec<Command> {
    state.status = Status::Ready;
    match outcome {
        Operated::Conflicted { entry, .. } => {
            state.live_entries = vec![entry];
            crate::update::operate::refresh_scope(
                state,
                graft_ops::apply::Refresh::Everything,
                next_id,
            )
        }
        Operated::Failed {
            refresh: Some(what),
            ..
        } => crate::update::operate::refresh_scope(state, what, next_id),
        // Unreachable from the app today, kept so the match stays whole and
        // right if that changes: `resumed` comes here only when the finish
        // summary's refusal test says the answer is a refusal, and
        // `finish::answers::refusal` is `None` for every `Clashed` — the
        // summary finishes a merge, and a set-aside never wraps a merge
        // (review m6-final-fixes-74 m8). Were it reached: the put-back wrote
        // refs and files before it clashed, so everything is stale.
        Operated::Clashed { .. } => crate::update::operate::refresh_scope(
            state,
            graft_ops::apply::Refresh::Everything,
            next_id,
        ),
        Operated::Failed { refresh: None, .. }
        | Operated::Done(_)
        | Operated::StagingNotKept { .. }
        | Operated::Incomplete { .. } => Vec::new(),
    }
}
