//! The conflict resolver, against a real stopped repository
//! (m6-conflicts/04).
//!
//! The fixture is `Shape::StoppedMerge`/`StoppedRebase`: **ours** is `main`
//! and **theirs** is `feature` in both, so a line named for a side means the
//! same thing whichever stop the test picked.

#![allow(
    clippy::expect_used,
    reason = "a test that cannot reach its own fixture has already failed"
)]

mod base;
mod card_names;
mod file_keys;
mod file_list;
pub(crate) use file_list::gone_notes_and_readme_merge;
mod editor_words;
mod handover;
mod last_file;
mod modal;
mod restart;
mod shell_begun;
mod stash_pop;
mod take_all;
mod takes;
mod three_panes;
mod whole_file;

use std::path::Path;

use graft::jobs::Dispatch;
use graft::update::resolver::OpenedFrom;
use graft::update::{AppState, Command, GraphOverlay, Status};
use graft_core::event::{KeyCode, KeyPress, MouseKind};
use graft_fixtures::{Scratch, Shape};
use graft_git::status::UnmergedHow;
use graft_ops::operation::{Operation, ResolveWholeParams, Side};

use crate::app_harness::{App, changes_pane, cursor_to, press_for_commands};

/// A repository stopped mid-operation, its first status read landed.
fn stopped(shape: Shape) -> (Scratch, App, AppState) {
    let scratch = graft_fixtures::copy_into_temp(shape);
    let (app, state) = App::open(scratch.path());
    let state = app.pump(state, "the stopped operation", |state| {
        state.in_progress.is_some() && state.working_tree.is_some()
    });
    (scratch, app, state)
}

/// The banner has the keyboard, as it does the moment a stop is discovered.
pub(crate) fn on_the_banner(app: &App, state: AppState) -> AppState {
    let mut state = state;
    let mut guard = 0;
    while !state.banner_focus {
        assert!(guard < 6, "Tab never came round to the banner");
        state = app.key(&state, KeyCode::Tab);
        guard += 1;
    }
    state
}

/// The changes pane, with the cursor on `path`'s row.
pub(crate) fn on_the_row(app: &App, state: AppState, path: &str) -> AppState {
    let mut state = state;
    let mut guard = 0;
    while state.banner_focus {
        assert!(guard < 6, "Tab never let the banner go");
        state = app.key(&state, KeyCode::Tab);
        guard += 1;
    }
    let state = changes_pane(app, state);
    cursor_to(app, state, path)
}

/// The resolver open on `path`, its first read landed.
pub(crate) fn resolver_on(app: &App, state: AppState, path: &str) -> AppState {
    let state = on_the_row(app, state, path);
    let state = app.key(&state, KeyCode::Enter);
    app.pump(state, "the conflict file", |state| {
        state
            .resolver
            .as_ref()
            .is_some_and(|open| open.file.is_some())
    })
}

/// The resolver open on `path`, its first read landed — a whole-file shape
/// (`file` stays `None` forever; [`resolver_on`] waits for it and hangs).
pub(crate) fn resolver_on_whole_file(app: &App, state: AppState, path: &str) -> AppState {
    let state = on_the_row(app, state, path);
    let state = app.key(&state, KeyCode::Enter);
    app.pump(state, "the whole-file card", |state| {
        state
            .resolver
            .as_ref()
            .is_some_and(|open| open.whole_file.is_some())
    })
}

/// The open resolver. Panics if it is closed, which is never what a test
/// that calls this meant.
pub(crate) fn open(state: &AppState) -> &graft::update::resolver::ResolverState {
    state.resolver.as_ref().expect("the resolver is open")
}

/// Waits for the take or the read in flight to land.
pub(crate) fn settled(app: &App, state: AppState) -> AppState {
    app.pump(state, "the job the resolver started", |state| {
        state
            .resolver
            .as_ref()
            .is_none_or(|open| open.job.is_none())
    })
}

/// The file's bytes as they stand on disk.
fn on_disk(repo: &Path, name: &str) -> String {
    std::fs::read_to_string(repo.join(name)).expect("the conflicted file")
}

// ------------------------------------------------------------ opening it

