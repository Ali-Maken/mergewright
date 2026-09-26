//! Stash put-backs over a file git doesn't keep.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::path::PathBuf;

use graft_ops::operation::{Operation, Refusal};

use super::{MINE, TempRepo, assert_refused, id, main_and_other_unignored};

// ── stashes ─────────────────────────────────────────────────────────────

fn stash_apply(repo: &TempRepo) -> Operation {
    Operation::StashApply {
        index: 0,
        commit: id(repo, "stash@{0}"),
    }
}

fn stash_words() -> Refusal {
    Refusal::StashInTheWay {
        unsaved: Vec::new(),
        staged: Vec::new(),
        untracked: vec![PathBuf::from("c.txt")],
    }
}

/// The stash holds an untracked `c.txt`; since then `c.txt` is ignored and
/// holds his bytes. The preview's words are unchanged; `apply` now refuses
/// before `begin`.
#[test]
fn stash_apply_over_an_ignored_file_is_refused_at_y() {
    let repo = main_and_other_unignored();
    repo.write("c.txt", b"stashed\n");
    repo.git(&["stash", "push", "-q", "-u"]);
    repo.write(".git/info/exclude", b"c.txt\n");
    repo.write("c.txt", MINE);
    assert_refused(
        &repo,
        &stash_apply(&repo),
        "c.txt",
        &stash_words().explain(),
    );
}

/// The stash's index adds `c.txt`; since then `c.txt` is ignored on disk.
#[test]
fn stash_whose_staged_new_file_meets_an_ignored_file() {
    let repo = main_and_other_unignored();
    repo.write("c.txt", b"staged\n");
    repo.git(&["add", "c.txt"]);
    repo.git(&["stash", "push", "-q"]);
    repo.write(".git/info/exclude", b"c.txt\n");
    repo.write("c.txt", MINE);
    assert_refused(
        &repo,
        &stash_apply(&repo),
        "c.txt",
        &stash_words().explain(),
    );
}
