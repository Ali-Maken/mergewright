//! `S`, `s`, `Enter`, `p` and `d` over the real job pool and real `git`
//! (m3-working-tree-undo-and-stash/06).
//!
//! The overlay's own tests settle what a key *means*; none of that is
//! evidence that a stash was saved. These press the keys, then read the
//! repository back with `git stash list` and off the disk, and check that
//! `u` puts every one of them back.
#![allow(
    clippy::expect_used,
    reason = "a test fixture that cannot be built has nothing to test; the panic names why"
)]

use std::path::Path;

use mergewright_core::event::KeyCode;

use crate::app_harness::{App, git, git_as_mergewright};

/// The committed contents of `sample.txt`, which a clean tree holds.
const COMMITTED: &str = "one\ntwo\nthree\n";
/// What the working tree holds once it is dirty.
const EDITED: &str = "one\ntwo\nthree\nfour\n";

/// A scratch repository with one commit, one tracked file, and an edit to
/// that file waiting in the working tree.
fn repo_with_an_edit() -> mergewright_fixtures::Scratch {
    let scratch = mergewright_fixtures::copy_into_temp(mergewright_fixtures::Shape::Trivial);
    let repo = scratch.path();
    std::fs::write(repo.join("sample.txt"), COMMITTED).expect("the scratch repo is writable");
    git(repo, &["add", "sample.txt"]);
    git_as_mergewright(repo, &["commit", "-m", "add sample"]);
    std::fs::write(repo.join("sample.txt"), EDITED).expect("the scratch repo is writable");
    scratch
}

/// `git stash list`, one line per entry — read out of git, never out of
/// `AppState`.
fn stash_list(repo: &Path) -> Vec<String> {
    git(repo, &["stash", "list"])
        .lines()
        .map(str::to_owned)
        .collect()
}

fn sample(repo: &Path) -> String {
    std::fs::read_to_string(repo.join("sample.txt")).expect("the tracked file is there")
}

/// Opens the app and the stash overlay, waiting for the list to land.
fn overlay(repo: &Path) -> (App, mergewright::update::AppState) {
    let (app, state) = App::open(repo);
    let state = app.pump(state, "the working-tree status", |state| {
        state.working_tree.is_some()
    });
    let state = app.key(&state, KeyCode::Char('S'));
    assert!(state.stash.shown, "`S` opens the overlay");
    let state = app.pump(state, "the stash list", |state| {
        !matches!(
            state.stash.entries,
            mergewright::update::detail::Loading::Idle
                | mergewright::update::detail::Loading::Pending(_)
        )
    });
    (app, state)
}

/// Waits for the overlay's list to catch up with a write that just landed.
fn settle_list(
    app: &App,
    state: mergewright::update::AppState,
    want: usize,
) -> mergewright::update::AppState {
    app.pump(state, "the stash list after a write", |state| {
        state.stash.rows().len() == want
    })
}

/// `s`, a message, `Enter`: the working tree goes into a stash and the file
/// on disk is the committed one again. Then `u` takes the stash back, and
/// `U` puts it there again — each step read back out of git.
#[test]
fn s_saves_the_working_tree_and_u_takes_the_stash_back() {
    let scratch = repo_with_an_edit();
    let repo = scratch.path();
    let (app, state) = overlay(repo);
    assert!(stash_list(repo).is_empty(), "nothing stashed yet");
    assert_eq!(state.stash.rows().len(), 0, "and the overlay agrees");

    let state = app.key(&state, KeyCode::Char('s'));
    let mut state = state;
    for character in "wip".chars() {
        state = app.key(&state, KeyCode::Char(character));
    }
    let state = app.key(&state, KeyCode::Enter);
    let state = app.settle_write(state, "the stash push");

    let listed = stash_list(repo);
    assert_eq!(listed.len(), 1, "one stash, from git: {listed:?}");
    assert!(listed[0].contains("wip"), "the message went in: {listed:?}");
    assert_eq!(sample(repo), COMMITTED, "the working tree is clean again");

    // The overlay re-read itself, without anyone pressing a key.
    let state = settle_list(&app, state, 1);
    assert_eq!(state.stash.rows().len(), 1);

    let state = app.key(&state, KeyCode::Char('u'));
    let state = app.settle_write(state, "undoing the stash push");
    assert!(
        stash_list(repo).is_empty(),
        "undo dropped the entry it created: {:?}",
        stash_list(repo)
    );
    assert_eq!(sample(repo), EDITED, "and the edit is back in the tree");

    let state = app.key(&state, KeyCode::Char('U'));
    let state = app.settle_write(state, "redoing the stash push");
    assert_eq!(stash_list(repo).len(), 1, "redo stashed it again");
    assert_eq!(sample(repo), COMMITTED);
    let _ = state;
    app.close();
}

