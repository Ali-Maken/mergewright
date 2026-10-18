//! Stashes in the history view (m7-power-tools/16): which of a stash's
//! commits are rows, over the real job pool and real `git`.
#![allow(
    clippy::expect_used,
    reason = "a test fixture that cannot be built has nothing to test; the panic names why"
)]

use std::path::Path;

use mergewright::update::{AppState, RefsRead};
use mergewright_core::event::{KeyCode, KeyPress, Modifiers};
use mergewright_ui::graph_column::{NodeKind, RowPaint};
use mergewright_ui::graph_screen::BadgeKind;

use crate::app_harness::{App, git};
use crate::preview::{FORK_EPOCH, FORK_NOW, dated_commit};

/// `git stash push -m <message>` (with `-u` when `untracked`), stamped
/// `minute` minutes after [`FORK_EPOCH`] so its ids never change.
fn dated_stash(repo: &Path, message: &str, untracked: bool, minute: i64) {
    let when = format!("@{} +0000", FORK_EPOCH + minute * 60);
    let mut args = vec!["stash", "push", "-m", message];
    if untracked {
        args.push("-u");
    }
    mergewright_fixtures::git::run_at(repo, &args, &when)
        .unwrap_or_else(|why| panic!("stashing {message:?}: {why}"));
}

/// Every row the walk has sent, as its subject.
fn subjects(state: &AppState) -> Vec<String> {
    state
        .history
        .as_ref()
        .and_then(|history| history.window.as_ref())
        .map(|window| {
            window
                .rows
                .iter()
                .map(|row| String::from_utf8_lossy(row.commit.subject()).into_owned())
                .collect()
        })
        .unwrap_or_default()
}

/// Opened, the whole walk laid out and the refs read.
fn opened(app: &App, state: AppState) -> AppState {
    app.pump(state, "every row and the refs", |state| {
        state.refs.is_some() && caught_up(state)
    })
}

/// The newest request's rows are in, to the end of history.
fn caught_up(state: &AppState) -> bool {
    state.history.as_ref().is_some_and(|history| {
        history.window.as_ref().is_some_and(|window| {
            window.request == history.asked && window.done && !window.rows.is_empty()
        })
    })
}

/// Opened, and the view knows the stash entries (`Inclusion::stashes`), its
/// rows laid out again under them.
fn opened_with_stashes(app: &App, state: AppState) -> AppState {
    let state = opened(app, state);
    app.pump(state, "the view's stash entries and its rows", |state| {
        state
            .history
            .as_ref()
            .is_some_and(|history| !history.view.inclusion.stashes().is_empty())
            && caught_up(state)
    })
}

/// A repository whose one stash (`stash push -u`) has all three commits: its
/// own, the index's and the untracked files'.
fn stashed_with_untracked() -> mergewright_fixtures::Scratch {
    let scratch = mergewright_fixtures::copy_into_temp(mergewright_fixtures::Shape::Trivial);
    let repo = scratch.path();
    dated_commit(repo, "notes.txt", "a\n", "add notes", 1);
    std::fs::write(repo.join("notes.txt"), "b\n").expect("the scratch repo is writable");
    std::fs::write(repo.join("scratch.txt"), "new\n").expect("the scratch repo is writable");
    dated_stash(repo, "wip", true, 2);
    scratch
}

/// A stash is one row: its index and untracked commits are git's
/// bookkeeping, not history anyone made (note "Pass 2a" R7;
/// `Inclusion::with_stashes`).
#[test]
fn stash_index_and_untracked_commits_are_not_drawn() {
    let scratch = stashed_with_untracked();
    let (app, state) = App::open(scratch.path());
    let state = opened_with_stashes(&app, state);
    let drawn = subjects(&state);
    assert!(
        drawn.iter().any(|subject| subject == "On main: wip"),
        "the stash's own row stays: {drawn:?}"
    );
    assert!(
        !drawn.iter().any(|subject| subject.starts_with("index on ")),
        "no index commit: {drawn:?}"
    );
    assert!(
        !drawn
            .iter()
            .any(|subject| subject.starts_with("untracked files on ")),
        "no untracked-files commit: {drawn:?}"
    );
    drop(state);
    app.close();
}

/// A stash (`-u`, so all three of its commits) made on `topic`'s one
/// commit, then `topic` deleted: that commit
/// is reached only through the stash's first parent — the shape a rebase
/// leaves when it moves the branch a stash was made on.
fn stashed_on_a_lost_base() -> mergewright_fixtures::Scratch {
    let scratch = mergewright_fixtures::copy_into_temp(mergewright_fixtures::Shape::Trivial);
    let repo = scratch.path();
    dated_commit(repo, "notes.txt", "a\n", "add notes", 1);
    git(repo, &["switch", "-c", "topic"]);
    dated_commit(repo, "topic.txt", "t\n", "topic base", 2);
    std::fs::write(repo.join("topic.txt"), "t2\n").expect("the scratch repo is writable");
    std::fs::write(repo.join("scratch.txt"), "new\n").expect("the scratch repo is writable");
    dated_stash(repo, "wip", true, 3);
    git(repo, &["switch", "main"]);
    git(repo, &["branch", "-D", "topic"]);
    scratch
}

