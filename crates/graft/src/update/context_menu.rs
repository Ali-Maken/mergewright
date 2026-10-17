//! The `.`/right-click popup: opening it, answering its own keys, and
//! carrying out whatever it dispatches (`context-actions-menu`,
//! m2-details-and-diffs/12).

use graft_core::event::{KeyCode, KeyPress};
use graft_core::job::JobId;
use graft_core::oid::Oid;

use graft_ui::context_menu::{ContextAction, ContextMenu};

use super::detail::DetailSpec;
use super::events::Command;
use super::preview::PreviewSubject;
use super::state::{AppState, Status};

mod slots;
pub mod verbs;

pub use verbs::HistoryVerb;

/// The `.`/right-click popup while it is open: the row it opened on, the
/// menu the widget draws, and what each of its history rows means.
///
/// The three travel together because they are one answer to one `.`: the
/// widget reports a row by index ([`ContextAction::History`]), and `verbs`
/// is the list that index points into. Keeping them in one field is what
/// makes it impossible for the labels on screen and the operations behind
/// them to drift apart.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpenMenu {
    /// The absolute `History` row it opened on, or [`WORKING_TREE_ROW`].
    pub row: usize,
    /// What the widget draws.
    pub menu: ContextMenu,
    /// The history verbs this row offers, in the order the menu lists them.
    /// Empty on the working-tree row and on a row the refs have not caught
    /// up with.
    pub verbs: Vec<HistoryVerb>,
}

/// The sentinel `row` for the uncommitted-changes row's menu: it names no
/// absolute `History` row (`AppState::on_working_tree` is what selects it
/// instead), so this is never a real row index.
pub(super) const WORKING_TREE_ROW: usize = usize::MAX;

/// `.`/right-click on a commit row: open its menu. `marked` asks the same
/// source of truth the marker column asks (`view::radar::findings_for`,
/// `view/entries.rs`) rather than a second, hand-maintained flag — a row
/// with at least one finding lists `Explain`.
pub(super) fn open_for_history_row(next: &mut AppState, row: usize) {
    let mut marked = false;
    let mut slots = slots::Slots::empty();
    if let Some(entry) = next
        .history
        .as_ref()
        .and_then(|history| history.row_at(row))
    {
        let id = entry.commit.id;
        let parents = crate::history::parents::of(entry, next.refs.as_deref()).to_vec();
        let subject = String::from_utf8_lossy(entry.commit.subject()).into_owned();
        let findings = crate::view::radar::findings_for(next, id, &parents, &subject);
        marked = !findings.is_empty();
        // No refs yet means no branch to name: the row's own actions and the
        // dropped keys still work, and a verb that cannot say which branch it
        // moves is one this menu is better off not offering.
        if let Some(refs) = next.refs.as_deref() {
            slots = slots::history_slots(id, &parents, &subject, refs, &findings, &next.keymap);
        }
    }
    let items = slots.menu_items();
    next.context_menu = Some(OpenMenu {
        row,
        menu: ContextMenu::for_row(marked, items, slots.leading, slots.dim_first, &next.keymap),
        verbs: slots.verbs,
    });
}

/// `.`/right-click on the uncommitted-changes row: `Open details`, the
/// dropped keys, and absorb — dim, reading `nothing staged`, when nothing
/// is (m7-power-tools/09).
pub(super) fn open_for_working_tree(next: &mut AppState) {
    let staged = next
        .working_tree
        .as_deref()
        .is_some_and(|tree| tree.staged().next().is_some());
    next.context_menu = Some(OpenMenu {
        row: WORKING_TREE_ROW,
        menu: ContextMenu::for_working_tree(staged, &next.keymap),
        verbs: Vec::new(),
    });
}

/// `.`: open the menu for whatever the cursor now names.
pub(super) fn open_selected(next: &mut AppState) {
    if next.on_working_tree {
        open_for_working_tree(next);
        return;
    }
    let Some(row) = next.history.as_ref().map(|history| history.cursor) else {
        return;
    };
    open_for_history_row(next, row);
}

