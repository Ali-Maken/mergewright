//! The journal screen's keys: move, next lost, branch here, close, `Enter`.

use super::{
    AppState, Command, EntryId, JobId, KeyCode, KeyPress, ListRow, Loading, Oid, OperationKind,
    Status, rebuild, row_commit, row_key,
};

/// `J`, and every key while the browser is open. `true` when the key was
/// this view's, so the caller sends it nowhere else — the same shape as
/// `update::map::map_key`, including the wildcard: an open overlay swallows
/// the keys it does not use rather than letting them move the graph
/// underneath it.
pub(in crate::update) fn journal_key(
    next: &mut AppState,
    commands: &mut Vec<Command>,
    key: KeyPress,
    next_id: &mut impl FnMut() -> JobId,
) -> bool {
    if key.modifiers.control || key.modifiers.alt {
        return false;
    }
    if next.journal.shown {
        return shown_key(next, commands, key, next_id);
    }
    // From the graph or from the changes pane, as `c` and `p` are: a
    // refusal that names `J` must name a key that works where he is
    // (m6-conflicts/85 m2). The overlay stands where the graph does and
    // takes the keys the graph would, so the keyboard goes to the graph.
    if key.code == KeyCode::Char('J') {
        commands.extend(journal_enter(next, next_id));
        return true;
    }
    false
}

/// `J`: the browser opens at once on what is on hand — stage 1, the `●`
/// rows, on the newest — the journal is read again, and stage 2 (git's
/// reflog records) is asked; its answer asks stage 3 ([`outside_landed`]).
pub(crate) fn journal_enter(
    state: &mut AppState,
    next_id: &mut impl FnMut() -> JobId,
) -> Vec<Command> {
    state.detail.focus = graft_ui::detail::DetailFocus::Graph;
    let journal = &mut state.journal;
    journal.shown = true;
    journal.cursor = None;
    journal.expanded = None;
    journal.touched = false;
    journal.offered = None;
    journal.recovering = None;
    // A find belongs to one look at J: it opens without one.
    journal.find = super::find::JournalFind::default();
    journal.outside = Loading::Idle;
    journal.lost = Loading::Idle;
    // Looking at the browser is what the interrupted-entry notice asks
    // for, so the notice stops once it has been taken up.
    journal.notice_seen = true;
    rebuild(state);
    // Read it fresh: another Graft on the same worktree may have appended
    // since the last read.
    let mut commands = state.journal.read(next_id);
    let id = next_id();
    state.journal.outside = Loading::Pending(id);
    commands.push(Command::ReadOutside { id });
    commands
}

/// The browser's own keys.
pub(super) fn shown_key(
    next: &mut AppState,
    commands: &mut Vec<Command>,
    key: KeyPress,
    next_id: &mut impl FnMut() -> JobId,
) -> bool {
    // `Tab` belongs to the detail pane's focus cycle, always: an overlay
    // that swallowed it would be one the user could never `Tab` out of
    // (m3-working-tree-undo-and-stash/06).
    if matches!(key.code, KeyCode::Tab | KeyCode::BackTab) {
        return false;
    }
    // Any key since `J` means he has chosen where to look: the lost rows'
    // preselect must not move the cursor out from under him.
    next.journal.touched = true;
    match key.code {
        KeyCode::Char('J') | KeyCode::Escape => {
            close(next, commands);
        }
        KeyCode::Up | KeyCode::Char('k') => {
            let above = next.journal.selected().checked_sub(1);
            move_to(next, above);
        }
        KeyCode::Down | KeyCode::Char('j') => {
            let below = next.journal.selected().saturating_add(1);
            let below = (below < next.journal.rows()).then_some(below);
            move_to(next, below);
        }
        KeyCode::Char('n') => next_lost(next, true),
        KeyCode::Char('N') => next_lost(next, false),
        KeyCode::Char(' ') => {
            if let Some(list) = &next.journal.list {
                let key = row_key(list, next.journal.selected());
                next.journal.expanded = if next.journal.expanded == key {
                    None
                } else {
                    key
                };
            }
        }
        KeyCode::Char('b') => branch_here(next),
        KeyCode::Enter => enter(next, commands, next_id),
        _ => {}
    }
    true
}