/// The drawn row whose subject is `subject`.
fn entry_for<'a>(
    view: &'a mergewright_ui::graph_screen::GraphView<'a>,
    subject: &str,
) -> &'a mergewright_ui::graph_screen::GraphEntry<'a> {
    view.rows
        .iter()
        .find(|entry| entry.subject.as_str() == subject)
        .unwrap_or_else(|| panic!("no row {subject:?}"))
}

/// Opened, with the stash-only read answered and marking something.
fn marked(app: &App, state: AppState) -> AppState {
    let state = opened_with_stashes(app, state);
    app.pump(state, "the stash-only read", |state| {
        state.stash_only.job.is_none() && !state.stash_only.commits.is_empty()
    })
}

/// Note "Pass 2a" R7: a commit only a stash reaches is drawn `□`, dimmed,
/// badged `only in stash@{0}` last — so it never reads as a commit a
/// branch still has.
#[test]
fn a_commit_only_a_stash_reaches_is_drawn_as_only_in_stash() {
    let scratch = stashed_on_a_lost_base();
    let (app, state) = App::open(scratch.path());
    let state = marked(&app, state);
    let view = mergewright::view::graph_view(&state, FORK_EPOCH);
    let base = entry_for(&view, "topic base");
    assert_eq!(base.line.node, NodeKind::StashOnly);
    assert_eq!(base.line.paint, RowPaint::Dimmed);
    assert_eq!(
        base.badges.last().map(|badge| badge.kind.clone()),
        Some(BadgeKind::OnlyInStash(0)),
        "{:?}",
        base.badges
    );
    let other = entry_for(&view, "add notes");
    assert_ne!(other.line.node, NodeKind::StashOnly, "a branch has it");
    assert!(
        !other
            .badges
            .iter()
            .any(|badge| matches!(badge.kind, BadgeKind::OnlyInStash(_))),
        "{:?}",
        other.badges
    );
    let stash = entry_for(&view, "On topic: wip");
    assert_eq!(stash.line.node, NodeKind::Stash, "the stash's own row");
    drop(view);
    drop(state);
    app.close();
}

/// First step (ticket 16's "Decided"): with two stashes, is `stash@{1}`'s
/// own commit a row? The walk's tips are every ref (`Walk::all_refs`), and
/// `stash@{1}` lives only in the reflog. The answer is recorded in the
/// ticket's NOTES; the marks below apply only to rows that are drawn.
#[test]
fn which_stash_entries_are_drawn() {
    let scratch = mergewright_fixtures::copy_into_temp(mergewright_fixtures::Shape::Trivial);
    let repo = scratch.path();
    dated_commit(repo, "notes.txt", "a\n", "add notes", 1);
    std::fs::write(repo.join("notes.txt"), "b\n").expect("the scratch repo is writable");
    dated_stash(repo, "older wip", false, 2);
    std::fs::write(repo.join("notes.txt"), "c\n").expect("the scratch repo is writable");
    dated_stash(repo, "newer wip", false, 3);

    let (app, state) = App::open(repo);
    let state = opened(&app, state);
    let drawn = subjects(&state);
    assert!(
        drawn.iter().any(|subject| subject.contains("newer wip")),
        "stash@{{0}} is a row: {drawn:?}"
    );
    assert!(
        !drawn.iter().any(|subject| subject.contains("older wip")),
        "stash@{{1}} is not a row — the walk's tips are refs, and it is only \
         in the reflog: {drawn:?}"
    );
    drop(state);
    app.close();
}

/// Whether the drawn row `subject` carries any "only in a stash" mark.
fn is_marked(state: &AppState, subject: &str) -> bool {
    let view = mergewright::view::graph_view(state, FORK_EPOCH);
    let entry = entry_for(&view, subject);
    entry.line.node == NodeKind::StashOnly
        || entry
            .badges
            .iter()
            .any(|badge| matches!(badge.kind, BadgeKind::OnlyInStash(_)))
}

