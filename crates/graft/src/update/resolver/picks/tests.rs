//! The pick stack and the two "next open" rules, over hand-built documents.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use graft_core::job::JobId;
use graft_git::conflict::{ConflictFile, Take};
use graft_ops::conflict::Restore;

use super::{BlockAt, Pick, Picks, Reread, blocks_of, next_open, next_open_in_file};

const TWO: &[u8] = b"one\n\
    <<<<<<< HEAD\nours a\n=======\ntheirs a\n>>>>>>> topic\n\
    two\n\
    <<<<<<< HEAD\nours b\n=======\ntheirs b\n>>>>>>> topic\n";

fn two() -> ConflictFile {
    graft_git::conflict::parse(TWO).expect("two open blocks")
}

fn took(mut file: ConflictFile, block: usize, take: Take) -> ConflictFile {
    file.take(block, take).expect("the block exists");
    file
}

fn open(index: usize) -> BlockAt {
    BlockAt { index, open: true }
}

fn taken(index: usize) -> BlockAt {
    BlockAt { index, open: false }
}

fn pick(path: &str, block: usize) -> Pick {
    let was = two();
    let after = took(was.clone(), block, Take::Ours);
    Pick::between(Path::new(path), &was, Arc::new(after)).expect("a block changed")
}

// ------------------------------------------------------------ the rules

#[test]
fn next_open_in_file_is_the_first_open_top_down() {
    assert_eq!(next_open_in_file(&[taken(0), open(1), open(2)]), Some(1));
    // Not "after the one just taken": the first, wherever the cursor was.
    assert_eq!(next_open_in_file(&[open(0), taken(1), open(2)]), Some(0));
    assert_eq!(next_open_in_file(&[taken(0), taken(1)]), None);
    assert_eq!(next_open_in_file(&[]), None);
}

#[test]
fn next_open_is_the_first_open_in_list_order() {
    let files = vec![
        (PathBuf::from("a.txt"), vec![taken(0), taken(1)]),
        (PathBuf::from("b.txt"), vec![taken(0), open(1)]),
        (PathBuf::from("c.txt"), vec![open(0)]),
    ];
    assert_eq!(next_open(&files), Some((1, 1)), "taken blocks skipped");
    let none = vec![(PathBuf::from("a.txt"), vec![taken(0)])];
    assert_eq!(next_open(&none), None);
}

#[test]
fn blocks_of_reads_the_document() {
    let file = took(two(), 1, Take::Theirs);
    assert_eq!(blocks_of(&file), vec![open(0), taken(1)]);
}

// ------------------------------------------------------------- a pick

#[test]
fn a_pick_remembers_each_block_as_it_was() {
    let was = took(two(), 0, Take::Ours);
    let landed = took(was.clone(), 0, Take::Theirs);
    let pick = Pick::between(Path::new("f"), &was, Arc::new(landed)).expect("changed");
    assert_eq!(
        pick.blocks,
        vec![Restore {
            block: 0,
            before: Some(Take::Ours)
        }],
        "a re-take remembers the earlier take, not open"
    );

    let mut all = two();
    all.take_all(Take::Ours);
    let pick = Pick::between(Path::new("f"), &two(), Arc::new(all)).expect("changed");
    assert_eq!(
        pick.blocks,
        vec![
            Restore {
                block: 0,
                before: None
            },
            Restore {
                block: 1,
                before: None
            }
        ],
        "one pick for the whole keypress"
    );
    assert_eq!(pick.first_block(), Some(0));
}

#[test]
fn the_same_side_again_is_no_pick() {
    let was = took(two(), 0, Take::Ours);
    let again = took(was.clone(), 0, Take::Ours);
    assert_eq!(Pick::between(Path::new("f"), &was, Arc::new(again)), None);
}

// ------------------------------------------------------------ the stack

#[test]
fn seal_drops_one_file_and_keeps_the_order_of_the_rest() {
    let mut picks = Picks::default();
    picks.push(pick("a", 0));
    picks.push(pick("b", 0));
    picks.push(pick("a", 1));
    assert_eq!(picks.in_file(Path::new("a")), 2);
    picks.seal(Path::new("a"));
    assert_eq!(picks.in_file(Path::new("a")), 0);
    assert_eq!(picks.last().map(|pick| pick.path.clone()), Some("b".into()));
    picks.clear();
    assert!(picks.is_empty());
}

/// `was` with `block` taken, fingerprinted as the take's write leaves it.
fn written(was: &ConflictFile, block: usize) -> ConflictFile {
    let mut after = took(was.clone(), block, Take::Ours);
    after.fingerprint = graft_git::conflict::blob_id(&after.render());
    after
}

