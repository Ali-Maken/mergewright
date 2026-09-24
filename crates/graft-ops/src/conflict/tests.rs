//! The resolver's job-side half, against the stopped-merge corpus.

use std::path::{Path, PathBuf};

use graft_fixtures::{Scratch, Shape};
use graft_git::conflict::{BlockState, ParseErrorKind, Take};
use graft_git::repo::RepoHandle;
use graft_git::status::{UnmergedHow, WorkingTreeStatus};

use super::{
    BaseError, OpenError, TakeError, apply_take, apply_take_all, base_for, mark_resolved_refusal,
};
use crate::operation::Refusal;

/// A writable copy of the corpus repository left stopped on a merge.
fn stopped() -> (Scratch, RepoHandle, WorkingTreeStatus) {
    let scratch = graft_fixtures::copy_into_temp(Shape::StoppedMerge);
    let repo = RepoHandle::open(scratch.path()).expect("the fixture opens");
    let status = graft_git::status::read(&repo, &()).expect("the status reads");
    (scratch, repo, status)
}

fn open(
    repo: &RepoHandle,
    status: &WorkingTreeStatus,
    path: &str,
) -> Result<super::Opened, OpenError> {
    super::open_conflict(repo, status, Path::new(path))
}

/// The file as it is on disk right now, re-read so the fingerprint is the
/// one a caller would hold.
fn reread(repo: &RepoHandle, path: &str) -> graft_git::conflict::ConflictFile {
    let full = repo.workdir().expect("a working tree").join(path);
    graft_git::conflict::read(&full).expect("the file parses")
}

#[test]
fn open_names_every_shape() {
    let (_scratch, repo, status) = stopped();

    // A binary `UU`: git wrote no markers, so there is nothing to parse.
    let binary = open(&repo, &status, "pic.bin").expect("pic.bin opens");
    assert!(binary.binary, "pic.bin is binary");
    assert!(binary.file.is_none());
    assert_eq!(binary.how, UnmergedHow::BothModified);

    // A whole-file shape: no markers, no file, but the shape is named.
    let deleted = open(&repo, &status, "theirs-deleted.txt").expect("it opens");
    assert_eq!(deleted.how, UnmergedHow::DeletedByThem);
    assert!(deleted.file.is_none());
    assert!(!deleted.binary);

    // A marker shape: parsed, with its blocks.
    let text = open(&repo, &status, "three.txt").expect("three.txt opens");
    assert_eq!(text.how, UnmergedHow::BothModified);
    assert_eq!(text.file.expect("parsed").blocks().len(), 3);

    // A path the status does not call unmerged.
    match open(&repo, &status, "no-such-file.txt") {
        Err(OpenError::Refused(Refusal::NotUnmerged { path })) => {
            assert_eq!(path, PathBuf::from("no-such-file.txt"));
        }
        other => panic!("expected NotUnmerged, got {other:?}"),
    }

    // Malformed markers are named with their line, never read as "no
    // conflicts here".
    let workdir = repo.workdir().expect("a working tree");
    std::fs::write(workdir.join("both.txt"), b"one\n=======\ntwo\n").expect("the file is written");
    match open(&repo, &status, "both.txt") {
        Err(OpenError::Markers(error)) => assert_eq!(error.line, 2),
        other => panic!("expected Markers, got {other:?}"),
    }
}

#[test]
fn a_longer_marker_size_never_reads_as_resolved() {
    let (_scratch, repo, status) = stopped();
    let workdir = repo.workdir().expect("a working tree");

    // `conflict-marker-size = 8`. The parser reads git's default size only,
    // so the danger is reading this as plain text — zero blocks, `open() ==
    // 0`, nothing left to resolve — and letting the markers be staged.
    let eight = b"<<<<<<<< ours\nmine\n========\nyours\n>>>>>>>> theirs\n";
    std::fs::write(workdir.join("both.txt"), eight).expect("the file is written");

    match open(&repo, &status, "both.txt") {
        Err(OpenError::Markers(error)) => {
            assert_eq!(error.what, ParseErrorKind::MarkerSize);
            assert_eq!(error.line, 1);
        }
        other => panic!("expected MarkerSize, got {other:?}"),
    }
}

#[test]
fn take_never_clobbers_an_edit() {
    let (_scratch, repo, status) = stopped();
    let opened = open(&repo, &status, "three.txt").expect("it opens");
    let file = opened.file.expect("parsed");

    // `$EDITOR` gets there first.
    let workdir = repo.workdir().expect("a working tree");
    let edited = b"a hand edit, nothing like what was there\n";
    std::fs::write(workdir.join("three.txt"), edited).expect("the edit is written");

    match apply_take(&repo, Path::new("three.txt"), &file, 0, Take::Ours) {
        Err(TakeError::Changed { now }) => {
            assert_eq!(now, graft_git::conflict::blob_id(edited));
        }
        other => panic!("expected Changed, got {other:?}"),
    }
    assert_eq!(
        std::fs::read(workdir.join("three.txt")).expect("it reads"),
        edited,
        "the edit survives byte for byte"
    );
}

