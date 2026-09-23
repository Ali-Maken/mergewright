//! `d`: start this file over — `checkout --merge -- <path>` behind a
//! confirm box (m6-conflicts/05, design note Proposed change 2c).

use graft_core::job::JobId;
use graft_term::sanitize_line;
use graft_ui::confirm::ConfirmView;

use super::reopen;
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
/// Refused in words, before the box, on a whole-file card — `ops` restarts
/// only a shape with markers to bring back, and the box would promise them
/// (m6-05 review, Major 1) — and while any write is running: confirming
/// would launch a second git write over the first (Major 3).
pub(super) fn restart_box(state: &mut AppState) {
    if let Some(words) = super::write_refusal(state) {
        state.status = Status::sticky(words);
        return;
    }
    let Some(open) = state.resolver.as_ref() else {
        return;
    };
    if let Some(whole) = open.whole_file.as_ref() {
        let refusal = graft_ops::operation::Refusal::NotMarkerShape {
            path: open.path.clone(),
            how: whole.how,
        };
        state.status = Status::Sticky {
            reason: refusal.explain(),
            keys: Some("o ours  t theirs".to_owned()),
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
    reopen(state, commands, path, from, next_id);
}