#[test]
fn enter_on_conflicts_row_opens_the_resolver() {
    let (scratch, app, state) = stopped(Shape::StoppedMerge);
    let state = on_the_row(&app, state, "both.txt");

    let (_, commands) = press_for_commands(&state, KeyPress::plain(KeyCode::Enter));
    // The opening read, and the file list's conflict set (m6-conflicts/13).
    let [Command::OpenConflict(_, path), Command::ReadConflictSet(_)] = commands.as_slice() else {
        panic!("`⏎` on a CONFLICTS row queued {commands:?}")
    };
    assert_eq!(path, Path::new("both.txt"));

    let state = app.key(&state, KeyCode::Enter);
    let state = settled(&app, state);
    assert_eq!(state.graph_overlay(), Some(GraphOverlay::Resolver));
    assert_eq!(
        open(&state).cursor_block,
        Some(0),
        "on the first open block"
    );
    assert_eq!(open(&state).opened_from, OpenedFrom::Pane);

    // The bar under the document is the resolver's own, not the pane's.
    let drawn = crate::captures::frame(&state, 120, 40);
    let footer = drawn.last().expect("a footer row").to_owned();
    // One `⌃O/⌃T all` hint (review m6-amend-16 M2) wins back `⏎ done`,
    // `j/k file` and `M mergetool` at 120; with `B base` and `d start over`
    // also drawn the full line is 135 cells, so `d` and `esc` stay bare.
    for key in [
        "o yours",
        "t theirs",
        "b both",
        "⌃O/⌃T all",
        "n next",
        "M mergetool",
        " esc",
    ] {
        assert!(footer.contains(key), "{key} missing from {footer:?}");
    }

    drop(state);
    app.close();
    drop(scratch);
}

#[test]
fn enter_on_banner_opens_and_esc_returns() {
    let (scratch, app, state) = stopped(Shape::StoppedMerge);
    let state = on_the_banner(&app, state);

    let state = app.key(&state, KeyCode::Enter);
    let opened = open(&state);
    assert_eq!(opened.opened_from, OpenedFrom::Banner);
    assert_eq!(
        opened.path,
        Path::new("added-by-them.txt"),
        "the banner opens the first conflicted path"
    );

    let state = app.key(&state, KeyCode::Escape);
    assert!(state.resolver.is_none(), "Esc closes it");
    assert!(state.banner_focus, "and gives the banner its keyboard back");
    assert!(
        state.in_progress.is_some(),
        "Esc never aborts what stopped (§2.6)"
    );

    drop(state);
    app.close();
    drop(scratch);
}

#[test]
fn open_refusals_in_words() {
    let (scratch, app, state) = stopped(Shape::StoppedMerge);
    let state = resolver_on(&app, state, "both.txt");

    // Already open: `⏎` inside the resolver means "mark resolved", so the
    // second open can only come from the code, which is what this asserts.
    let mut commands = Vec::new();
    let mut next = state.clone();
    let mut raw = 9_001;
    let opened = graft::update::resolver::open_resolver(
        &mut next,
        &mut commands,
        "three.txt".into(),
        OpenedFrom::Pane,
        &mut || {
            raw += 1;
            graft_core::job::JobId::new(raw)
        },
    );
    assert!(opened, "the refusal is still an answer");
    assert!(commands.is_empty(), "and it queued nothing");
    assert!(
        matches!(&next.status, Status::Sticky { reason, .. } if reason == "the resolver is already open"),
        "{:?}",
        next.status
    );
    assert_eq!(open(&next).path, Path::new("both.txt"), "still the first");

    drop(state);
    app.close();
    drop(scratch);
}

// ------------------------------------- the last file, and the other refusals