/// The pick taking `block` of `was` in `path`, as a landed take records it.
fn written_pick(path: &str, was: &ConflictFile, block: usize) -> Pick {
    Pick::between(Path::new(path), was, Arc::new(written(was, block))).expect("a block changed")
}

/// The file read back: its bytes parsed, every taken block collapsed.
fn read_back(file: &ConflictFile) -> ConflictFile {
    graft_git::conflict::parse(&file.render()).expect("a document")
}

/// Ticket ease-pass/05r (review ease-pass-05 M1 part 2): a read of the
/// bytes the newest pick in the file wrote shows that pick's document —
/// its taken clashes kept, which the read has collapsed into text.
#[test]
fn reconcile_shows_the_newest_picks_document_while_the_bytes_are_its() {
    let mut picks = Picks::default();
    let first = written_pick("a", &two(), 0);
    let second = written_pick("a", &first.after, 1);
    let (older, newest) = (Arc::clone(&first.after), Arc::clone(&second.after));
    picks.push(first);
    picks.push(pick("b", 0));
    picks.push(second);

    let read = read_back(&newest);
    assert_eq!(read.fingerprint, newest.fingerprint, "the same bytes");
    assert_eq!(read.blocks().len(), 0, "the read collapsed both takes");
    assert_eq!(
        picks.reconcile(Path::new("a"), Some(&read)),
        Reread::Picked(Arc::clone(&newest))
    );
    assert_eq!(newest.blocks().len(), 2, "taken clashes kept");
    assert_eq!(picks.in_file(Path::new("a")), 2, "nothing dropped");
    assert_eq!(read.open(), newest.open(), "the same open clashes");

    // The older pick's bytes are not the newest's: someone wrote since.
    assert_eq!(
        picks.reconcile(Path::new("a"), Some(&read_back(&older))),
        Reread::Dropped { count: 2 }
    );
}

/// Review ease-pass-05 M2: other bytes drop every pick in the file, and
/// say how many; other files keep theirs, in order.
#[test]
fn reconcile_drops_a_file_whose_bytes_are_not_its_last_picks() {
    let mut picks = Picks::default();
    picks.push(written_pick("a", &two(), 0));
    picks.push(pick("b", 0));
    let edited = graft_git::conflict::parse(b"edited\n").expect("plain text");
    assert_eq!(
        picks.reconcile(Path::new("a"), Some(&edited)),
        Reread::Dropped { count: 1 }
    );
    assert_eq!(picks.in_file(Path::new("a")), 0);
    assert_eq!(picks.in_file(Path::new("b")), 1, "other files keep theirs");
    assert_eq!(
        picks.reconcile(Path::new("b"), None),
        Reread::Dropped { count: 1 },
        "no document there now"
    );
    assert_eq!(
        picks.reconcile(Path::new("c"), None),
        Reread::NoPicks,
        "no picks, nothing to drop"
    );
    assert_eq!(
        picks.reconcile(Path::new("c"), Some(&two())),
        Reread::NoPicks
    );
    assert!(picks.is_empty());
}

#[test]
fn one_untake_out_at_a_time() {
    let mut picks = Picks::default();
    picks.push(pick("a", 0));
    picks.push(pick("a", 1));
    let id = JobId::new(7);
    let out = picks.start_undo(id).expect("a pick to undo");
    assert_eq!(out.blocks[0].block, 1, "the last pick");
    assert_eq!(picks.undoing(), Some(id));
    assert_eq!(picks.start_undo(JobId::new(8)), None, "one at a time");
    assert_eq!(picks.finish_undo(JobId::new(8)), None, "not the one out");
    assert_eq!(picks.finish_undo(id), Some(out.clone()));
    assert_eq!(picks.undoing(), None);
    picks.put_back(out);
    assert_eq!(picks.in_file(Path::new("a")), 2, "back on top");
}

#[test]
fn a_landing_is_spent_once_and_only_on_the_bytes_it_wrote() {
    let mut picks = Picks::default();
    let file = Arc::new(two());
    picks.land_after_read(Path::new("a"), Arc::clone(&file), Some(1));
    assert_eq!(picks.take_landing(Path::new("b"), file.fingerprint), None);
    assert_eq!(
        picks.take_landing(Path::new("a"), file.fingerprint),
        Some((Arc::clone(&file), Some(1)))
    );
    assert_eq!(picks.take_landing(Path::new("a"), file.fingerprint), None);
    picks.land_after_read(Path::new("a"), Arc::clone(&file), Some(1));
    let other = graft_git::conflict::blob_id(b"other\n");
    assert_eq!(picks.take_landing(Path::new("a"), other), None);
}