#[test]
fn take_all_never_clobbers_a_changed_file() {
    let (_scratch, repo, status) = stopped();
    let file = open(&repo, &status, "three.txt")
        .expect("it opens")
        .file
        .expect("parsed");
    let workdir = repo.workdir().expect("a working tree");
    let edited = b"edited outside\n";
    std::fs::write(workdir.join("three.txt"), edited).expect("the edit is written");
    match apply_take_all(&repo, Path::new("three.txt"), &file, Take::Theirs) {
        Err(TakeError::Changed { now }) => {
            assert_eq!(now, graft_git::conflict::blob_id(edited));
        }
        other => panic!("expected Changed, got {other:?}"),
    }
    assert_eq!(
        std::fs::read(workdir.join("three.txt")).expect("it reads"),
        edited
    );
}

#[test]
fn apply_take_all_after_a_take_keeps_it() {
    let (_scratch, repo, status) = stopped();
    let file = open(&repo, &status, "three.txt")
        .expect("it opens")
        .file
        .expect("parsed");
    let mut expected = file.clone();
    for (index, take) in [(0, Take::Ours), (1, Take::Theirs), (2, Take::Theirs)] {
        expected.take(index, take).expect("the block exists");
    }
    let landed =
        apply_take(&repo, Path::new("three.txt"), &file, 0, Take::Ours).expect("the take applies");
    let all = apply_take_all(&repo, Path::new("three.txt"), &landed, Take::Theirs)
        .expect("the bulk take applies");
    assert_eq!(all.render(), expected.render());
    assert_eq!(all.blocks().len(), 3);
    assert_eq!(reread(&repo, "three.txt").render(), expected.render());
}

#[test]
fn retake_replaces() {
    let (_scratch, repo, status) = stopped();
    let opened = open(&repo, &status, "three.txt").expect("it opens");
    let file = opened.file.expect("parsed");
    let before = file.blocks().len();

    let both = apply_take(&repo, Path::new("three.txt"), &file, 1, Take::Both)
        .expect("the first take applies");
    assert_eq!(both.blocks()[1].state, BlockState::Took(Take::Both));

    let ours = apply_take(&repo, Path::new("three.txt"), &both, 1, Take::Ours)
        .expect("the second take applies");
    assert_eq!(
        ours.blocks()[1].state,
        BlockState::Took(Take::Ours),
        "the second take replaces the first"
    );
    assert_eq!(ours.blocks().len(), before, "no block was added");
    assert_eq!(ours.open(), before - 1, "one block is resolved, not two");

    // And the bytes on disk are the document's rendering: a taken block is
    // its taken lines, so re-reading the path finds one conflict fewer.
    let on_disk = reread(&repo, "three.txt");
    assert_eq!(on_disk.render(), ours.render());
    assert_eq!(on_disk.blocks().len(), before - 1);
    assert_eq!(on_disk.fingerprint, ours.fingerprint);
}

#[test]
fn take_names_a_block_that_is_not_there() {
    let (_scratch, repo, status) = stopped();
    let opened = open(&repo, &status, "three.txt").expect("it opens");
    let file = opened.file.expect("parsed");
    match apply_take(&repo, Path::new("three.txt"), &file, 9, Take::Ours) {
        Err(TakeError::NoSuchBlock(9)) => {}
        other => panic!("expected NoSuchBlock(9), got {other:?}"),
    }
}

#[test]
fn base_for_is_equality_or_nothing() {
    let (_scratch, repo, status) = stopped();

    // Three blocks, all three given a base.
    let mut three = open(&repo, &status, "three.txt")
        .expect("it opens")
        .file
        .expect("parsed");
    assert_eq!(
        base_for(&repo, Path::new("three.txt"), &mut three).expect("the base attaches"),
        3
    );

    // `AA` has no stage 1: there is no common ancestor to show.
    let mut added = open(&repo, &status, "both-added.txt")
        .expect("it opens")
        .file
        .expect("parsed");
    match base_for(&repo, Path::new("both-added.txt"), &mut added) {
        Err(BaseError::NoBase) => {}
        other => panic!("expected NoBase, got {other:?}"),
    }

    // A hand edit inside a block: the honest answer is "unavailable", never
    // a base lined up by position.
    let workdir = repo.workdir().expect("a working tree");
    let original = std::fs::read(workdir.join("three.txt")).expect("it reads");
    let text = String::from_utf8(original).expect("the fixture is utf-8");
    let hand_edited = hand_edit_first_ours_line(&text);
    std::fs::write(workdir.join("three.txt"), hand_edited.as_bytes()).expect("written");
    let mut touched = graft_git::conflict::read(&workdir.join("three.txt")).expect("it parses");
    match base_for(&repo, Path::new("three.txt"), &mut touched) {
        Err(BaseError::Mismatch(mismatch)) => assert_eq!(mismatch.block, 0),
        other => panic!("expected Mismatch, got {other:?}"),
    }
}