/// The cursor to row `index`, when there is one.
pub(super) fn move_to(next: &mut AppState, index: Option<usize>) {
    if let (Some(list), Some(index)) = (&next.journal.list, index)
        && let Some(key) = row_key(list, index)
    {
        next.journal.cursor = Some(key);
    }
}

/// `n`/`N` with no find active: the next (older) or previous (newer) `✗`
/// row, lost edits included. Nowhere to go stays put.
pub(super) fn next_lost(next: &mut AppState, older: bool) {
    let Some(list) = &next.journal.list else {
        return;
    };
    let from = next.journal.selected();
    let is_lost = |index: &usize| {
        matches!(
            list.rows.get(*index),
            Some(ListRow::Lost { .. } | ListRow::LostEdit { .. })
        )
    };
    let found = if older {
        (from + 1..list.rows.len()).find(is_lost)
    } else {
        (0..from).rev().find(is_lost)
    };
    move_to(next, found);
}

/// `b`: the name prompt for a new branch at the row's commit, J left shown
/// under it (V1: as the stash's `b` leaves S). The name is prefilled when
/// the last `⏎` on this row offered one.
pub(super) fn branch_here(next: &mut AppState) {
    let Some(list) = next.journal.list.clone() else {
        return;
    };
    let index = next.journal.selected();
    match list.rows.get(index) {
        Some(ListRow::LostEdit { .. }) => {
            next.status = Status::Sticky {
                reason: "a lost edit has no commit".to_owned(),
                keys: lost_edit_keys(&next.keymap),
            };
            return;
        }
        // A `●` entry can move several refs or none, so "its commit" has
        // no single answer (m7/30o Minor 3): say where `b` does work.
        Some(ListRow::Graft(_)) => {
            // Reached by `journal.branch`, so bound; an unbound one draws
            // nothing rather than a keyless instruction (W3).
            if let Some(words) = branch_needs_row(&next.keymap) {
                next.status = Status::sticky(words);
            }
            return;
        }
        Some(ListRow::Outside(_) | ListRow::Lost { .. }) | None => {}
    }
    let Some(at) = row_commit(&list, index) else {
        next.status = Status::sticky("this step names no commit");
        return;
    };
    let key = row_key(&list, index);
    let prefill = next
        .journal
        .offered
        .as_ref()
        .filter(|(offered, _)| Some(offered) == key.as_ref())
        .map_or("", |(_, name)| name.as_str())
        .to_owned();
    // `false`: something modal is up, and it owns the keyboard — nothing
    // this key could say would be seen under it.
    let _opened: bool = crate::update::name_prompt::open_name_prompt_with(
        next,
        crate::update::name_prompt::NameKind::Branch {
            at,
            checkout: false,
        },
        &prefill,
    );
}

/// `Esc`/`J`: the browser goes away, and a read it asked for that is only
/// refreshing a list already on hand goes with it — nothing on screen is
/// waiting for that one.
///
/// A read with **nothing** behind it stays. It is the launch read, and the
/// footer's interrupted-entry notice and `u` are both waiting on it whether
/// the browser is open or not; cancelling it used to blank the field back to
/// `Idle`, which is exactly the condition [`request`] re-reads on, so `settle`
/// asked for the same read again in the same pass. That churn is
/// `KNOWN_GAPS.md` row 217, and the cause is here rather than in `settle`:
/// `Esc` was stopping work the application still wanted
/// (m5-history-and-ghost-preview/05).
pub(in crate::update) fn close(next: &mut AppState, commands: &mut Vec<Command>) {
    let journal = &mut next.journal;
    journal.shown = false;
    journal.cursor = None;
    journal.expanded = None;
    journal.offered = None;
    // The list's own reads are for the open browser only.
    if let Loading::Pending(id) = journal.outside {
        commands.push(Command::CancelJob(id));
    }
    if let Loading::Pending(id) = journal.lost {
        commands.push(Command::CancelJob(id));
    }
    journal.outside = Loading::Idle;
    journal.lost = Loading::Idle;
    if let Some((id, _)) = journal.recovering.take() {
        commands.push(Command::CancelJob(id));
    }
    if let Some(id) = journal.reading
        && journal.entries != Loading::Pending(id)
    {
        journal.reading = None;
        commands.push(Command::CancelJob(id));
    }
}

