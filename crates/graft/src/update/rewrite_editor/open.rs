//! Opening the rewrite editor: from a row, from a rebase's preview, and the answer that fills it.

use super::{
    AppState, Command, JobId, Oid, OpenedFrom, RebaseParams, RewriteEditor, RewriteOpened,
    RewriteRequest, Status, after,
};
use crate::update::op_keys::OpKeys;
use graft_ui::keymap::{ActionId, Keymap};

/// The keys that open the editor again after a box beat it to the screen:
/// `R opens it` from a row (`graph.rewrite`, on the graph), `r then R
/// opens it` from a rebase's preview (`graph.rebase`, then the preview's
/// `preview.rewrite`; 10b W5). `None` when a key of it is unbound.
pub(crate) fn opens_it(from_rebase: bool, keys: &Keymap) -> Option<String> {
    let open = keys.spell_keys(&[ActionId("preview.rewrite")]);
    if from_rebase {
        let rebase = keys.spell_keys(&[ActionId("graph.rebase")])?;
        Some(format!("{rebase} then {} opens it", open?))
    } else {
        keys.key_verb(&[ActionId("graph.rewrite")], "opens it")
    }
}

/// Is the editor's `r` field open (`AppState::text_field_open`)?
pub(crate) fn field_open(state: &AppState) -> bool {
    state
        .rewrite
        .editor
        .as_ref()
        .is_some_and(|editor| editor.field.is_some())
}

// ── opening ────────────────────────────────────────────────────────────

/// `R` on the graph: read what the editor opens on, for the cursor's row
/// (`HEAD`'s on the uncommitted row). A write running or git part-way
/// through refuses in the preview's words (one wording, H4). A read already
/// out is cancelled and this one starts: the newest `R` is his intent
/// (30n Q2).
pub(crate) fn open_rewrite(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    next_id: &mut impl FnMut() -> JobId,
) {
    if state.rewrite.editor.is_some() {
        return;
    }
    if let Some(reason) = crate::update::preview::preview_refusal(state) {
        state.status = Status::Sticky { reason, keys: None };
        return;
    }
    let Some(from) = cursor_commit(state) else {
        state.status =
            Status::Waiting("still reading the refs \u{2014} try again in a moment".to_owned());
        return;
    };
    cancel_opening(state, commands);
    let id = next_id();
    state.rewrite.opening = Some(id);
    state.rewrite.opening_from = Some(OpenedFrom::Row(from));
    state.status = Status::Waiting(format!("reading {}\u{2026}", branch_name(state)));
    commands.push(Command::OpenRewrite {
        id,
        req: RewriteRequest::Open { from },
    });
}

/// `R` in a rebase preview whose suggested drops `preview::keeps_drops`
/// found (rebase-preview-keeps-drops): read the editor on that very rebase
/// — `params` as its `y` would run them — its drops marked `d`, so he sees
/// what he keeps before `y`. The preview is closed by the caller. Refused
/// as [`open_rewrite`] is, in the same words; a read already out is
/// cancelled and this one starts, as there.
pub(crate) fn open_rewrite_from_rebase(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    next_id: &mut impl FnMut() -> JobId,
    params: RebaseParams,
) {
    if state.rewrite.editor.is_some() {
        return;
    }
    if let Some(reason) = crate::update::preview::preview_refusal(state) {
        state.status = Status::Sticky { reason, keys: None };
        return;
    }
    cancel_opening(state, commands);
    let id = next_id();
    state.rewrite.opening = Some(id);
    state.rewrite.opening_from = Some(OpenedFrom::Rebase);
    state.status = Status::Waiting(format!("reading {}\u{2026}", branch_name(state)));
    commands.push(Command::OpenRewrite {
        id,
        req: RewriteRequest::FromRebase(params),
    });
}

/// The commit `R` opens on: the cursor's row, or `HEAD` on the uncommitted
/// row. `None` while the history or the refs it needs are unread.
pub(super) fn cursor_commit(state: &AppState) -> Option<Oid> {
    if state.on_working_tree {
        return state.refs.as_deref()?.head();
    }
    Some(state.history.as_ref()?.selected()?.commit.id)
}

/// His branch's short name, for `reading feature-a…`.
pub(super) fn branch_name(state: &AppState) -> String {
    state
        .refs
        .as_deref()
        .and_then(crate::update::preview::current_branch)
        .map_or_else(
            || "your branch".to_owned(),
            |tip| crate::update::preview::short_name_of(&tip.name),
        )
}

/// `Esc` while the read is out, or a newer `R`: cancel it (its answer is
/// dropped by id) and clear its line. `true` when there was one.
pub(super) fn cancel_opening(state: &mut AppState, commands: &mut Vec<Command>) -> bool {
    let Some(id) = state.rewrite.opening.take() else {
        return false;
    };
    state.rewrite.opening_from = None;
    commands.push(Command::CancelJob(id));
    state.rewrite.stale.insert(id);
    if matches!(state.status, Status::Waiting(_)) {
        state.status = Status::Ready;
    }
    true
}

/// Why an answered read did not open the editor: a box took the keys while
/// it was out (30n Q2). Hidden, not cleared, while that box owns the
/// bottom row; he reads it when he closes the box.
pub(super) const NOT_OPENED: &str = "rewrite editor not opened \u{b7} another box was open";

/// The open read answered: the overlay, a refusal on the status line, or
/// the failure. The overlay never opens over another box — a modal with
/// the keys, or another overlay (`edit_stop::no_box_is_up`): the answer is
/// dropped and the status says how to open it again.
pub(crate) fn opened(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    id: JobId,
    answer: Result<RewriteOpened, String>,
    next_id: &mut impl FnMut() -> JobId,
) {
    if state.rewrite.stale.remove(&id) || state.rewrite.opening != Some(id) {
        return;
    }
    state.rewrite.opening = None;
    let from = state.rewrite.opening_from.take();
    match answer {
        Ok(RewriteOpened::Editor(opened)) => {
            if matches!(state.status, Status::Waiting(_)) {
                state.status = Status::Ready;
            }
            let from = from.unwrap_or(OpenedFrom::Row(opened.params.head.oid));
            if !crate::update::keys::edit_stop::no_box_is_up(state) {
                let keys = opens_it(matches!(from, OpenedFrom::Rebase), &state.keymap);
                state.status = Status::Sticky {
                    reason: NOT_OPENED.to_owned(),
                    keys,
                };
                return;
            }
            let mut editor = Box::new(RewriteEditor::new(*opened, from));
            after::replan(state, &mut editor, commands, next_id);
            state.rewrite.editor = Some(editor);
        }
        Ok(RewriteOpened::Refused(refusal)) => {
            state.status = Status::sticky(refusal.explain(&OpKeys(&state.keymap)));
        }
        Err(why) => {
            state.status = Status::Failed(format!("couldn't open the rewrite editor: {why}"));
        }
    }
}

// ── routing ────────────────────────────────────────────────────────────
