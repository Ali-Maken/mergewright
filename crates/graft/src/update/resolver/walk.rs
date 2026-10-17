//! `n`: the first open clash of the whole operation, from the top
//! (ease-pass 05, [`first_open`]). `N`: the previous block, taken or not,
//! then the previous file in the list (m6-conflicts/15, unchanged — the
//! walk below is written for both directions).
//!
//! Before the first block, one rule decides, by position only (keymap
//! §2.6, the user's call of 2026-09-24):
//! 1. another file in the list not done → cross to it;
//! 2. else this file has an open block → wrap to it;
//! 3. else `no more conflicts this way`, a notice ([`NO_MORE_THIS_WAY`]);
//!    the bar keeps offering `⏎ done`.
//!
//! Crossing first is what keeps `N` from circling a file that still has an
//! open block and never leaving it.

use std::path::PathBuf;

use graft_core::job::JobId;
use graft_git::conflict::{BlockState, ConflictFile};

use super::files::{RowState, listed_files, select_file};
use super::picks::{BlockAt, blocks_of, next_open};
use super::scrolled_to;
use super::take::pending_file;
use crate::update::events::Command;
use crate::update::state::AppState;

/// Rule 3's words: nothing open anywhere this way (m6-conflicts/61). An info
/// notice on the notice row, never the status line, so the bar keeps its
/// own hints — `⏎ done` among them. `n` says it too, with nothing open
/// anywhere (05b gives a merge its own words).
pub(crate) const NO_MORE_THIS_WAY: &str = "no more conflicts this way";

/// `n` with the cursor already where `n` goes: said, an info notice, so the
/// bar's `n` is never a dead key (the mock's `n` does nothing there).
pub(crate) const ON_FIRST_OPEN: &str = "already on the first open clash";

/// `n`: the first open clash of the whole operation, from the top of the
/// list (ease-pass 05, the mock's `nextOpen`) — taken blocks and finished
/// files skipped. In the file on screen the cursor moves to it; another
/// file is opened, and its fresh read lands on its first block, which a
/// fresh read always has open (taken blocks are plain text on disk). A
/// whole-file card, or a file not read yet, counts as open: there is a
/// choice in it to make. Already there: [`ON_FIRST_OPEN`]. Nothing open
/// anywhere: [`NO_MORE_THIS_WAY`].
pub(super) fn first_open(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    next_id: &mut impl FnMut() -> JobId,
) {
    let Some(open) = state.resolver.as_ref() else {
        return;
    };
    let here = match pending_file(open) {
        Some(Ok(file)) if open.done.is_none() => blocks_of(&file),
        // The take in flight names a block the document lacks: a take's
        // words, and no walk over a document that cannot be what is on disk.
        Some(Err(missing)) => {
            if let Some(open) = state.resolver.as_mut() {
                open.notice = Some((no_block(missing.0), true));
            }
            return;
        }
        _ if open.done.is_none() && open.whole_file.is_some() => vec![ANY_OPEN],
        // Done, or its opening read still out: nothing here to go to.
        _ => Vec::new(),
    };
    let (path, card, cursor) = (
        open.path.clone(),
        open.whole_file.is_some(),
        open.cursor_block,
    );
    let listed = listed_files(state);
    let files: Vec<(PathBuf, Vec<BlockAt>)> = listed
        .iter()
        .map(|row| {
            let blocks = if row.path == path {
                here.clone()
            } else {
                row_blocks(&row.state)
            };
            (row.path.clone(), blocks)
        })
        .collect();
    match next_open(&files) {
        Some((index, _)) if listed[index].path != path => {
            cross(state, commands, index, true, next_id);
        }
        // A card has no block for the cursor: it is already where `n` goes.
        Some(_) if card => say(state, ON_FIRST_OPEN),
        Some((_, block)) if cursor == Some(block) => say(state, ON_FIRST_OPEN),
        Some((_, block)) => move_cursor(state, block, None),
        None => no_more_conflicts(state),
    }
}

/// A file not on screen, as `n` counts it: one open block where anything
/// is left to choose, none where nothing is.
fn row_blocks(row: &RowState) -> Vec<BlockAt> {
    match row {
        RowState::Open { open, .. } if *open > 0 => vec![ANY_OPEN],
        RowState::Open { .. } | RowState::AllTaken | RowState::Done => Vec::new(),
        RowState::WholeFile { .. } | RowState::Binary | RowState::Unread => vec![ANY_OPEN],
    }
}

/// A file's first block, open — where its fresh read lands the cursor.
const ANY_OPEN: BlockAt = BlockAt {
    index: 0,
    open: true,
};