/// The menu's own keys, while it is open. It claims exactly five — `j`/`k`
/// (and the arrows) move, `Enter` dispatches the selected action and closes,
/// `Esc` and `.` close without acting — and **nothing else**
/// (`docs/design/02-keymap.md` §2.1, "an overlay that needs only a cursor is
/// not a modal pane"). None of the five is a verb, so no verb letter is
/// taken away from the panes underneath.
///
/// Returning `true` means the key stops here. Every other key gets `false`:
/// the menu closes and the key carries on down `keys::key_event` to mean what
/// it always means, so `m` on a row labelled `Merge main into feature (m)`
/// really merges instead of being eaten. A trailing `(key)` in a label is a
/// promise that the letter is live right now; the old catch-all
/// (`_ => close` then an unconditional `true`) broke that promise for every
/// verb the menu advertises (m5-history-and-ghost-preview/13).
///
/// Nothing further down `key_event` reads `next.context_menu`, so closing
/// here and falling through is the whole of "as if the menu had not been
/// open" — `remote::cancel_operating`'s `context_menu.is_some()` guard sees
/// the `None` this already wrote.
pub(super) fn context_menu_key(
    next: &mut AppState,
    commands: &mut Vec<Command>,
    key: KeyPress,
) -> bool {
    let Some(open) = next.context_menu.as_mut() else {
        return false;
    };
    match key.code {
        KeyCode::Char('j') | KeyCode::Down => open.menu.move_cursor(1),
        KeyCode::Char('k') | KeyCode::Up => open.menu.move_cursor(-1),
        KeyCode::Enter => {
            // A disabled row answers with its reason and stays open: the
            // reason is drawn beside the row it belongs to, and closing the
            // menu would take the question off screen with the answer.
            if let Some(reason) = open.menu.selected_refusal() {
                let reason = reason.as_str().to_owned();
                next.status = Status::sticky(reason);
                return true;
            }
            let action = open.menu.selected();
            // The verb list travels with the action it was chosen from, so
            // nothing can dispatch against a list that has since been
            // rebuilt.
            let row = open.row;
            let verbs = std::mem::take(&mut open.verbs);
            next.context_menu = None;
            commands.extend(dispatch(next, row, &verbs, action));
        }
        // `.` closes what `.` opened, exactly as `Esc` does. It is the
        // doorway (ADR-0039), and §2.1 says a key that opens a place closes
        // it again on the same key or `Esc`. `.` has to be named here rather
        // than left to the fall-through below, because falling through
        // reaches `keys::graph_key`, whose `.` reopens the menu that was
        // just closed — so `.` could never shut it
        // (m5-history-and-ghost-preview/13 left this unpinned as a spec
        // call; ticket 15 is the ruling).
        KeyCode::Escape | KeyCode::Char('.') => next.context_menu = None,
        _ => {
            next.context_menu = None;
            return false;
        }
    }
    true
}

/// The `Oid` a menu opened on `row` acts on: `None` on the working-tree
/// sentinel, else whatever row that absolute index names.
fn oid_for(next: &AppState, row: usize) -> Option<Oid> {
    if row == WORKING_TREE_ROW {
        return None;
    }
    next.history
        .as_ref()
        .and_then(|history| history.row_at(row))
        .map(|entry| entry.commit.id)
}

/// `apply_context_action`'s `next_id` plumbing from `next.local_jobs` — the
/// same copy-out/copy-back idiom `keys::toggle_compare_selected` uses,
/// needed because `next` is borrowed both by the closure and by the call.
pub(super) fn dispatch(
    next: &mut AppState,
    row: usize,
    verbs: &[HistoryVerb],
    action: ContextAction,
) -> Vec<Command> {
    let oid = oid_for(next, row);
    let mut local_jobs = next.local_jobs;
    let commands =
        apply_context_action(next, action, row, oid, verbs, &mut || local_jobs.next_id());
    next.local_jobs = local_jobs;
    commands
}

