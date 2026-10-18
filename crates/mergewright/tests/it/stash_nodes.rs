//! Stash entries' nodes through the real app (ease-pass 06r M1): the
//! newest entry, though git gives it three parents (its index and its
//! untracked files), draws the stash glyph, never a merge's `◉`. An older
//! entry is not a row here: the walk's tips are refs and `stash@{1}` lives
//! only in the reflog (`stash_in_history::which_stash_entries_are_drawn`).
//! Its `●` is `view::node::tests`' to pin.
#![allow(
    clippy::expect_used,
    reason = "a fixture that cannot be built has nothing to test; the panic names why"
)]

use mergewright::session::Clock;
use mergewright::update::AppState;

use crate::app_harness::App;
use crate::captures::frame_at;

const COLUMNS: u16 = 120;
const ROWS: u16 = 36;
/// After every commit and stash of [`two_stashes`], so ages read the same.
const NOW: i64 = 1_700_100_000;
/// `stash@{1}`'s message.
const OLDER: &str = "older";
/// `stash@{0}`'s message: pushed last, with an untracked file.
const NEWEST: &str = "newest, with an untracked file";

/// Two commits on `main`, then two stash entries on top: `stash@{1}` over
/// an edit (two parents), `stash@{0}` pushed with `-u` over another edit
/// and an untracked file (three). Every step a minute after the last,
/// through the fixtures' hermetic git.
fn two_stashes() -> tempfile::TempDir {
    let scratch = tempfile::tempdir().expect("a temporary directory");
    let repo = scratch.path();
    mergewright_fixtures::git::init_repo(repo).expect("git init");
    let mut at: i64 = 1_700_000_000;
    let mut step = |args: &[&str]| {
        at += 60;
        mergewright_fixtures::git::run_at(repo, args, &format!("@{at} +0000"))
            .unwrap_or_else(|why| panic!("git {args:?}: {why}"));
    };
    let write = |name: &str, text: &str| {
        std::fs::write(repo.join(name), text).unwrap_or_else(|why| panic!("{name}: {why}"));
    };
    write("notes.txt", "one\n");
    step(&["add", "notes.txt"]);
    step(&["commit", "--quiet", "-m", "Add the notes"]);
    write("notes.txt", "one\ntwo\n");
    step(&["commit", "--quiet", "-am", "Write the second note"]);
    write("notes.txt", "one\ntwo\nthree\n");
    step(&["stash", "push", "--quiet", "-m", OLDER]);
    write("notes.txt", "one\ntwo\nfour\n");
    write("scratch.txt", "untracked\n");
    step(&["stash", "push", "--quiet", "-u", "-m", NEWEST]);
    scratch
}

fn drawn(state: &AppState) -> Vec<String> {
    frame_at(state, COLUMNS, ROWS, Some(Clock::Fixed(NOW)))
}

/// The capture: the newest entry, three parents in git, drawn `■`; no
/// merge glyph on screen, since this history has no merge.
#[test]
fn capture_stash_entries_at_120() {
    let scratch = two_stashes();
    let (app, state) = App::open(scratch.path());
    let state = app.resize(&state, COLUMNS, ROWS);
    let state = app.pump(state, "the newest stash entry drawn", |state| {
        drawn(state).iter().any(|line| line.contains(NEWEST))
    });
    let state = crate::captures::at_rest(&app, state);
    let frame = drawn(&state);
    let newest = frame
        .iter()
        .find(|line| line.contains(NEWEST))
        .expect("the newest entry is drawn");
    assert!(
        newest.contains('\u{25a0}'),
        "the newest draws `■`: {newest:?}"
    );
    assert!(
        frame.iter().all(|line| !line.contains('\u{25c9}')),
        "no merge glyph: this history has no merge\n{}",
        frame.join("\n")
    );
    assert!(
        frame.iter().all(|line| !line.contains(": older")),
        "stash@{{1}} is a row now: `stash_nodes`' header is stale\n{}",
        frame.join("\n")
    );
    crate::captures::check_shot(
        "ease-pass/stash-entries-120.txt",
        &format!(
            "ease-pass 06r M1, 120x36, a repository with two stash entries, at first paint: \
             drawn by \
             `view::view` from a real AppState over the job pool \
             (crates/mergewright/tests/it/stash_nodes.rs, `two_stashes`: `stash@{{0}}` pushed with \
             `-u`, so git gives it three parents). Not a pty capture: a character grid, \
             colour unverified (CLAUDE.md rule 5); the look is yours to judge (rule 6).\n\
             What right looks like: `stash@{{0}}` ({NEWEST}) draws `■`, not `◉`; its index \
             and untracked commits are not drawn; `stash@{{1}}` ({OLDER}) is not a row (only \
             in the reflog); `HEAD` (`Write the second note`) draws `◆`.\n\n{}\n",
            frame.join("\n")
        ),
    );
    drop(state);
    app.close();
}