/// A branch put back on the stash's base — as a rebase undone, or a branch
/// made there by hand — and the next refs read unmarks it: the mark is the
/// newest read's answer, not a fact kept from launch.
#[test]
fn a_rebased_base_on_a_branch_again_loses_the_mark() {
    let scratch = stashed_on_a_lost_base();
    let repo = scratch.path();
    let (app, state) = App::open(repo);
    let state = marked(&app, state);
    assert!(is_marked(&state, "topic base"));

    git(repo, &["branch", "again", "stash@{0}^1"]);
    let state = app.press(
        &state,
        KeyPress {
            code: KeyCode::Char('r'),
            modifiers: Modifiers {
                control: true,
                ..Modifiers::NONE
            },
        },
    );
    let state = app.pump(state, "the refresh's refs and stash-only read", |state| {
        matches!(state.refs_read, RefsRead::Current(_))
            && state.stash_only.job.is_none()
            && state.stash_only.commits.is_empty()
    });
    let state = app.pump(state, "the rows after the refresh", caught_up);
    assert!(!is_marked(&state, "topic base"), "a branch has it again");
    drop(state);
    app.close();
}

/// `H` hides the stash, and with it every commit only the stash reaches:
/// the marks go with their rows (`Inclusion::hidden_stashes`, scout 16 §2).
/// A regression test: `H` is unchanged by ticket 16.
#[test]
fn h_hides_the_only_in_stash_rows_too() {
    let scratch = stashed_on_a_lost_base();
    let (app, state) = App::open(scratch.path());
    let state = marked(&app, state);
    assert!(
        subjects(&state)
            .iter()
            .any(|subject| subject == "topic base")
    );

    let state = app.key(&state, KeyCode::Char('H'));
    let state = app.pump(state, "the rows with the stash hidden", |state| {
        state
            .history
            .as_ref()
            .is_some_and(|history| !history.view.inclusion.hidden_stashes.is_empty())
            && caught_up(state)
    });
    let drawn = subjects(&state);
    assert!(
        !drawn
            .iter()
            .any(|subject| subject == "topic base" || subject == "On topic: wip"),
        "{drawn:?}"
    );
    let view = mergewright::view::graph_view(&state, FORK_EPOCH);
    assert!(
        view.rows
            .iter()
            .all(|entry| entry.line.node != NodeKind::StashOnly),
        "no mark is left on a row that is still drawn"
    );
    drop(view);
    drop(state);
    app.close();
}

/// `docs/shots/m7/stash-in-history-<size>.txt`. What right looks like: no
/// `index on` and no `untracked files on` row; exactly one `□` row, `topic
/// base`, badged `[only in stash@{0}]`; the stash's own row, `On topic:
/// wip`, still there with its `■` and `[stash@{0}]`. Colour (the dimmed row,
/// the badge's colour) unverified (rule 5).
fn capture_stash_in_history(columns: u16, rows: u16, shot: &str) {
    let scratch = stashed_on_a_lost_base();
    let (app, state) = App::open(scratch.path());
    let state = app.resize(&state, columns, rows);
    let state = marked(&app, state);
    let state = crate::captures::settled_frame(&app, state);
    let state = crate::captures::radar_answered(&app, state);
    let frame = crate::captures::frame_at(
        &state,
        columns,
        rows,
        Some(mergewright::session::Clock::Fixed(FORK_NOW)),
    );
    let body = format!("{}\n", frame.join("\n"));
    assert!(!body.contains("index on "), "{body}");
    assert!(!body.contains("untracked files on "), "{body}");
    let marked_rows: Vec<&String> = frame
        .iter()
        .filter(|row| row.contains('\u{25a1}'))
        .collect();
    assert_eq!(marked_rows.len(), 1, "{body}");
    assert!(marked_rows[0].contains("topic base"), "{body}");
    assert!(marked_rows[0].contains("only in stash@{0}"), "{body}");
    assert!(body.contains("On topic: wip"), "{body}");
    // The cursor stands on the stash's row: its bar (m8/15 A4).
    let bar = frame.last().expect("a footer row");
    assert!(bar.contains("⏎ open · S stash"), "{bar}");
    for refused in ["m merge", "r rebase", "R rewrite", "o take"] {
        assert!(!bar.contains(refused), "{bar}");
    }
    crate::captures::check_shot(&format!("{shot}-{columns}x{rows}.txt"), &body);
    drop(state);
    app.close();
}

#[test]
fn capture_stash_in_history_120x40() {
    capture_stash_in_history(120, 40, "m7/stash-in-history");
}

#[test]
fn capture_stash_in_history_80x24() {
    capture_stash_in_history(80, 24, "m7/stash-in-history");
}

/// `docs/shots/m8-bar-stash-row-120x36.txt` (m8/15 A4). What right looks
/// like: the cursor on the stash's row, its bar `⏎ open · S stash · …` with
/// no `m merge`, `r rebase`, `R rewrite` or `o take`.
#[test]
fn capture_m8_bar_stash_row_120x36() {
    capture_stash_in_history(120, 36, "m8-bar-stash-row");
}
