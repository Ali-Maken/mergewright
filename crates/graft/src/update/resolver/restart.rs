//! `d`: start this file over — `checkout --merge -- <path>` behind a
//! confirm box (m6-conflicts/05, design note Proposed change 2c); for a
//! done whole-file choice, `checkout --ours|--theirs -- <path>` and the
//! keep-or-delete choice comes back (m6-conflicts/58).

use graft_core::job::JobId;
use graft_ops::operation::RestartKind;
use graft_term::sanitize_line;
use graft_ui::confirm::ConfirmView;

use super::reopen;
use crate::jobs::conflict::StartOver;
use crate::update::confirm::{ConfirmThen, PendingConfirm};
use crate::update::events::Command;
use crate::update::state::{AppState, Status};

/// `d`: a confirm box with the start-over behind it — `checkout --merge --
/// <path>` for a file with markers, `checkout --ours|--theirs -- <path>` for
/// a done whole-file choice (m6-conflicts/58) — the discard verb's one
/// meaning inside the resolver (design note, Proposed change 2(c)). Refused, quietly, while a read or a take is in flight: the same
/// concurrent-write worry [`super::base::toggle_base`] guards against, and confirming a
/// restart mid-write would hand the operation a file the take job is still
/// writing to.
///
/// Refused in words while any write is running: confirming would launch a
/// second git write over the first (m6-05 review, Major 3). Then refused in
/// words, before the box, wherever [`start_over_refusal`] says ops would
/// refuse — the box would promise what ops will not bring back: the
/// markers, or for a whole-file choice the keep-or-delete choice.
pub(super) fn restart_box(state: &mut AppState) {
    if let Some(words) = super::write_refusal(state) {
        state.status = Status::sticky(words);
        return;
    }
    let Some(open) = state.resolver.as_ref() else {
        return;
    };
    if let Some(reason) = start_over_refusal(state) {
        // A whole-file card's way out is its own `o`/`t`, so the line
        // offers them; a done file's bar already says what is left.
        let card = open.whole_file.is_some();
        state.status = Status::Sticky {
            reason,
            keys: card.then(|| super::names::side_keys(state)),
        };
        return;
    }
    let path = open.path.clone();
    let done = open.done.is_some();
    let kind = start_over_kind(state);
    let Some(open) = state.resolver.as_mut() else {
        return;
    };
    open.notice = None;
    let (comes_back, lost) = match kind {
        RestartKind::Markers => (
            "the conflict comes back with all its markers",
            "takes and edits in this file are lost",
        ),
        RestartKind::WholeFile { .. } => (
            "the choice comes back: keep or delete",
            "edits since the choice are lost",
        ),
    };
    // A file marked done drops the choices behind its `⏎`
    // (m7-power-tools/17's words); before `⏎` it is takes and edits.
    let dropped = format!("your choices in {} are dropped", path.display());
    let lost = if done { dropped.as_str() } else { lost };
    // The box names the operation the restart folds into (a merge, a
    // rebase …). A stash-pop conflict has none open, so it makes no claim
    // about one.
    let lost = match state.in_progress.as_ref() {
        Some(detail) => format!(
            "{lost} · not undoable while the {} is open",
            detail.kind.name()
        ),
        None => lost.to_owned(),
    };
    state.confirm = Some(PendingConfirm {
        view: ConfirmView::new(
            // Done: `Start <file> over` (m7-power-tools/17); before `⏎`
            // the m6 title keeps its `?`.
            sanitize_line(&format!(
                "Start {} over{}",
                path.display(),
                if done { "" } else { "?" }
            )),
            vec![sanitize_line(comes_back), sanitize_line(&lost)],
            // The flag the plan runs: one source (`RestartKind::checkout_flag`).
            sanitize_line(&format!(
                "git checkout {} -- {}",
                kind.checkout_flag(),
                path.display()
            )),
        )
        .with_labels("Cancel", "Start over"),
        then: ConfirmThen::RestartConflict { path, kind },
    });
}

/// What ops said `d` puts back, where `d` opens the box.
///
/// A whole-file card or a done file carries ops' answer from when it opened.
/// A file open with its blocks (unmerged, markers) carries none: ops asks
/// its stages straight from the index there, which is always `Markers`
/// (`restart_source`'s `Index` arm) — and a stale answer is ops'
/// `RestartChanged` to refuse, not the app's to guess.
fn start_over_kind(state: &AppState) -> RestartKind {
    let Some(open) = state.resolver.as_ref() else {
        return RestartKind::Markers;
    };
    let answer = match (open.whole_file.as_ref(), open.done.as_ref()) {
        (Some(whole), _) => Some(&whole.start_over),
        (None, Some(done)) => Some(&done.start_over),
        (None, None) => None,
    };
    match answer {
        Some(StartOver::Can { kind }) => *kind,
        Some(StartOver::Cannot { .. }) | None => RestartKind::Markers,
    }
}

/// **The one predicate for "does `d` open the box"**: `None` exactly when it
/// does, else the words it answers with instead. Read by [`restart_box`],
/// by the bar (`view::rows::resolver_context`), and by the done file's
/// identity row, card and refusal words — none keeps a copy of the rule
/// (m6-conflicts/28).
///
/// Structural only: [`super::write_refusal`] (a write in flight) stays
/// ahead of it in `restart_box`, and is not a reason to hide `d`.
///
/// - A whole-file card: ops' own answer, asked when it opened — a
///   one-sided shape is `Refusal::NotMarkerShape`; a binary `UU`/`AA` is
///   whatever ops says (ticket 30: no app copy of the rule).
/// - A done file: ops' own answer, asked when it opened
///   (`jobs::conflict::StartOver`).
pub(crate) fn start_over_refusal(state: &AppState) -> Option<String> {
    let open = state.resolver.as_ref()?;
    if let Some(whole) = open.whole_file.as_ref() {
        // Ops' answer, raw: its refusal already names the file and shape.
        return match &whole.start_over {
            StartOver::Cannot { why } => Some(why.clone()),
            StartOver::Can { .. } => None,
        };
    }
    match open.done.as_ref().map(|done| &done.start_over) {
        // m7-power-tools/17's words. Ops' reason already leads with the
        // path; the words name it once.
        Some(StartOver::Cannot { why }) => {
            let path = open.path.display().to_string();
            let reason = why
                .strip_prefix(&format!("{path}: "))
                .unwrap_or(why.as_str());
            Some(format!(
                "git can't bring back the conflict in {path}: {reason}"
            ))
        }
        Some(StartOver::Can { .. }) | None => None,
    }
}

/// The `RestartConflict` `d` launched landed: reopen the same file fresh,
/// markers and all — a `checkout --merge` write, unlike `⏎`'s `Stage`, never
/// advances to another file.
pub fn restart_landed(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    id: JobId,
    next_id: &mut impl FnMut() -> JobId,
) {
    let Some(open) = state.resolver.as_ref() else {
        return;
    };
    if open.restarting != Some(id) {
        return;
    }
    let path = open.path.clone();
    let from = open.opened_from;
    // Landed: see `resolved_advance`, which clears its id the same way.
    if let Some(open) = state.resolver.as_mut() {
        open.restarting = None;
    }
    // Conflicted afresh: nothing chosen before describes it now — its picks
    // included, dropped here so the re-read does not word his own start
    // over as `changed since your picks` (ticket ease-pass/05r).
    crate::update::finish::forget(state, &path);
    state.picks.seal(&path);
    reopen(state, commands, path, from, next_id);
}