/// `Enter`, by the row under the cursor (keymap §2.11, 2b):
/// - `●`: `u` on the entry `u` would take back, `U` on the one `U` would
///   put back, and on anything older the time machine ([`enter_graft`]).
/// - `$` switch: switch back to the branch it left (the M7 switch rule).
/// - any other `$`, `✗` or lost edit: ask what it puts back
///   (`recover_for`), answered in [`recover_answered`].
pub(super) fn enter(
    next: &mut AppState,
    commands: &mut Vec<Command>,
    next_id: &mut impl FnMut() -> JobId,
) {
    let Some(list) = next.journal.list.clone() else {
        return;
    };
    let index = next.journal.selected();
    match list.rows.get(index) {
        Some(ListRow::Graft(id)) => enter_graft(next, commands, id.clone(), next_id),
        Some(ListRow::Outside(step)) if switched_from(&step.record.subject).is_some() => {
            let Some(target) = switched_from(&step.record.subject) else {
                return;
            };
            if crate::update::switch::switch_into(next, commands, target, "\u{23ce}", next_id) {
                // The result is on the graph.
                close(next, commands);
            }
        }
        // What it puts back needs the refs read fresh, off the loop; the
        // answer lands in `recover_answered`. A second `⏎` replaces the
        // first, whose answer is then stale by id.
        Some(ListRow::Outside(_) | ListRow::Lost { .. } | ListRow::LostEdit { .. }) => {
            let Some(key) = row_key(&list, index) else {
                return;
            };
            if let Some((stale, _)) = next.journal.recovering.take() {
                commands.push(Command::CancelJob(stale));
            }
            let id = next_id();
            next.journal.recovering = Some((id, key));
            commands.push(Command::RecoverFor {
                id,
                list,
                row: index,
                keys: std::sync::Arc::clone(&next.keymap),
            });
        }
        None => {}
    }
}

/// The branch a shell switch left, as a switch target: `checkout: moving
/// from main to spike` → `main`; a detached start is its commit.
pub(crate) fn switched_from(subject: &str) -> Option<crate::update::switch::SwitchTarget> {
    let (from, _) = subject
        .strip_prefix("checkout: moving from ")?
        .split_once(" to ")?;
    let hex = (from.len() == 40 || from.len() == 64) && from.bytes().all(|b| b.is_ascii_hexdigit());
    Some(if hex {
        crate::update::switch::SwitchTarget::Detached(Oid::from_hex(from).ok()?)
    } else {
        crate::update::switch::SwitchTarget::Branch(format!("refs/heads/{from}"))
    })
}