/// `n`/`N`: the next or previous block, then the three rules above.
pub(super) fn walk_blocks(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    forward: bool,
    next_id: &mut impl FnMut() -> JobId,
) {
    // A done file has no blocks: it is past its own end, so rule 1 only,
    // and nothing to mark done in rule 3 (m6-conflicts/17 D6).
    if state
        .resolver
        .as_ref()
        .is_some_and(|open| open.done.is_some())
    {
        cross_from_done(state, commands, forward, next_id);
        return;
    }
    let (pending, card, cursor) = {
        let Some(open) = state.resolver.as_ref() else {
            return;
        };
        (
            pending_file(open),
            open.whole_file.is_some(),
            open.cursor_block,
        )
    };
    // With neither a document nor a card the opening read is still in
    // flight, and there is nothing yet to walk from.
    let file = match pending {
        Some(Ok(file)) => Some(file),
        // The document lacks a block a take in flight names: the same
        // words a take gets for it, and no walk over a document that
        // cannot be what is on disk.
        Some(Err(missing)) => {
            if let Some(open) = state.resolver.as_mut() {
                open.notice = Some((no_block(missing.0), true));
            }
            return;
        }
        None if card => None,
        None => return,
    };
    // A whole-file card has no blocks, so it is at its own end straight
    // away: rule 1 only (m6-05 review, Major 1).
    if let Some(block) = file
        .as_ref()
        .and_then(|file| next_block(file, cursor, forward))
    {
        move_cursor(state, block, None);
        return;
    }
    if let Some(index) = cross_target(state, forward) {
        cross(state, commands, index, forward, next_id);
        return;
    }
    if let Some((block, left)) = file.as_ref().and_then(|file| wrap_target(file, forward)) {
        move_cursor(state, block, Some(format!("wrapped · {left} still open")));
        return;
    }
    no_more_conflicts(state);
}

/// Rule 3: nothing open anywhere — say so on the notice row. The status
/// line is not touched, so the bar under the document keeps its own hints:
/// `⏎ done` on a document, the side keys on a card (m6-conflicts/61). Once
/// every file is done the finish row takes the notice row instead
/// (`view::resolver::finish_row`), and it already names the next key.
fn no_more_conflicts(state: &mut AppState) {
    say(state, NO_MORE_THIS_WAY);
}

/// `words` on the notice row, as information.
fn say(state: &mut AppState, words: &str) {
    if let Some(open) = state.resolver.as_mut() {
        open.notice = Some((words.to_owned(), false));
    }
}

/// `n`/`N` on a done file: cross to the next (previous) file not done, or
/// say there is none (m6-conflicts/17 D6).
fn cross_from_done(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    forward: bool,
    next_id: &mut impl FnMut() -> JobId,
) {
    match cross_target(state, forward) {
        Some(index) => cross(state, commands, index, forward, next_id),
        None => no_more_conflicts(state),
    }
}

/// Rule 2: this file's first (`N`: last) open block, and how many are
/// open, or `None` when none is.
fn wrap_target(file: &ConflictFile, forward: bool) -> Option<(usize, usize)> {
    let open = file
        .blocks()
        .into_iter()
        .filter(|at| at.state == BlockState::Open)
        .map(|at| at.index);
    let block = if forward { open.min() } else { open.max() }?;
    Some((block, file.open()))
}

/// The cursor onto `block`, scrolled into view, with `notice` under the
/// title (`None` clears it).
fn move_cursor(state: &mut AppState, block: usize, notice: Option<String>) {
    let top = scrolled_to(state, block);
    if let Some(open) = state.resolver.as_mut() {
        open.cursor_block = Some(block);
        open.top = top;
        open.notice = notice.map(|words| (words, false));
    }
}

/// Open row `index` of the list: `n` lands on its first block, `N` on its
/// last. A refusal (a take in flight, a box up) is `select_file`'s, in its
/// words, and leaves this file open.
fn cross(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    index: usize,
    forward: bool,
    next_id: &mut impl FnMut() -> JobId,
) {
    let was = state.resolver.as_ref().map(|open| open.path.clone());
    let Some(target) = listed_files(state).into_iter().nth(index) else {
        return;
    };
    let spent = select_file(state, commands, index, next_id);
    debug_assert!(spent, "index came from listed_files");
    let Some(open) = state.resolver.as_mut() else {
        return;
    };
    if Some(&open.path) == was.as_ref() {
        // Refused: the resolver is as it was, and the status says why.
        return;
    }
    open.land_last = !forward;
    open.notice = Some((format!("▸ {}", target.path.display()), false));
}

/// The words a take or a walk gets when the document has no block `index`.
pub(super) fn no_block(index: usize) -> String {
    format!("no block {index} · re-read the file")
}

/// The next block from `from` in `forward`'s direction, **taken or not**,
/// or `None` past the end. `from: None` starts outside the file, so it
/// answers the first (or last) block.
#[must_use]
pub fn next_block(file: &ConflictFile, from: Option<usize>, forward: bool) -> Option<usize> {
    let count = file.blocks().len();
    match (from, forward) {
        (_, _) if count == 0 => None,
        (None, true) => Some(0),
        (None, false) => Some(count - 1),
        (Some(cursor), true) => cursor.checked_add(1).filter(|next| *next < count),
        (Some(cursor), false) => cursor
            .checked_sub(1)
            .map(|previous| previous.min(count - 1)),
    }
}

/// The `listed_files` index `n` (or `N`) crosses to: the next row not done
/// after the open file's, in the list's order and wrapping — never the
/// open file itself.
pub(super) fn cross_target(state: &AppState, forward: bool) -> Option<usize> {
    let open = state.resolver.as_ref()?;
    let listed = listed_files(state);
    let count = listed.len();
    let at = listed.iter().position(|file| file.path == open.path)?;
    (1..count)
        .map(|step| {
            if forward {
                (at + step) % count
            } else {
                (at + count - step) % count
            }
        })
        .find(|index| listed[*index].state != RowState::Done)
}