/// `Enter` then `y` on a stash applies it: the edit is back in the tree and the
/// entry is still on the list. `u` then takes the application back.
#[test]
fn enter_applies_a_stash_and_u_takes_the_application_back() {
    let scratch = repo_with_an_edit();
    let repo = scratch.path();
    git_as_mergewright(repo, &["stash", "push", "-m", "earlier"]);
    assert_eq!(sample(repo), COMMITTED, "git stashed the edit away");

    let (app, state) = overlay(repo);
    assert_eq!(state.stash.rows().len(), 1, "the overlay lists it");

    // `⏎` opens the apply preview; `y` applies (m6-conflicts/86).
    let state = crate::stash_clash::put_back(&app, &state, KeyCode::Enter);
    let state = app.settle_write(state, "the stash apply");
    assert_eq!(sample(repo), EDITED, "the stash landed in the tree");
    assert_eq!(
        stash_list(repo).len(),
        1,
        "apply keeps the entry; only pop takes it off"
    );

    let state = app.key(&state, KeyCode::Char('u'));
    let state = app.settle_write(state, "undoing the stash apply");
    assert_eq!(sample(repo), COMMITTED, "the tree is clean again");
    assert_eq!(stash_list(repo).len(), 1, "and the stash is still there");
    let _ = state;
    app.close();
}

/// `p` then `y` pops: the edit comes back and the entry leaves the list. `u` puts
/// the entry back — at `stash@{0}`, which is the only place git can put it.
#[test]
fn p_pops_a_stash_and_u_puts_the_entry_back() {
    let scratch = repo_with_an_edit();
    let repo = scratch.path();
    git_as_mergewright(repo, &["stash", "push", "-m", "earlier"]);

    let (app, state) = overlay(repo);
    let state = crate::stash_clash::put_back(&app, &state, KeyCode::Char('p'));
    let state = app.settle_write(state, "the stash pop");
    assert_eq!(sample(repo), EDITED);
    assert!(stash_list(repo).is_empty(), "pop took the entry off");

    let state = app.key(&state, KeyCode::Char('u'));
    let state = app.settle_write(state, "undoing the stash pop");
    let listed = stash_list(repo);
    assert_eq!(listed.len(), 1, "the entry is back: {listed:?}");
    assert!(listed[0].starts_with("stash@{0}"), "{listed:?}");
    assert_eq!(sample(repo), COMMITTED);
    let _ = state;
    app.close();
}

/// `d` asks first; the box's second button drops. `u` then puts the entry
/// back — and because git cannot insert into a reflog, it comes back at
/// `stash@{0}` whatever its index was.
#[test]
fn d_drops_a_stash_behind_the_box_and_u_puts_it_back_at_the_top() {
    let scratch = repo_with_an_edit();
    let repo = scratch.path();
    git_as_mergewright(repo, &["stash", "push", "-m", "older"]);
    std::fs::write(repo.join("sample.txt"), "one\ntwo\nthree\nfive\n")
        .expect("the scratch repo is writable");
    git_as_mergewright(repo, &["stash", "push", "-m", "newer"]);
    assert_eq!(stash_list(repo).len(), 2);

    let (app, state) = overlay(repo);
    assert_eq!(state.stash.rows().len(), 2);
    // Down to `stash@{1}`, the older one, so the drop is not the top entry.
    let state = app.key(&state, KeyCode::Char('j'));
    assert_eq!(state.stash.selected, 1);

    let state = app.key(&state, KeyCode::Char('d'));
    let pending = state.confirm.as_ref().expect("the box opened");
    assert_eq!(pending.view.title.as_str(), "Drop stash@{1}?");
    assert!(state.operating.is_none(), "nothing ran before the answer");
    // `→` off the safe button, then `Enter`.
    let state = app.key(&state, KeyCode::Right);
    let state = app.key(&state, KeyCode::Enter);
    let state = app.settle_write(state, "the stash drop");

    let listed = stash_list(repo);
    assert_eq!(listed.len(), 1, "one left: {listed:?}");
    assert!(listed[0].contains("newer"), "{listed:?}");

    let state = app.key(&state, KeyCode::Char('u'));
    let state = app.settle_write(state, "undoing the stash drop");
    let listed = stash_list(repo);
    assert_eq!(listed.len(), 2, "the dropped entry is back: {listed:?}");
    assert!(
        listed[0].contains("older"),
        "it comes back at stash@{{0}}, not where it was: {listed:?}"
    );
    let _ = state;
    app.close();
}

/// Every stash entry gets its own `stash@{n}` badge, not just the newest.
/// `refs/stash` is one ref; the older entries live in its reflog, which is
/// why they had none before (`KNOWN_GAPS.md`,
/// m3-working-tree-undo-and-stash/06). Read through the real refs job.
#[test]
fn every_stash_entry_is_badged_on_a_real_repository() {
    let scratch = repo_with_an_edit();
    let repo = scratch.path();
    git_as_mergewright(repo, &["stash", "push", "-m", "older"]);
    std::fs::write(repo.join("sample.txt"), "one\ntwo\nthree\nfive\n")
        .expect("the scratch repo is writable");
    git_as_mergewright(repo, &["stash", "push", "-m", "newer"]);

    let (app, state) = App::open(repo);
    let state = app.pump(state, "the refs", |state| state.refs.is_some());
    let refs = state.refs.as_ref().expect("the refs job answered");

    let stashes = refs.stashes();
    assert_eq!(stashes.len(), 2, "both entries reached the badges");
    assert_eq!(stashes[0].1, 0);
    assert_eq!(stashes[1].1, 1);
    for &(commit, index) in stashes {
        assert!(
            refs.on(commit)
                .contains(&mergewright::refs::RefLabel::Stash(index)),
            "stash@{{{index}}} has no badge on {commit:?}"
        );
    }
    app.close();
}