/// `git` expected to exit non-zero: a merge or a `stash pop` that stops on
/// a conflict. Anything but a conflict stop is still a failed fixture.
fn git_stops(repo: &Path, args: &[&str]) {
    let out = std::process::Command::new("git")
        .arg("-C")
        .arg(repo)
        .args([
            "-c",
            "user.name=Graft Test",
            "-c",
            "user.email=test@example.invalid",
        ])
        .args(args)
        .output()
        .unwrap_or_else(|why| panic!("git {args:?}: {why}"));
    assert!(
        !out.status.success(),
        "git {args:?} was meant to stop on a conflict and did not"
    );
    assert!(
        !still_unmerged(repo).is_empty(),
        "git {args:?} failed without a conflict: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// git's own answer: the paths still unmerged, one per line.
fn still_unmerged(repo: &Path) -> String {
    crate::app_harness::git(repo, &["diff", "--name-only", "--diff-filter=U"])
}

/// A merge stopped on exactly two text conflicts, `a.txt` and `b.txt` —
/// small enough that "the last conflicted file" is reached through the
/// real walk rather than a state built to skip it.
fn two_file_merge() -> Scratch {
    use crate::app_harness::{git, git_as_graft};
    let scratch = graft_fixtures::copy_into_temp(Shape::Trivial);
    let repo = scratch.path();
    let commit_both = |side: &str| {
        for name in ["a.txt", "b.txt"] {
            std::fs::write(repo.join(name), format!("{side}\n")).expect("writable scratch");
        }
        git(repo, &["add", "a.txt", "b.txt"]);
        git_as_graft(repo, &["commit", "-m", side]);
    };
    commit_both("base");
    let home = git(repo, &["rev-parse", "--abbrev-ref", "HEAD"]);
    git(repo, &["checkout", "-b", "side"]);
    commit_both("side");
    git(repo, &["checkout", home.trim()]);
    commit_both("main");
    git_stops(repo, &["merge", "side"]);
    scratch
}

/// A `git stash pop` stopped on a conflict in `sample.txt`: unmerged paths
/// and no operation, so no banner (`KNOWN_GAPS.md` row 229).
fn stash_pop_conflict() -> Scratch {
    use crate::app_harness::{git, git_as_graft};
    let scratch = graft_fixtures::copy_into_temp(Shape::Trivial);
    let repo = scratch.path();
    std::fs::write(repo.join("sample.txt"), "base\n").expect("writable scratch");
    git(repo, &["add", "sample.txt"]);
    git_as_graft(repo, &["commit", "-m", "base"]);
    std::fs::write(repo.join("sample.txt"), "stashed\n").expect("writable scratch");
    git_as_graft(repo, &["stash", "push"]);
    std::fs::write(repo.join("sample.txt"), "committed\n").expect("writable scratch");
    git_as_graft(repo, &["commit", "-am", "committed"]);
    git_stops(repo, &["stash", "pop"]);
    scratch
}

/// A `git stash pop` stopped on a modify/delete: the stash deleted
/// `sample.txt`, the branch then changed it — `deleted by them`, no banner.
fn stash_pop_delete_conflict() -> Scratch {
    use crate::app_harness::{git, git_as_graft};
    let scratch = graft_fixtures::copy_into_temp(Shape::Trivial);
    let repo = scratch.path();
    std::fs::write(repo.join("sample.txt"), "base\n").expect("writable scratch");
    git(repo, &["add", "sample.txt"]);
    git_as_graft(repo, &["commit", "-m", "base"]);
    git(repo, &["rm", "-q", "sample.txt"]);
    git_as_graft(repo, &["stash", "push"]);
    std::fs::write(repo.join("sample.txt"), "committed\n").expect("writable scratch");
    git_as_graft(repo, &["commit", "-am", "committed"]);
    git_stops(repo, &["stash", "pop"]);
    scratch
}

/// The app open on `scratch`, its first status read landed.
fn opened_on(scratch: &Scratch) -> (App, AppState) {
    let (app, state) = App::open(scratch.path());
    let state = app.pump(state, "the first status read", |state| {
        state.working_tree.is_some()
    });
    (app, state)
}

/// Take ours on every open block of the file on screen, then `⏎`, and wait
/// for the stage and whatever the resolver does after it.
fn take_all_and_mark(app: &App, state: AppState) -> AppState {
    let path = open(&state).path.clone();
    let mut state = state;
    while open(&state).file.as_ref().expect("a document").open() > 0 {
        state = app.key(&state, KeyCode::Char('o'));
        state = settled(app, state);
    }
    let state = app.key(&state, KeyCode::Enter);
    assert!(
        state.operating.is_some(),
        "`⏎` queued no stage for {}: {:?}",
        path.display(),
        state.status
    );
    // Where the stage lands: the next file's document, or — the last file
    // (m6-conflicts/31) — the same file reopened read-only as done, or the
    // resolver closed (a stash pop).
    app.pump(state, "the stage and what follows it", |state| {
        state.operating.is_none()
            && state.status_read.is_none()
            && state.resolver.as_ref().is_none_or(|open| {
                open.job.is_none()
                    && if open.path == path {
                        open.done.is_some()
                    } else {
                        open.file.is_some()
                    }
            })
    })
}

/// The journal on disk, as the app's own reader sees it.
fn journal_entries(repo: &Path) -> Vec<graft_core::journal::JournalEntry> {
    graft_ops::journal::Journal::open(&repo.join(".git/graft/undo-journal.jsonl"))
        .expect("the journal the merge just wrote is readable")
        .entries()
        .to_vec()
}
