//! The preview's keys: confirm, cancel, the keeps/drops toggle, `Enter` on a choice.

use super::{
    Again, AppState, Command, DropSuggestion, HistoryKey, JobId, KeyCode, KeyPress, Modifiers,
    Operation, PREVIEW_MIN, PreviewBox, PreviewPlan, PreviewState, PreviewSubject, RebaseParams,
    close_preview, confirm_preview, current_branch, is_history_key, preview_box, stash_put_back,
    stash_verb,
};

/// The preview's keys, while it is open.
///
/// Modal: **every** key but `Ctrl+C` is claimed, so nothing else — `u`
/// least of all — acts over a picture the user is still reading. `←`/`→`
/// and `Tab`/`BackTab` move between the choices, clamped and without
/// wrapping (ADR-0038 §1: walking left off `Cancel` must not arrive at
/// `Confirm`); `Enter` acts on the choice under the cursor; `Esc` closes,
/// cancelling a plan still in flight. Everything else is swallowed.
///
/// `true` whenever a preview was open.
pub fn preview_key(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    key: KeyPress,
    next_id: &mut impl FnMut() -> JobId,
) -> bool {
    if state.preview.is_none() {
        return false;
    }
    // `Ctrl+C` quits from anywhere, including here. It is the one key no
    // modal in this app claims.
    if key.code == KeyCode::Char('c') && key.modifiers == Modifiers::CONTROL {
        return false;
    }
    // Counted before the borrow, since it reads the whole state; the `else`
    // then covers the same case the check above did, without a panic
    // standing between the two (review 08 Minor 3).
    let choices = choice_count(state);
    let Some(preview) = state.preview.as_mut() else {
        return false;
    };
    if let PreviewSubject::Operation { opened_by, .. } = &preview.subject
        && confirm_key(key, opened_by)
    {
        confirm_by_key(state, commands, next_id);
        return true;
    }
    if key.code == KeyCode::Char('R') && !key.modifiers.control && !key.modifiers.alt {
        // `R` keeps a suggested drop: the rewrite editor on this very
        // rebase, its drops marked. Live only where `keeps_drops` says;
        // anywhere else swallowed like every other key.
        if let Some(params) = keeps_drops_here(state).cloned() {
            close_preview(state, commands);
            crate::update::rewrite_editor::open_rewrite_from_rebase(
                state, commands, next_id, params,
            );
        }
        return true;
    }
    match key.code {
        KeyCode::Left => preview.cursor = preview.cursor.saturating_sub(1),
        KeyCode::Right | KeyCode::Tab | KeyCode::BackTab => {
            preview.cursor = preview
                .cursor
                .saturating_add(1)
                .min(choices.saturating_sub(1));
        }
        KeyCode::Enter => enter(state, commands, next_id),
        KeyCode::Escape => close_preview(state, commands),
        // Swallowed: the picture keeps the keyboard until it is answered.
        _ => {}
    }
    true
}

/// `y` in any operation preview; the opening verb again in an `m`- or
/// `r`-opened one. Nothing else — not `b` in a name-prompt preview, not `.`
/// in a context-menu one, not the other verb (user ruling, 2026-09-23).
pub(super) fn confirm_key(key: KeyPress, opened_by: &str) -> bool {
    if key.code == KeyCode::Char('y') && !key.modifiers.control && !key.modifiers.alt {
        return true;
    }
    let again = match is_history_key(key) {
        Some(HistoryKey::Merge) => "m",
        Some(HistoryKey::Rebase) => "r",
        None => return false,
    };
    again == opened_by
}

/// The rebase `R` opens the rewrite editor on from this preview
/// (rebase-preview-keeps-drops): `Some` exactly when `subject` is a rebase
/// whose plan suggests drops (`already_in` is `Drop`) and whose as-run op
/// carries them, of the branch `HEAD` is on (`head_branch`, a full ref
/// name). The key, the `R to keep them` row and the footer's `R keep` all
/// ask this, so none of them names a key the others would not answer.
pub(crate) fn keeps_drops<'a>(
    subject: &'a PreviewSubject,
    plan: &PreviewPlan,
    head_branch: Option<&str>,
) -> Option<&'a RebaseParams> {
    let Some(Operation::Rebase(params)) = subject.operation() else {
        return None;
    };
    let suggested = matches!(plan.already_in, Some(DropSuggestion::Drop(_)));
    (suggested && !params.drops.is_empty() && head_branch == Some(params.branch.name.as_str()))
        .then_some(params)
}

