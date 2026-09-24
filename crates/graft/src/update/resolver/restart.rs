//! `d`: start this file over — `checkout --merge -- <path>` behind a
//! confirm box (m6-conflicts/05, design note Proposed change 2c).

use graft_core::job::JobId;
use graft_term::sanitize_line;
use graft_ui::confirm::ConfirmView;

use super::reopen;
use crate::jobs::conflict::StartOver;
use crate::update::confirm::{ConfirmThen, PendingConfirm};
use crate::update::events::Command;
use crate::update::state::{AppState, Status};

/// `d`: a confirm box, `checkout --merge -- <path>` behind it — the discard
/// verb's one meaning inside the resolver (design note, Proposed change
/// 2(c)). Refused, quietly, while a read or a take is in flight: the same
/// concurrent-write worry [`super::base::toggle_base`] guards against, and confirming a
/// restart mid-write would hand the operation a file the take job is still
/// writing to.
///
/// Refused in words while any write is running: confirming would launch a
/// second git write over the first (m6-05 review, Major 3). Then refused in
/// words, before the box, wherever [`start_over_refusal`] says ops would
/// refuse — the box would promise markers ops will not bring back.
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
    let Some(open) = state.resolver.as_mut() else {
        return;
    };
    open.notice = None;
    // The box names the operation the restart folds into (a merge, a
    // rebase …). A stash-pop conflict has none open, so it makes no claim
    // about one.
    let lost = match state.in_progress.as_ref() {
        Some(detail) => format!(
            "takes and edits in this file are lost · not undoable while the {} is open",
            detail.kind.name()
        ),
        None => "takes and edits in this file are lost".to_owned(),
    };
    state.confirm = Some(PendingConfirm {
        view: ConfirmView::new(
            sanitize_line(&format!("Start {} over?", path.display())),
            vec![
                sanitize_line("the conflict comes back with all its markers"),
                sanitize_line(&lost),
            ],
            sanitize_line(&format!("git checkout --merge -- {}", path.display())),
        )
        .with_labels("Cancel", "Start over"),
        then: ConfirmThen::RestartConflict { path },
    });
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
            StartOver::Can => None,
        };
    }
    match open.done.as_ref().map(|done| &done.start_over) {
        Some(StartOver::Cannot { why }) => Some(format!("can't start over · {why}")),
        Some(StartOver::Can) | None => None,
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
    // Conflicted afresh: nothing chosen before describes it now.
    crate::update::finish::forget(state, &path);
    reopen(state, commands, path, from, next_id);
}
