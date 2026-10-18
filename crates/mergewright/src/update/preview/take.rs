//! The take (`t`) preview: its key, its operation and its title.

use super::{
    AppState, Command, JobId, KeyCode, KeyPress, Operation, PickParams, PreviewSubject, RefLabel,
    Status, open_preview, short, short_name_of,
};
use crate::update::op_keys::OpKeys;
use mergewright_ui::keymap::{ActionId, Keymap};

/// `o takes a commit · this row isn't one` (`graph.take`); unbound, the
/// fact alone.
pub(crate) fn not_a_commit_words(keys: &Keymap) -> String {
    match keys.key_verb(&[ActionId("graph.take")], "takes a commit") {
        Some(takes) => format!("{takes} \u{b7} this row isn't one"),
        None => "this row isn't one".to_owned(),
    }
}

/// `o is waiting for the refs` (`graph.take`); unbound, `waiting for the
/// refs` (the take was asked from the menu).
pub(crate) fn waiting_for_refs(keys: &Keymap) -> String {
    keys.key_verb(&[ActionId("graph.take")], "is waiting for the refs")
        .unwrap_or_else(|| "waiting for the refs".to_owned())
}

/// A one-commit cherry-pick's title in the words `o` is named by
/// (m7-power-tools/11): `take <sha7> "<subject>" onto <branch>`, whether it
/// was opened by `o` or by slot 2's `⏎`, planning, ready or refused.
/// `describe_pick`'s `cherry-pick <sha7> "…"` stays the journal's and `u`'s
/// words; only the box says `take`.
pub(super) fn take_title_of(op: &Operation) -> Option<String> {
    let Operation::CherryPick(params) = op else {
        return None;
    };
    if params.commits.len() != 1 {
        return None;
    }
    let branch = (params.head.name != "HEAD").then(|| short_name_of(&params.head.name));
    Some(take_title(params, branch.as_deref()))
}

/// `take <sha7> "<subject>" onto <b>`, `onto HEAD` when detached. The
/// whole line goes through `sanitize_line` where it is drawn, subject and
/// branch included.
pub(super) fn take_title(params: &PickParams, branch: Option<&str>) -> String {
    let sha = params
        .commits
        .first()
        .map_or_else(String::new, |&id| short(id));
    format!(
        "take {sha} \"{}\" onto {}",
        params.subject,
        branch.unwrap_or("HEAD")
    )
}

/// Is this `o`, plain?
#[must_use]
pub(crate) fn is_take_key(key: KeyPress) -> bool {
    key.code == KeyCode::Char('o') && !key.modifiers.control && !key.modifiers.alt
}

/// `o` (keymap §2.3): take the cursor's commit onto `HEAD`'s branch —
/// the very operation slot 2 of the `.` menu names (`verbs::pick_params`),
/// through the same preview, so both read `take … onto`.
///
/// What the graph already knows is refused here, before any preview opens,
/// in the words the plan would use (`PickParams::check_commit`): `HEAD`'s
/// own row, a merge. An ancestor of `HEAD` needs a read of history, so its
/// preview's plan refuses it (m7-power-tools/11a) and the box closes with
/// the same words. A row that is not a commit — the uncommitted row, a
/// stash — is refused in words.
pub(crate) fn take_key(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    next_id: &mut impl FnMut() -> JobId,
) {
    match take_operation(state) {
        Ok(op) => {
            // Whether it opened: every refusal inside `open_preview` wrote
            // its own sticky line, and `o` was pressed outright, so there is
            // nothing left to decide here (as `history_key`).
            let _opened: bool = open_preview(
                state,
                commands,
                PreviewSubject::Operation {
                    op: Box::new(op),
                    opened_by: "o",
                },
                false,
                next_id,
            );
        }
        Err(status) => state.status = status,
    }
}

/// The operation `o` names from the cursor's row, or the status saying why
/// there is none.
pub(super) fn take_operation(state: &AppState) -> Result<Operation, Status> {
    let not_a_commit = || Status::sticky(not_a_commit_words(&state.keymap));
    if state.on_working_tree {
        return Err(not_a_commit());
    }
    let row = state
        .history
        .as_ref()
        .and_then(crate::history::History::selected)
        .ok_or_else(not_a_commit)?;
    let refs = state
        .refs
        .as_deref()
        .ok_or_else(|| Status::sticky(waiting_for_refs(&state.keymap)))?;
    let id = row.commit.id;
    if refs
        .on(id)
        .iter()
        .any(|label| matches!(label, RefLabel::Stash(_)))
    {
        return Err(not_a_commit());
    }
    // No tip on HEAD: reachable, since an orphan branch still shows the
    // other branches' commits. `check_pick` refuses that HEAD as
    // `HeadUnborn` (mergewright-ops `head_is`), and `o` says the same words
    // (ticket 11 Decided; 30n m3).
    let head = crate::update::context_menu::verbs::head_tip(refs).ok_or_else(|| {
        Status::sticky(mergewright_ops::operation::Refusal::HeadUnborn.explain(&OpKeys(&state.keymap)))
    })?;
    let subject = String::from_utf8_lossy(row.commit.subject()).into_owned();
    let params = crate::update::context_menu::verbs::pick_params(id, &subject, &head);
    let refused = params.check_commit(id, id == head.oid, row.commit.parents.len());
    let op = Operation::CherryPick(params);
    match refused {
        Ok(()) => Ok(op),
        Err(refusal) => Err(crate::update::operate::refusal_status(
            &op,
            &refusal,
            state.size.columns,
            &state.keymap,
        )),
    }
}
