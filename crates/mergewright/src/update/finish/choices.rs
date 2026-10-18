//! Recording what the user chose per conflicted file, for the summary's
//! `RESOLVED BY YOU` words (m6-conflicts/19 scope 3, built by 35).
//!
//! Written by the resolver's answers as they land — never by the key that
//! asked — so a take that failed, or was dropped with its queue, records
//! nothing. The whole map goes when the operation ends
//! (`undo::operation_ended`).

use std::path::Path;

use mergewright_git::conflict::{BlockState, ConflictFile, Take};
use mergewright_ops::operation::Side;

use crate::update::events::TakeAt;
use crate::update::state::AppState;

/// A block take landed on `path`. `was` is the document the request was
/// applied to, `landed` the one that came back.
///
/// `Block(i)` records `i`; `Every` records each block `was` had open and
/// `landed` took — the blocks this write changed, not ones taken earlier.
/// `takes` is sized to the document's block count on first write; the
/// count never changes under a take (a taken block stays a block).
pub(crate) fn record_take(
    state: &mut AppState,
    path: &Path,
    was: Option<&ConflictFile>,
    landed: &ConflictFile,
    at: TakeAt,
    take: Take,
) {
    let blocks = landed.blocks();
    let entry = state.choices.entry(path.to_path_buf()).or_default();
    if entry.takes.len() != blocks.len() {
        entry.takes.resize(blocks.len(), None);
    }
    match at {
        TakeAt::Block(index) => {
            if let Some(slot) = entry.takes.get_mut(index) {
                *slot = Some(take);
            }
        }
        TakeAt::Every => {
            // No document before (none had landed yet) means no block was
            // open in it: nothing is recorded, which is the truth.
            let before = was.map(ConflictFile::blocks).unwrap_or_default();
            for (index, now) in blocks.iter().enumerate() {
                let was_open = before
                    .get(index)
                    .is_some_and(|block| block.state == BlockState::Open);
                if let (true, BlockState::Took(took)) = (was_open, now.state) {
                    entry.takes[index] = Some(took);
                }
            }
        }
    }
}

/// A whole-file `o`/`t` landed: the file is one side, whole.
pub(crate) fn record_whole(state: &mut AppState, path: &Path, side: Side) {
    state.choices.entry(path.to_path_buf()).or_default().whole = Some(side);
}

/// `$EDITOR` or the mergetool had `path` and gave the terminal back cleanly.
pub(crate) fn record_edited(state: &mut AppState, path: &Path) {
    state.choices.entry(path.to_path_buf()).or_default().edited = true;
}

/// `d` started `path` over: it is conflicted afresh, and nothing chosen
/// before still describes it.
pub(crate) fn forget(state: &mut AppState, path: &Path) {
    state.choices.remove(path);
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use mergewright_git::conflict::{ConflictFile, Take, parse};

    use super::*;
    use crate::update::finish::Choices;

    const TWO_BLOCKS: &[u8] = b"a\n<<<<<<< HEAD\nours 1\n=======\ntheirs 1\n>>>>>>> feature\n\
b\n<<<<<<< HEAD\nours 2\n=======\ntheirs 2\n>>>>>>> feature\n";

    fn document() -> ConflictFile {
        parse(TWO_BLOCKS).expect("two blocks parse")
    }

    fn recorded(state: &AppState) -> Option<&Choices> {
        state.choices.get(Path::new("notes.txt"))
    }

    #[test]
    fn a_block_take_records_that_block_only() {
        let mut state = AppState::default();
        let was = document();
        let mut landed = was.clone();
        landed.take(1, Take::Theirs).expect("block 1");
        record_take(
            &mut state,
            Path::new("notes.txt"),
            Some(&was),
            &landed,
            TakeAt::Block(1),
            Take::Theirs,
        );
        assert_eq!(
            recorded(&state).map(|choices| choices.takes.clone()),
            Some(vec![None, Some(Take::Theirs)])
        );
    }

    #[test]
    fn every_records_only_the_blocks_it_took() {
        let mut state = AppState::default();
        let mut was = document();
        was.take(0, Take::Ours).expect("block 0");
        record_take(
            &mut state,
            Path::new("notes.txt"),
            Some(&document()),
            &was,
            TakeAt::Block(0),
            Take::Ours,
        );
        let mut landed = was.clone();
        landed.take_all(Take::Both);
        record_take(
            &mut state,
            Path::new("notes.txt"),
            Some(&was),
            &landed,
            TakeAt::Every,
            Take::Both,
        );
        assert_eq!(
            recorded(&state).map(|choices| choices.takes.clone()),
            Some(vec![Some(Take::Ours), Some(Take::Both)]),
            "block 0 was taken before the `Every`, and keeps its own take"
        );
    }

    #[test]
    fn whole_edited_and_forget() {
        let mut state = AppState::default();
        let path = Path::new("notes.txt");
        record_whole(&mut state, path, Side::Theirs);
        record_edited(&mut state, path);
        assert_eq!(
            recorded(&state),
            Some(&Choices {
                takes: Vec::new(),
                whole: Some(Side::Theirs),
                edited: true,
            })
        );
        forget(&mut state, path);
        assert_eq!(recorded(&state), None);
    }
}
