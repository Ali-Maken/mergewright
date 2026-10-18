//! `file_state`: one row per `UnmergedHow`, and the three ways a marker
//! document stands.

use mergewright_git::conflict::{ConflictFile, Take, parse};
use mergewright_git::status::UnmergedHow;

use super::{FileState, file_state};
use crate::conflict::Opened;

const TWO_CLASHES: &[u8] =
    b"top\n<<<<<<< ours\na\n=======\nb\n>>>>>>> theirs\nmid\n<<<<<<< ours\nc\n=======\nd\n>>>>>>> theirs\nend\n";

fn document(bytes: &[u8]) -> ConflictFile {
    parse(bytes).expect("the document parses")
}

fn opened(file: Option<ConflictFile>, how: UnmergedHow, binary: bool) -> Opened {
    Opened { file, how, binary }
}

#[test]
fn a_marker_document_is_open_until_every_clash_is_picked_then_solved() {
    let mut file = document(TWO_CLASHES);
    let how = UnmergedHow::BothModified;
    assert_eq!(
        file_state(&opened(Some(file.clone()), how, false), true),
        FileState::Open { open: 2 }
    );
    file.take(0, Take::Ours).expect("block 0 exists");
    assert_eq!(
        file_state(&opened(Some(file.clone()), how, false), true),
        FileState::Open { open: 1 }
    );
    file.take(1, Take::Theirs).expect("block 1 exists");
    assert_eq!(
        file_state(&opened(Some(file), how, false), true),
        FileState::Solved
    );
}

#[test]
fn a_file_whose_markers_were_edited_away_is_solved_while_unmerged() {
    let file = document(b"top\nmerged by hand\nend\n");
    for how in [UnmergedHow::BothModified, UnmergedHow::BothAdded] {
        assert_eq!(
            file_state(&opened(Some(file.clone()), how, false), true),
            FileState::Solved,
            "{how:?}"
        );
    }
}

#[test]
fn every_whole_file_shape_and_a_binary_conflict_is_open_with_no_markers() {
    for how in [
        UnmergedHow::BothDeleted,
        UnmergedHow::AddedByUs,
        UnmergedHow::AddedByThem,
        UnmergedHow::DeletedByUs,
        UnmergedHow::DeletedByThem,
    ] {
        assert_eq!(
            file_state(&opened(None, how, false), true),
            FileState::Open { open: 0 },
            "{how:?}"
        );
    }
    for how in [UnmergedHow::BothModified, UnmergedHow::BothAdded] {
        assert_eq!(
            file_state(&opened(None, how, true), true),
            FileState::Open { open: 0 },
            "binary {how:?}"
        );
    }
}

#[test]
fn a_path_git_no_longer_calls_unmerged_is_done_whatever_its_bytes() {
    let open = document(TWO_CLASHES);
    for opened in [
        opened(Some(open), UnmergedHow::BothModified, false),
        opened(None, UnmergedHow::DeletedByThem, false),
    ] {
        assert_eq!(file_state(&opened, false), FileState::Done);
    }
}

#[test]
fn open_clashes_counts_a_whole_file_card_as_one() {
    assert_eq!(FileState::Open { open: 3 }.open_clashes(), 3);
    assert_eq!(FileState::Open { open: 0 }.open_clashes(), 1);
    assert_eq!(FileState::Solved.open_clashes(), 0);
    assert_eq!(FileState::Done.open_clashes(), 0);
}
