//! `apply_untake`: a pick put back, byte for byte, against the stopped-merge
//! corpus (ease-pass 05).

use std::path::Path;

use mergewright_fixtures::{Scratch, Shape};
use mergewright_git::conflict::{BlockState, ConflictFile, Take};
use mergewright_git::repo::RepoHandle;

use super::{Restore, TakeError, apply_take, apply_take_all, apply_untake};

const PATH: &str = "three.txt";

/// The corpus stopped on a merge, `three.txt` (three open blocks) read as
/// git wrote it, and those bytes.
fn stopped() -> (Scratch, RepoHandle, ConflictFile, Vec<u8>) {
    let scratch = mergewright_fixtures::copy_into_temp(Shape::StoppedMerge);
    let repo = RepoHandle::open(scratch.path()).expect("the fixture opens");
    let full = repo.workdir().expect("a working tree").join(PATH);
    let bytes = std::fs::read(&full).expect("it reads");
    let file = mergewright_git::conflict::read(&full).expect("the file parses");
    assert_eq!(file.open(), 3, "the fixture has three open blocks");
    (scratch, repo, file, bytes)
}

fn on_disk(repo: &RepoHandle) -> Vec<u8> {
    std::fs::read(repo.workdir().expect("a working tree").join(PATH)).expect("it reads")
}

fn open_again(block: usize) -> Restore {
    Restore {
        block,
        before: None,
    }
}

#[test]
fn untake_reopens_a_block_to_the_bytes_git_wrote() {
    let (_scratch, repo, file, git_wrote) = stopped();
    let took = apply_take(&repo, Path::new(PATH), &file, 0, Take::Ours).expect("the take applies");
    assert_ne!(on_disk(&repo), git_wrote, "the take wrote");

    let back =
        apply_untake(&repo, Path::new(PATH), &took, &[open_again(0)]).expect("the untake applies");

    assert_eq!(back.blocks()[0].state, BlockState::Open);
    assert_eq!(back.open(), 3);
    assert_eq!(on_disk(&repo), git_wrote, "bytes equal the file git wrote");
    assert_eq!(
        back.fingerprint,
        mergewright_git::conflict::blob_id(&git_wrote)
    );
}

#[test]
fn untake_of_a_second_pick_leaves_the_first() {
    let (_scratch, repo, file, _) = stopped();
    let one = apply_take(&repo, Path::new(PATH), &file, 0, Take::Ours).expect("take 1");
    let after_one = on_disk(&repo);
    let two = apply_take(&repo, Path::new(PATH), &one, 1, Take::Theirs).expect("take 2");

    let back = apply_untake(&repo, Path::new(PATH), &two, &[open_again(1)]).expect("untake 2");

    assert_eq!(back.blocks()[0].state, BlockState::Took(Take::Ours));
    assert_eq!(back.blocks()[1].state, BlockState::Open);
    assert_eq!(
        on_disk(&repo),
        after_one,
        "bytes equal the file before pick 2"
    );
}

#[test]
fn untake_of_a_retake_restores_the_earlier_take() {
    let (_scratch, repo, file, git_wrote) = stopped();
    let ours = apply_take(&repo, Path::new(PATH), &file, 0, Take::Ours).expect("take ours");
    let after_ours = on_disk(&repo);
    let theirs = apply_take(&repo, Path::new(PATH), &ours, 0, Take::Theirs).expect("re-take");

    let restore = Restore {
        block: 0,
        before: Some(Take::Ours),
    };
    let back = apply_untake(&repo, Path::new(PATH), &theirs, &[restore]).expect("untake re-take");
    assert_eq!(back.blocks()[0].state, BlockState::Took(Take::Ours));
    assert_eq!(on_disk(&repo), after_ours, "kept-yours again, not open");

    let open = apply_untake(&repo, Path::new(PATH), &back, &[open_again(0)]).expect("untake");
    assert_eq!(open.blocks()[0].state, BlockState::Open);
    assert_eq!(on_disk(&repo), git_wrote);
}

#[test]
fn untake_of_a_take_all_reopens_every_block_in_one_write() {
    let (_scratch, repo, file, git_wrote) = stopped();
    let all = apply_take_all(&repo, Path::new(PATH), &file, Take::Theirs).expect("take all");
    assert_eq!(all.open(), 0);

    let restores = [open_again(0), open_again(1), open_again(2)];
    let back = apply_untake(&repo, Path::new(PATH), &all, &restores).expect("untake all");

    assert_eq!(back.open(), 3);
    assert_eq!(on_disk(&repo), git_wrote);
}

#[test]
fn untake_never_clobbers_a_changed_file() {
    let (_scratch, repo, file, _) = stopped();
    let took = apply_take(&repo, Path::new(PATH), &file, 0, Take::Ours).expect("the take applies");
    let edited = b"edited outside between the pick and u\n";
    std::fs::write(repo.workdir().expect("a working tree").join(PATH), edited).expect("written");

    match apply_untake(&repo, Path::new(PATH), &took, &[open_again(0)]) {
        Err(TakeError::Changed { now }) => {
            assert_eq!(now, mergewright_git::conflict::blob_id(edited));
        }
        other => panic!("expected Changed, got {other:?}"),
    }
    assert_eq!(on_disk(&repo), edited, "the edit survives byte for byte");
}

#[test]
fn untake_out_of_step_writes_nothing() {
    let (_scratch, repo, file, git_wrote) = stopped();
    let took = apply_take(&repo, Path::new(PATH), &file, 0, Take::Ours).expect("the take applies");
    let after = on_disk(&repo);

    // Block 1 was never taken: the document is out of step with the pick.
    match apply_untake(
        &repo,
        Path::new(PATH),
        &took,
        &[open_again(0), open_again(1)],
    ) {
        Err(TakeError::NoSuchBlock(1)) => {}
        other => panic!("expected NoSuchBlock(1), got {other:?}"),
    }
    assert_eq!(
        on_disk(&repo),
        after,
        "nothing is written, block 0 included"
    );
    assert_ne!(after, git_wrote);
}