/// `Enter` on a `●` row: `u`, `U` or **the time machine** — revert to just
/// before that entry (`docs/design/02-keymap.md` §2.11,
/// `docs/design/06-undo-journal-design.md` §6.6).
///
/// The time machine is several restores in a row, so it is the one thing
/// here that never runs from this key: it opens a Ghost Preview and the
/// preview's `Confirm` runs it (ADR-0031, PROMPT.md §1.5). The browser is
/// **hidden, not closed** — the cursor is left where it is, so `Esc` on the
/// preview puts the browser back on the same row
/// (`preview::close_preview`'s `return_to_journal`).
pub(super) fn enter_graft(
    next: &mut AppState,
    commands: &mut Vec<Command>,
    id: EntryId,
    next_id: &mut impl FnMut() -> JobId,
) {
    let Loading::Ready(read) = &next.journal.entries else {
        return;
    };
    let Some(entry) = read.entries.iter().find(|entry| entry.id == id) else {
        return;
    };
    let kind = entry.kind;
    // A step Graft doesn't undo (a fetch, a push): the time machine skips
    // it, so "before this" is "before" the next newer `●` row, which keeps
    // its own `⏎`. Nothing to go back to here (m7/30o Minor 4).
    if let Some(word) = not_undone_kind(entry) {
        next.status = Status::sticky(format!(
            "Graft doesn't undo a {word} · nothing to go back to on this row"
        ));
        return;
    }
    let redo = if graft_ops::journal::next_undo(&read.entries).is_some_and(|at| at.id == id) {
        false
    } else if graft_ops::journal::next_redo(&read.entries).is_some_and(|at| at.id == id) {
        true
    } else {
        // The time machine aims at an **operation**. An `Undo` or `Redo`
        // row is a move along the journal, not a state the repository was
        // in, and `plan_revert_to` reaches its answer by inverting the
        // operations in force since the target — it has no way to put back
        // what an undo took away. Aimed at `undo(B)` in `[A, B, undo(B),
        // C]` it plans `inverse(C)` alone and leaves `B` undone, while the
        // box's title reads `revert to before "undo: B"`. Refusing in
        // words is the honest answer: the row you pick is the state you get
        // (review 08 Major 2), or no row is picked at all.
        let step = match kind {
            OperationKind::Undo => Some("an undo"),
            OperationKind::Redo => Some("a redo"),
            _ => None,
        };
        if let Some(step) = step {
            next.status = Status::sticky(format!(
                "the time machine reverts to before an operation · this row is {step}"
            ));
            return;
        }
        if crate::update::preview::open_preview(
            next,
            commands,
            crate::update::preview::PreviewSubject::RevertTo(id),
            true,
            next_id,
        ) {
            next.journal.shown = false;
        }
        // A refusal left its own sentence on the status line and the
        // browser stays where it is; there is nothing else to say.
        return;
    };
    undo_from_j(next, commands, redo, next_id);
}

/// The word J draws for a step Graft doesn't undo, or `None` for one it
/// does. The predicate is `graft_ops::journal::undoable`, the one
/// `plan_revert_to` skips by, so J's `⏎` and its bar cannot disagree with
/// what the time machine would do.
pub(crate) fn not_undone_kind(entry: &graft_core::journal::JournalEntry) -> Option<&'static str> {
    if graft_ops::journal::undoable(entry) {
        return None;
    }
    Some(match entry.kind {
        OperationKind::Fetch => "fetch",
        OperationKind::Push => "push",
        // `undoable` is false for a pull only when it changed nothing.
        OperationKind::Pull => "pull that changed nothing",
        // `undoable` answers false for no other kind; "step" is honest if
        // graft-ops ever adds one.
        _ => "step",
    })
}

/// `u`/`U` in J and `⏎` on the `← u`/`← U` row: one path, one outcome
/// (m7/30o R3). Launched → J closes, the result is on the graph, as 03's
/// undo landing follows. Asked → J stays under the undo-after-push box
/// (its launching choice closes J, `undo::after_push_chosen`; `Esc` brings
/// J back on the same row). Refused → J stays, the words on its status row.
pub(crate) fn undo_from_j(
    next: &mut AppState,
    commands: &mut Vec<Command>,
    redo: bool,
    next_id: &mut impl FnMut() -> JobId,
) {
    use crate::update::undo::UndoStart;
    match crate::update::undo::start_undo(next, commands, redo, next_id) {
        UndoStart::Launched => close(next, commands),
        UndoStart::Asked | UndoStart::Refused => {}
    }
}

/// `⏎ puts it back`: what a lost edit answers to (`journal.revert`).
pub(crate) fn lost_edit_keys(keys: &graft_ui::keymap::Keymap) -> Option<String> {
    keys.key_verb(
        &[graft_ui::keymap::ActionId("journal.revert")],
        "puts it back",
    )
}

/// `b makes a branch on a $ or ✗ row`: where `journal.branch` does work.
pub(crate) fn branch_needs_row(keys: &graft_ui::keymap::Keymap) -> Option<String> {
    keys.key_verb(
        &[graft_ui::keymap::ActionId("journal.branch")],
        "makes a branch on a $ or \u{2717} row",
    )
}