/// Append a word to the first line after the first `<<<<<<<` marker: an edit
/// inside a block's ours side, which is what makes the base unattachable.
fn hand_edit_first_ours_line(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 16);
    let mut seen_marker = false;
    let mut edited = false;
    for line in text.split_inclusive('\n') {
        if !edited && seen_marker {
            out.push_str(line.trim_end_matches('\n'));
            out.push_str(" EDITED\n");
            edited = true;
            continue;
        }
        if line.starts_with("<<<<<<<") {
            seen_marker = true;
        }
        out.push_str(line);
    }
    assert!(edited, "the fixture has a block to edit");
    out
}

#[test]
fn mark_resolved_refusal_counts_open() {
    let (_scratch, repo, status) = stopped();
    let path = Path::new("three.txt");
    let file = open(&repo, &status, "three.txt")
        .expect("it opens")
        .file
        .expect("parsed");
    assert_eq!(file.open(), 3);

    let one = apply_take(&repo, path, &file, 0, Take::Ours).expect("take 1");
    match mark_resolved_refusal(path, &one) {
        Some(Refusal::ConflictsOpen { path: named, open }) => {
            assert_eq!(named, PathBuf::from("three.txt"));
            assert_eq!(open, 2);
        }
        other => panic!("expected ConflictsOpen {{ open: 2 }}, got {other:?}"),
    }

    let two = apply_take(&repo, path, &one, 1, Take::Theirs).expect("take 2");
    let three = apply_take(&repo, path, &two, 2, Take::Both).expect("take 3");
    assert_eq!(mark_resolved_refusal(path, &three), None);
}

// ------------------------------------------------ a done file, read-only
// (m6-conflicts/25)

/// `git <args>` in the fixture's working tree, which must succeed.
fn git(repo: &RepoHandle, args: &[&str]) {
    let workdir = repo.workdir().expect("a working tree");
    let out = std::process::Command::new("git")
        .current_dir(workdir)
        .args(args)
        .output()
        .expect("git runs");
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

fn status_of(repo: &RepoHandle) -> WorkingTreeStatus {
    graft_git::status::read(repo, &()).expect("the status reads")
}

fn open_done(
    repo: &RepoHandle,
    status: &WorkingTreeStatus,
    path: &str,
) -> Result<super::DoneFile, OpenError> {
    super::open_done(repo, status, Path::new(path))
}

#[test]
fn open_done_reads_the_file_on_disk() {
    let (_scratch, repo, _) = stopped();
    let workdir = repo.workdir().expect("a working tree");
    std::fs::write(workdir.join("both.txt"), b"settled\n").expect("the file is written");
    git(&repo, &["add", "both.txt"]);
    match open_done(&repo, &status_of(&repo), "both.txt").expect("it opens") {
        super::DoneFile::Text { file, unstaged } => {
            assert_eq!(file.open(), 0);
            assert!(file.blocks().is_empty());
            assert!(!unstaged, "what is on disk is what is staged");
        }
        other => panic!("expected Text, got {other:?}"),
    }
    std::fs::write(workdir.join("both.txt"), b"settled, then edited\n")
        .expect("the file is written");
    match open_done(&repo, &status_of(&repo), "both.txt").expect("it opens") {
        super::DoneFile::Text { unstaged, .. } => assert!(unstaged, "edited since done"),
        other => panic!("expected Text, got {other:?}"),
    }
}

#[test]
fn open_done_on_a_removed_path_is_removed() {
    let (_scratch, repo, _) = stopped();
    git(&repo, &["rm", "--quiet", "ours-deleted.txt"]);
    assert_eq!(
        open_done(&repo, &status_of(&repo), "ours-deleted.txt").expect("it opens"),
        super::DoneFile::Removed
    );
}

#[test]
fn open_done_on_a_binary_is_binary() {
    let (_scratch, repo, _) = stopped();
    git(&repo, &["add", "pic.bin"]);
    assert_eq!(
        open_done(&repo, &status_of(&repo), "pic.bin").expect("it opens"),
        super::DoneFile::Binary { unstaged: false }
    );
}

#[test]
fn open_done_refuses_an_unmerged_path() {
    let (_scratch, repo, status) = stopped();
    match open_done(&repo, &status, "three.txt") {
        Err(OpenError::Refused(Refusal::Unmerged { paths })) => {
            assert_eq!(paths, vec![PathBuf::from("three.txt")]);
        }
        other => panic!("expected Unmerged, got {other:?}"),
    }
}
