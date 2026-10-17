//! A commit of chosen files (ease-pass/04): the scratch-index plan, the
//! choice that is exactly the staged set, and `NothingChosen`.

use std::path::PathBuf;

use graft_core::journal::HeadRef;
use graft_git::status::{Change, StatusEntry};

use super::{commit, entry, facts, lines};
use crate::keys::NoKeys;
use crate::operation::{
    Chosen, CommitParams, Include, IndexFile, Operation, Part, Refusal, RepoFacts,
};

fn whole(path: &str) -> Chosen {
    Chosen {
        path: PathBuf::from(path),
        part: Part::Whole,
    }
}

fn as_staged(path: &str) -> Chosen {
    Chosen {
        path: PathBuf::from(path),
        part: Part::StagedPart,
    }
}

fn chosen(message: &str, files: Vec<Chosen>) -> CommitParams {
    CommitParams {
        include: Include::Chosen(files),
        ..commit(message)
    }
}

/// `facts()` (staged `s.rs`, unstaged `u.rs`, untracked `n.rs`) plus a
/// staged rename `old.rs` → `moved.rs`.
fn with_rename() -> RepoFacts {
    let mut f = facts();
    f.status.entries.push(StatusEntry {
        original_path: Some(PathBuf::from("old.rs")),
        staged: Some(Change::Renamed),
        ..entry("moved.rs")
    });
    f
}

#[test]
fn a_choice_of_exactly_the_staged_set_plans_todays_one_line() {
    let f = facts();
    let op = Operation::Commit(chosen("s", vec![as_staged("s.rs")]));
    assert_eq!(lines(&op, &f), ["git commit -F -"]);
    assert_eq!(op.plan(&f)[0].index, IndexFile::Real);
    let amend = Operation::Amend(chosen("s", vec![as_staged("s.rs")]));
    assert_eq!(lines(&amend, &f), ["git commit -F - --amend"]);
}

#[test]
fn a_choice_runs_on_the_scratch_index_then_resets_the_chosen_paths() {
    let f = with_rename();
    let op = Operation::Commit(chosen("s", vec![whole("u.rs"), whole("n.rs")]));
    assert_eq!(
        lines(&op, &f),
        [
            "GIT_INDEX_FILE=<scratch index> git --literal-pathspecs reset -q -- s.rs moved.rs old.rs",
            "GIT_INDEX_FILE=<scratch index> git --literal-pathspecs add -A -- u.rs n.rs",
            "GIT_INDEX_FILE=<scratch index> git commit -F -",
            "git --literal-pathspecs reset -q -- n.rs u.rs",
        ]
    );
    let indexes: Vec<IndexFile> = op.plan(&f).iter().map(|line| line.index).collect();
    assert_eq!(
        indexes,
        [
            IndexFile::Scratch,
            IndexFile::Scratch,
            IndexFile::Scratch,
            IndexFile::Real
        ]
    );
}

#[test]
fn a_staged_part_is_not_added_and_nothing_held_skips_the_reset() {
    let f = facts();
    let op = Operation::Amend(CommitParams {
        sign_off: true,
        ..chosen("s", vec![as_staged("s.rs"), whole("u.rs")])
    });
    assert_eq!(
        lines(&op, &f),
        [
            "GIT_INDEX_FILE=<scratch index> git --literal-pathspecs add -A -- u.rs",
            "GIT_INDEX_FILE=<scratch index> git commit -F - --amend --signoff",
            "git --literal-pathspecs reset -q -- s.rs u.rs",
        ]
    );
}

#[test]
fn on_an_unborn_branch_the_held_paths_leave_the_copy_by_rm_cached() {
    let f = RepoFacts {
        head: HeadRef::Unborn("refs/heads/main".to_owned()),
        ..facts()
    };
    let op = Operation::Commit(chosen("s", vec![whole("n.rs")]));
    assert_eq!(
        lines(&op, &f),
        [
            "GIT_INDEX_FILE=<scratch index> git --literal-pathspecs rm --cached -f -q -- s.rs",
            "GIT_INDEX_FILE=<scratch index> git --literal-pathspecs add -A -- n.rs",
            "GIT_INDEX_FILE=<scratch index> git commit -F -",
            "git --literal-pathspecs reset -q -- n.rs",
        ]
    );
}

#[test]
fn the_logged_line_names_the_real_scratch_path() {
    let f = facts();
    let op = Operation::Commit(chosen("s", vec![whole("u.rs")]));
    let plan = op.plan(&f);
    let at = std::path::Path::new("/r/.git/mwr/commit-index");
    assert!(plan[0].command_line().starts_with("GIT_INDEX_FILE="));
    assert_eq!(
        plan[2].command_line_in(at),
        "GIT_INDEX_FILE=/r/.git/mwr/commit-index git commit -F -"
    );
    assert_eq!(
        plan[3].command_line_in(at),
        "git --literal-pathspecs reset -q -- u.rs"
    );
}

#[test]
fn nothing_chosen_is_refused_for_commit_and_amend() {
    let f = facts();
    for op in [
        Operation::Commit(chosen("s", vec![])),
        Operation::Amend(chosen("s", vec![])),
    ] {
        assert_eq!(op.check(&f), Err(Refusal::NothingChosen));
    }
    let empty_allowed = CommitParams {
        allow_empty: true,
        ..chosen("s", vec![])
    };
    assert_eq!(Operation::Commit(empty_allowed).check(&f), Ok(()));
    assert_eq!(
        Refusal::NothingChosen.explain(&NoKeys),
        "nothing goes in · put a file in first"
    );
    // A choice needs nothing staged: the choice is what goes in.
    let mut nothing_staged = facts();
    nothing_staged.status.entries.retain(|e| !e.is_staged());
    assert_eq!(
        Operation::Commit(chosen("s", vec![whole("u.rs")])).check(&nothing_staged),
        Ok(())
    );
    // Amend keeps its own refusals.
    let unborn = RepoFacts {
        head: HeadRef::Unborn("refs/heads/main".to_owned()),
        ..facts()
    };
    assert_eq!(
        Operation::Amend(chosen("s", vec![whole("u.rs")])).check(&unborn),
        Err(Refusal::UnbornHead)
    );
    assert_eq!(
        Operation::Amend(chosen("", vec![whole("u.rs")])).check(&f),
        Err(Refusal::EmptySubject)
    );
}
