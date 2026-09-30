//! m7-power-tools/30q: `u` gives back his bytes, not git's re-encoding of
//! them (ADR-0052).
//!
//! Each test runs an ordinary operation over an uncommitted file git would
//! convert, then `u`, through the same `apply`/`undo` path the app uses, and
//! reads the file back off disk byte for byte, never from a returned value.
//!
//! **Not registered in `main.rs` yet, on purpose.** Committed by 30q-a as
//! R0's evidence: both tests are red on 8c3f4b1b and after 30q-a (`:44`,
//! `:64`; `u` writes the file back with CRLF endings), because the snapshot
//! now keeps the bytes but `RestoreWorktree` does not write them yet. 30q-b
//! adds `mod undo_bytes;` when they go green.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::path::PathBuf;

use graft_ops::operation::{Operation, StashParams, StashScope, Target};

use crate::support::{NOW, TempRepo, run, run_undo};

/// What `HEAD` holds at `src/a.rs`.
const COMMITTED: &[u8] = b"pub fn a() -> u32 {\n    1\n}\n";
/// His uncommitted edit (`W`): LF endings only.
const WORKING: &[u8] = b"pub fn a() -> u32 {\n    4\n}\n";

/// `core.autocrlf=true`, a committed `src/a.rs`, then `W` written over it
/// with LF endings only.
fn lf_edit_under_autocrlf() -> TempRepo {
    let repo = TempRepo::init();
    repo.git(&["config", "core.autocrlf", "true"]);
    repo.write("src/a.rs", COMMITTED);
    repo.commit_all("first");
    repo.write("src/a.rs", WORKING);
    repo
}

/// A0 (30q R0): a plain discard, then `u`.
#[test]
fn u_after_a_discard_brings_back_an_lf_file_under_autocrlf() {
    let repo = lf_edit_under_autocrlf();
    run(
        &repo,
        &Operation::Discard(Target::Paths(vec![PathBuf::from("src/a.rs")])),
        NOW,
    );
    assert_ne!(repo.read("src/a.rs"), WORKING, "the discard wrote over W");

    run_undo(&repo, NOW + 1, false).unwrap_or_else(|why| panic!("undo: {}", why.explain()));
    assert_eq!(repo.read("src/a.rs"), WORKING, "W is back, byte for byte");
}

/// A0 (30q R0): a stash push, then `u`.
#[test]
fn u_after_a_stash_brings_back_an_lf_file_under_autocrlf() {
    let repo = lf_edit_under_autocrlf();
    run(
        &repo,
        &Operation::StashPush(StashParams {
            message: Some("wip".to_owned()),
            scope: StashScope::All {
                include_untracked: false,
            },
        }),
        NOW,
    );
    assert_ne!(repo.read("src/a.rs"), WORKING, "the stash took W away");

    run_undo(&repo, NOW + 1, false).unwrap_or_else(|why| panic!("undo: {}", why.explain()));
    assert_eq!(repo.read("src/a.rs"), WORKING, "W is back, byte for byte");
}