/// [`keeps_drops`] for the open preview: its landed plan, and the branch
/// `HEAD` is on as the refs read it. `None` while the plan is out.
pub(super) fn keeps_drops_here(state: &AppState) -> Option<&RebaseParams> {
    let preview = state.preview.as_ref()?;
    // A box that cannot act (undrawable, or nothing to do) names no key;
    // `R` must not answer where the footer says only `esc` (30n m1).
    if !preview.acts() {
        return None;
    }
    let planned = preview.planned()?;
    let head = state.refs.as_deref().and_then(current_branch)?;
    keeps_drops(&preview.subject, &planned.plan, Some(&head.name))
}

/// A confirm key: `Confirm` wherever the cursor is, but only over a picture
/// that is on screen, landed and able to act. Otherwise swallowed, and the
/// box stays open — unlike `Enter` on `Cancel`, a confirm key never closes
/// it. Not queued while planning: the user sees the picture before he
/// confirms (M5's promise), and the box already reads `predicting…`.
pub(super) fn confirm_by_key(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    next_id: &mut impl FnMut() -> JobId,
) {
    if !fits(state) {
        return;
    }
    let Some(preview) = state.preview.as_ref() else {
        return;
    };
    if preview.planning() || !preview.acts() {
        return;
    }
    confirm_preview(state, commands, next_id);
}

/// The footer keys a box for `preview` names: `y confirm` for an operation
/// preview, and the opening verb when it confirms again — but only where
/// [`confirm_by_key`] answers them: a plan that can act, or one still
/// planning, where the key is waiting for the picture rather than dead. A
/// closing box and a can't-act plan name neither (`02-keymap.md:111`,
/// review m6-amend-11-12 Minor 1).
///
/// `keep` draws `R keep` after them, exactly when `R` is live
/// ([`keeps_drops`]).
pub(super) fn footer_keys(mut view: PreviewBox, preview: &PreviewState, keep: bool) -> PreviewBox {
    if !(preview.acts() || preview.planning()) {
        return view;
    }
    view.keep = keep;
    // A stash put-back names its confirm by its verb, `y apply` / `y pop`
    // (m6-conflicts/86), through the same footer slot `m merge` uses. `y`
    // is the key either way (`confirm_key`); the opening key is never a
    // confirm here — no `p p`.
    if let Some((_, pop)) = stash_put_back(&preview.subject) {
        view.again = Some(Again::confirm(stash_verb(pop)));
        return view;
    }
    // A put-back from J names its confirm the same way: `y put it back`
    // (m7-power-tools/24-a2), never `R keep`.
    if let Some(Operation::Recover(_)) = preview.subject.operation() {
        view.keep = false;
        view.again = Some(Again::confirm("put it back"));
        return view;
    }
    if let PreviewSubject::Operation { opened_by, .. } = &preview.subject {
        view.confirm_key = true;
        view.again = match *opened_by {
            "m" => Some(Again::opening("m", "merge")),
            "r" => Some(Again::opening("r", "rebase")),
            _ => None,
        };
    }
    view
}

/// `Enter`: act on the choice under the cursor.
pub(super) fn enter(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    next_id: &mut impl FnMut() -> JobId,
) {
    // A picture that cannot be seen is never confirmed (design note, "New
    // UI"): below `PREVIEW_MIN` the box is one line of text and the plan is
    // not on screen at all, so `Enter` does nothing and `Esc` is the way
    // out.
    if !fits(state) {
        return;
    }
    let Some(preview) = state.preview.as_ref() else {
        return;
    };
    // Index 0 is always the safe choice, and a box with nothing to confirm
    // has only that one.
    if preview.cursor == 0 || !preview.acts() {
        close_preview(state, commands);
        return;
    }
    confirm_preview(state, commands, next_id);
}

/// Is the screen big enough to show a preview at all?
///
/// Measured against the terminal, which is what the design note's "< 40
/// cols or < 12 rows" means and what [`preview_box_rect`] measures too, so
/// the key and the picture cannot disagree about whether there is a picture.
#[must_use]
pub fn fits(state: &AppState) -> bool {
    state.size.columns >= PREVIEW_MIN.0 && state.size.rows >= PREVIEW_MIN.1
}

/// How many choices the open box offers; `0` when none is open.
pub(super) fn choice_count(state: &AppState) -> usize {
    preview_box(state).map_or(0, |view| view.choices().len())
}