/// What a chosen context-menu item does. `oid` is `None` only on the
/// working-tree row; `CopySha`, `CompareWithThis` and `Explain` can never be
/// reached there (`ContextMenu::for_working_tree` never lists them), so that
/// combination is a routing bug, not a user-facing error — `debug_assert!`
/// and no command.
pub fn apply_context_action(
    state: &mut AppState,
    action: ContextAction,
    row: usize,
    oid: Option<Oid>,
    verbs: &[HistoryVerb],
    next_id: &mut impl FnMut() -> JobId,
) -> Vec<Command> {
    match action {
        // `None` is a routing bug `commit_of` asserts on: no command.
        ContextAction::CopySha => match commit_of(oid, "CopySha") {
            Some(oid) => copy_sha(state, oid),
            None => Vec::new(),
        },
        ContextAction::CompareWithThis => {
            let Some(oid) = commit_of(oid, "CompareWithThis") else {
                return Vec::new();
            };
            super::detail::toggle_compare(state, row, oid, next_id)
        }
        ContextAction::OpenDetails => {
            let spec = match oid {
                Some(oid) => DetailSpec::Commit(oid),
                None => DetailSpec::WorkingTree,
            };
            state.detail.open(spec, next_id)
        }
        ContextAction::Explain => {
            let Some(oid) = commit_of(oid, "Explain") else {
                return Vec::new();
            };
            state.detail.explain_requested = Some(oid);
            Vec::new()
        }
        // The keys the hint bar drops at 80 and 120 columns, so that `. more`
        // leads to them (m5-03 review, Major 1). Each one calls exactly what
        // its key calls — a second path to the same function, never a second
        // implementation. `oid` and `row` are beside the point: all four act
        // on `HEAD` and the current branch, wherever the cursor is.
        //
        // The `bool` each key function returns says whether it claimed the
        // key. From the menu the action was chosen outright, and a refusal
        // (no remote, nothing to undo) already says so on the status line, so
        // there is nothing left here to decide on.
        ContextAction::Fetch => {
            let mut commands = Vec::new();
            let _claimed: bool = super::remote::fetch_key(state, &mut commands, next_id);
            commands
        }
        ContextAction::Pull => {
            let mut commands = Vec::new();
            let _claimed: bool = super::remote::pull_key(state, &mut commands, next_id);
            commands
        }
        ContextAction::Push => {
            let _claimed: bool = super::remote::push_key(state);
            Vec::new()
        }
        ContextAction::Undo => super::undo::undo_key(state, false, next_id),
        ContextAction::Absorb => super::amend_into::absorb_chosen(state, next_id),
        // A history verb: `Checkout`, `Merge`, `Rebase`, `Cherry-pick`,
        // `Revert`, `Reset`, a ref deletion or one of the Fix items. The
        // widget reported which row was chosen; `verbs` is the list that
        // index points into, and `update::context_menu::verbs` decided what
        // each one means (m5-history-and-ghost-preview/08, piece 6).
        ContextAction::History(index) => {
            let Some(chosen) = verbs.get(index) else {
                debug_assert!(
                    false,
                    "History({index}) against {} verbs: the menu and the list it was built from disagree",
                    verbs.len()
                );
                return Vec::new();
            };
            run_history_verb(state, chosen, next_id)
        }
        ContextAction::Unavailable => unavailable(),
    }
}

/// `Copy SHA`: the full SHA to the clipboard, its first seven on the
/// status line unless a failure is showing there.
fn copy_sha(state: &mut AppState, oid: Oid) -> Vec<Command> {
    let sha = oid.to_string();
    let short = sha.chars().take(7).collect();
    if !matches!(state.status, Status::Failed(_) | Status::Sticky { .. }) {
        state.status = Status::Copied { short };
    }
    vec![Command::CopyToClipboard(sha)]
}

/// The dim slot is always drawn disabled: `Enter` and a click answer with
/// its reason before dispatch, so reaching here is a routing bug.
fn unavailable() -> Vec<Command> {
    debug_assert!(false, "the dim slot was dispatched");
    Vec::new()
}

/// The commit an action acts on. `CopySha`, `CompareWithThis` and `Explain`
/// are never listed on the working-tree row (`ContextMenu::for_working_tree`
/// lists none of them), so `None` here is a routing bug rather than anything
/// the user did: assert in debug, and answer with no command.
fn commit_of(oid: Option<Oid>, action: &str) -> Option<Oid> {
    debug_assert!(
        oid.is_some(),
        "{action} is unreachable on the working-tree row (for_working_tree never lists it)"
    );
    oid
}

/// A chosen history verb. Checkout follows the switch rule — at once on a
/// clean tree, the preview with edits or a detached target (10b); every
/// other verb ends in its preview.
fn run_history_verb(
    state: &mut AppState,
    chosen: &HistoryVerb,
    next_id: &mut impl FnMut() -> JobId,
) -> Vec<Command> {
    match super::switch::target_of(&chosen.op) {
        Some(target) => super::switch::switch_to(state, target, ".", next_id),
        None => preview_history_verb(state, chosen, next_id),
    }
}

/// A chosen history verb other than Checkout: open the Ghost Preview of the
/// very operation its label named.
///
/// These end in a picture, never in a write — only `Confirm` in that
/// picture runs anything (PROMPT.md §1.7). Checkout on a clean tree is the
/// one `.` item that writes at once (the switch rule, keymap §2.1).
fn preview_history_verb(
    state: &mut AppState,
    chosen: &HistoryVerb,
    next_id: &mut impl FnMut() -> JobId,
) -> Vec<Command> {
    let mut commands = Vec::new();
    // The `bool` says whether a preview opened. Every refusal inside
    // `open_preview` — a write already running, git mid-operation — has
    // written its own sticky line saying so, and the item was chosen
    // outright, so there is nothing left here to decide on: the same
    // reasoning as the four dropped keys above.
    let _opened: bool = super::preview::open_preview(
        state,
        &mut commands,
        PreviewSubject::Operation {
            op: Box::new(chosen.op.clone()),
            opened_by: ".",
        },
        false,
        next_id,
    );
    commands
}

#[cfg(test)]
mod tests;
