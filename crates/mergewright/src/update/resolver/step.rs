//! `←`/`→` (`h`/`l`): the previous / next clash **in the file on screen**,
//! open or taken (ticket ease-pass/05r, review ease-pass-05 M1). §2.2's
//! movement keys, so movement only: no take, no write, never another file —
//! `j`/`k` cross files, `n` finds the first open clash, `N` walks back and
//! crosses. Clamped at the file's first and last clash, and said there.

use mergewright_core::event::{KeyCode, KeyPress};
use mergewright_core::job::JobId;

use super::picks::{BlockAt, blocks_of};
use super::take::pending_file;
use super::walk::{first_open, move_cursor, no_block, say, walk_blocks};
use crate::update::events::Command;
use crate::update::state::AppState;

/// The keys that move the cursor from clash to clash: `n` the first open
/// one ([`first_open`]), `N` the previous block then the previous file
/// ([`walk_blocks`]), `←`/`→` (`h`/`l`) this file's own ([`step_clash`]).
/// `false` for any other key, which `act` then tries on.
pub(super) fn clash_keys(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    key: KeyPress,
    next_id: &mut impl FnMut() -> JobId,
) -> bool {
    if key.modifiers.control || key.modifiers.alt {
        return false;
    }
    match key.code {
        KeyCode::Char('n') => first_open(state, commands, next_id),
        KeyCode::Char('N') => walk_blocks(state, commands, false, next_id),
        KeyCode::Char('h') | KeyCode::Left => step_clash(state, false),
        KeyCode::Char('l') | KeyCode::Right => step_clash(state, true),
        _ => return false,
    }
    true
}

/// `←` (`forward = false`) or `→`: the cursor onto the clash before or
/// after it in this file, open or taken, landing as `N` lands — on the
/// block, scrolled into view. A done file never gets here (`act`'s done
/// gate words it). The document walked is the one `N` walks: as it will
/// stand once every take in flight has landed.
fn step_clash(state: &mut AppState, forward: bool) {
    let Some(open) = state.resolver.as_ref() else {
        return;
    };
    let name = open.path.display().to_string();
    let blocks = match pending_file(open) {
        Some(Ok(file)) => blocks_of(&file),
        // A take in flight names a block the document lacks: `N`'s words.
        Some(Err(missing)) => {
            if let Some(open) = state.resolver.as_mut() {
                open.notice = Some((no_block(missing.0), true));
            }
            return;
        }
        // A whole-file card: one choice, no clashes.
        None if open.whole_file.is_some() => Vec::new(),
        // The opening read is still out: nothing yet to step through.
        None => return,
    };
    if blocks.is_empty() {
        say(state, &format!("{name} has no clashes to step through"));
        return;
    }
    let target = match open.cursor_block {
        Some(at) => step_in_file(&blocks, at, forward),
        // No block under the cursor: come in from the file's edge.
        None if forward => blocks.first().map(|block| block.index),
        None => blocks.last().map(|block| block.index),
    };
    if let Some(block) = target {
        move_cursor(state, block, None);
    } else {
        let end = if forward { "last" } else { "first" };
        say(state, &format!("{end} clash in {name}"));
    }
}

/// The block before (`forward = false`) or after `at`, open or taken;
/// `None` at the file's edge.
#[must_use]
pub fn step_in_file(blocks: &[BlockAt], at: usize, forward: bool) -> Option<usize> {
    let indices = blocks.iter().map(|block| block.index);
    if forward {
        indices.filter(|index| *index > at).min()
    } else {
        indices.filter(|index| *index < at).max()
    }
}
