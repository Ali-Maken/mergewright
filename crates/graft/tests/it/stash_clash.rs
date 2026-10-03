//! m6-conflicts/72b: a stash put back that clashes lands on the resolver,
//! says which stash, and finishing drops a popped one — against a real
//! repository, the pop or apply made from `S`.
#![allow(
    clippy::expect_used,
    reason = "a test fixture that cannot be built has nothing to test; the panic names why"
)]

use std::path::Path;

use graft::update::detail::Loading;
use graft::update::{AppState, Command, LiveOrigin, Status};
use graft_core::event::{KeyCode, KeyPress};
use graft_core::journal::PutBackWhy;
use graft_ui::detail::DetailFocus;

use graft::session::Clock;

use crate::app_harness::{App, git, press_for_commands};
use crate::captures::{check_shot, frame, frame_at};
use crate::preview::{FORK_EPOCH, FORK_NOW, dated_commit};

/// The file the stash and the branch both change.
pub(crate) const NOTES: &str = "notes.md";

/// `main` with `notes.md` committed as `base`, a stash holding `stashed`
/// over it, then `committed` committed on top: popping or applying the stash
/// clashes in `notes.md`, one block. Dated, so every id is the same on every
/// run.
pub(crate) fn a_stash_that_clashes() -> graft_fixtures::Scratch {
    let scratch = graft_fixtures::copy_into_temp(graft_fixtures::Shape::Trivial);
    let path = scratch.path();
    dated_commit(path, NOTES, "base\n", "add the notes", 1);
    std::fs::write(path.join(NOTES), "stashed\n").expect("the scratch repo is writable");
    let when = format!("@{} +0000", FORK_EPOCH + 2 * 60);
    graft_fixtures::git::run_at(path, &["stash", "push"], &when)
        .unwrap_or_else(|why| panic!("stashing: {why}"));
    dated_commit(path, NOTES, "committed\n", "commit over the notes", 3);
    scratch
}

pub(crate) fn stash_list(repo: &Path) -> Vec<String> {
    git(repo, &["stash", "list", "--format=%H"])
        .lines()
        .map(str::to_owned)
        .collect()
}

pub(crate) fn unmerged(repo: &Path) -> String {
    git(repo, &["ls-files", "-u"]).trim().to_owned()
}

/// Opened, the working tree read, and the `S` overlay's list landed.
pub(crate) fn overlay_open(app: &App, state: AppState) -> AppState {
    let state = app.pump(state, "the working tree and the refs", |state| {
        state.working_tree.is_some() && state.refs.is_some()
    });
    let state = app.key(&state, KeyCode::Char('S'));
    app.pump(state, "the stash list", |state| {
        !matches!(state.stash.entries, Loading::Idle | Loading::Pending(_))
    })
}

/// The stash list's put-back preview has its plan (m6-conflicts/86): `⏎`
/// or `p` opened it, and `y` is answered only once the picture is up.
pub(crate) fn previewed(app: &App, state: AppState) -> AppState {
    app.pump(state, "the stash preview's plan", |state| {
        state
            .preview
            .as_ref()
            .is_some_and(|preview| !matches!(preview.plan, Loading::Idle | Loading::Pending(_)))
    })
}

/// `key` (`⏎` apply or `p` pop) on the stash list, its preview landed, then
/// `y`: the put-back as `⏎`/`p` alone ran it before m6-conflicts/86.
pub(crate) fn put_back(app: &App, state: &AppState, key: KeyCode) -> AppState {
    let state = previewed(app, app.key(state, key));
    app.key(&state, KeyCode::Char('y'))
}

/// The put-back has answered, the resolver's first read has landed and
/// every read the answer asked for is in.
pub(crate) fn landed(app: &App, state: AppState) -> AppState {
    app.pump(state, "the clash landing", |state| {
        state.operating.is_none()
            && state
                .resolver
                .as_ref()
                .is_some_and(|open| open.file.is_some())
            && state.status_read.is_none()
            && state.journal.reading.is_none()
            && state.clash.is_some()
    })
}

/// Every read after the last write is in.
pub(crate) fn quiet(app: &App, state: AppState) -> AppState {
    app.pump(state, "the reads after the write", |state| {
        state.operating.is_none()
            && state.status_read.is_none()
            && state.journal.reading.is_none()
            && !matches!(state.refs_read, graft::update::RefsRead::Pending(_))
            && !state.clash_index_out()
    })
}

/// `S p y` from the graph onto a clash, landed.
fn popped(app: &App, state: AppState) -> AppState {
    let state = overlay_open(app, state);
    let state = put_back(app, &state, KeyCode::Char('p'));
    landed(app, state)
}

fn notice(state: &AppState) -> String {
    state
        .resolver
        .as_ref()
        .and_then(|open| open.notice.as_ref())
        .map(|(words, _)| words.clone())
        .unwrap_or_default()
}

#[test]
fn a_clashing_pop_lands_on_the_resolver_on_the_first_clashed_file() {
    let scratch = a_stash_that_clashes();
    let (app, state) = App::open(scratch.path());
    let state = popped(&app, state);

    assert!(!state.stash.shown, "the `S` overlay closed");
    assert!(state.detail.shown, "the changes pane is open");
    let open = state.resolver.as_ref().expect("the resolver is open");
    assert_eq!(open.path, Path::new(NOTES));
    assert_eq!(open.opened_from, graft::update::resolver::OpenedFrom::Pane);
    assert_eq!(
        notice(&state),
        "stash@{0} clashed in 1 file \u{b7} esc, u puts it all back"
    );
    let clash = state.clash.as_ref().expect("the clash is recorded");
    assert_eq!(clash.clash.why, PutBackWhy::Pop);
    assert_eq!(clash.clash.index, Some(0));
    assert_eq!(clash.origin, LiveOrigin::ThisRun);
    assert_eq!(stash_list(scratch.path()).len(), 1, "git kept the stash");
    drop(state);
    app.close();
}

#[test]
fn the_stash_pane_is_named_by_its_index() {
    let scratch = a_stash_that_clashes();
    let (app, state) = App::open(scratch.path());
    let state = popped(&app, state);
    let names = graft::update::resolver::side_names(&state);
    assert_eq!(names.ours.as_str(), "yours \u{b7} main");
    assert_eq!(names.theirs.as_str(), "stashed \u{b7} stash@{0}");
    for (columns, rows) in [(120, 40), (80, 24)] {
        let drawn = frame(&state, columns, rows);
        let bar = drawn.last().expect("a bottom row");
        assert!(
            bar.contains("o yours") && bar.contains("t stashed"),
            "{columns}x{rows}: {bar}"
        );
        assert!(
            drawn
                .iter()
                .any(|row| row.contains("stashed \u{b7} stash@{0}")),
            "{columns}x{rows}:\n{}",
            drawn.join("\n")
        );
    }
    drop(state);
    app.close();
}

#[test]
fn esc_after_a_clash_lands_on_conflicts_in_the_changes_pane() {
    let scratch = a_stash_that_clashes();
    let (app, state) = App::open(scratch.path());
    let state = popped(&app, state);
    let state = app.key(&state, KeyCode::Escape);
    let state = quiet(&app, state);
    let state = app.pump(state, "the changes pane's list", |state| {
        state.detail.kept_cursor.is_none()
    });
    assert!(state.resolver.is_none(), "esc closed the resolver");
    assert!(state.detail.shown, "on the changes pane");
    assert_eq!(state.detail.focus, DetailFocus::Files);
    assert_eq!(
        graft::update::banner::cursor_conflict(&state).as_deref(),
        Some(Path::new(NOTES)),
        "the cursor is on the CONFLICTS row"
    );
    assert!(state.clash.is_some(), "esc ends nothing");
    drop(state);
    app.close();
}

#[test]
fn the_last_file_done_after_a_pop_clash_drops_the_stash() {
    let scratch = a_stash_that_clashes();
    let (app, state) = App::open(scratch.path());
    let state = popped(&app, state);
    let state = app.key(&state, KeyCode::Char('o'));
    let state = crate::resolver::settled(&app, state);
    let state = app.key(&state, KeyCode::Enter);
    let state = app.pump(state, "the stage, the drop and their reads", |state| {
        state.resolver.is_none()
            && state.operating.is_none()
            && matches!(&state.status, Status::Done { what, .. } if what.contains("dropped"))
    });
    let state = quiet(&app, state);
    assert_eq!(
        state.status,
        Status::Done {
            what: "every file done \u{b7} stash@{0} dropped".to_owned(),
            also: None,
            keys: Some("u brings it back".to_owned()),
        }
    );
    assert!(
        stash_list(scratch.path()).is_empty(),
        "the stash is dropped"
    );
    assert_eq!(unmerged(scratch.path()), "");
    assert!(state.clash.is_none(), "the clash is over");
    assert!(
        state.detail.shown,
        "the resolver closed to the changes pane"
    );
    drop(state);
    app.close();
}

#[test]
fn after_an_apply_clash_the_stash_is_kept() {
    let scratch = a_stash_that_clashes();
    let (app, state) = App::open(scratch.path());
    let state = overlay_open(&app, state);
    let state = put_back(&app, &state, KeyCode::Enter);
    let state = landed(&app, state);
    assert_eq!(
        state.clash.as_ref().map(|clash| clash.clash.why),
        Some(PutBackWhy::Apply)
    );
    let state = app.key(&state, KeyCode::Char('o'));
    let state = crate::resolver::settled(&app, state);
    let state = app.key(&state, KeyCode::Enter);
    let state = app.pump(state, "the stage", |state| state.resolver.is_none());
    let state = quiet(&app, state);
    assert_eq!(
        state.status,
        Status::Done {
            what: "every file done \u{b7} stash@{0} kept".to_owned(),
            also: None,
            keys: None,
        }
    );
    assert_eq!(stash_list(scratch.path()).len(), 1, "an apply never drops");
    drop(state);
    app.close();
}

/// A pop made in the shell has no record: the resolver names no stash, the
/// last file closes as it always did, and nothing is dropped.
#[test]
fn a_shell_pop_clash_names_no_stash_and_drops_nothing() {
    let scratch = a_stash_that_clashes();
    let path = scratch.path();
    let out = std::process::Command::new("git")
        .arg("-C")
        .arg(path)
        .args(["stash", "pop"])
        .output()
        .expect("git runs");
    assert!(!out.status.success(), "the shell pop clashes");
    let (app, state) = App::open(path);
    let state = quiet(&app, state);
    assert!(state.clash.is_none(), "no record, no clash");
    let state = crate::resolver::resolver_on(&app, state, NOTES);
    assert_eq!(
        graft::update::resolver::side_names(&state).theirs.as_str(),
        "stashed"
    );
    let state = app.key(&state, KeyCode::Char('o'));
    let state = crate::resolver::settled(&app, state);
    let state = app.key(&state, KeyCode::Enter);
    let state = app.pump(state, "the stage and the close", |state| {
        state.operating.is_none() && state.resolver.is_none() && state.status_read.is_none()
    });
    assert!(
        matches!(&state.status, Status::Done { what, also: None, keys: None } if what == "every file done"),
        "{:?}",
        state.status
    );
    assert_eq!(
        stash_list(path).len(),
        1,
        "a shell pop's stash is left alone"
    );
    drop(state);
    app.close();
}

/// Two stashes that each clash with a commit made since: A (`stash@{0}`)
/// on `a.txt`, B (`stash@{1}`) on `b.txt`.
fn two_clashing_stashes() -> graft_fixtures::Scratch {
    let scratch = graft_fixtures::copy_into_temp(graft_fixtures::Shape::Trivial);
    let path = scratch.path();
    std::fs::write(path.join("a.txt"), "base\n").expect("writable");
    std::fs::write(path.join("b.txt"), "base\n").expect("writable");
    git(path, &["add", "a.txt", "b.txt"]);
    git(path, &["commit", "-q", "-m", "base"]);
    std::fs::write(path.join("b.txt"), "stashed b\n").expect("writable");
    git(path, &["stash", "push", "-q", "-m", "B"]);
    std::fs::write(path.join("a.txt"), "stashed a\n").expect("writable");
    git(path, &["stash", "push", "-q", "-m", "A"]);
    std::fs::write(path.join("a.txt"), "committed a\n").expect("writable");
    std::fs::write(path.join("b.txt"), "committed b\n").expect("writable");
    git(path, &["commit", "-q", "-am", "moved on"]);
    scratch
}

/// Review 72+72b M2 in the app (ADR-0045 §2 as amended, §7): `S p y` clashes
/// on A; the shell takes ours for `a.txt` and pops B, which clashes on
/// `b.txt`. The index is not what Graft last recorded, so no clash: the
/// resolver's theirs names no stash, and `⏎` on `b.txt` drops nothing.
#[test]
fn a_shell_pop_after_an_unfinished_graft_pop_names_no_stash_and_drops_nothing() {
    let scratch = two_clashing_stashes();
    let path = scratch.path();
    let (app, state) = App::open(path);
    let state = popped(&app, state);
    assert!(state.clash.is_some(), "Graft's pop of A clashed");
    let state = app.key(&state, KeyCode::Escape);
    let state = quiet(&app, state);

    git(path, &["checkout", "--ours", "a.txt"]);
    git(path, &["add", "a.txt"]);
    let popped_b = std::process::Command::new("git")
        .arg("-C")
        .arg(path)
        .args(["stash", "pop", "stash@{1}"])
        .output()
        .expect("git runs");
    assert!(!popped_b.status.success(), "B's pop clashes");
    assert_eq!(stash_list(path).len(), 2, "git kept both");

    let state = app.press(&state, KeyPress::control(KeyCode::Char('r')));
    let state = quiet(&app, state);
    assert!(state.clash.is_none(), "B's conflict is not A's clash");
    // `esc` left the keys on the changes pane's list: straight to the row.
    assert_eq!(state.detail.focus, DetailFocus::Files);
    let state = app.pump(state, "b.txt in the pane", |state| {
        matches!(&state.detail.files, Loading::Ready(files)
            if files.rows.iter().any(|row| row.name.as_str() == "b.txt"))
    });
    let state = crate::app_harness::cursor_to(&app, state, "b.txt");
    let state = app.key(&state, KeyCode::Enter);
    let state = app.pump(state, "the conflict file", |state| {
        state
            .resolver
            .as_ref()
            .is_some_and(|open| open.file.is_some())
    });
    assert_eq!(
        graft::update::resolver::side_names(&state).theirs.as_str(),
        "stashed",
        "theirs names no stash"
    );
    let state = app.key(&state, KeyCode::Char('o'));
    let state = crate::resolver::settled(&app, state);
    let state = app.key(&state, KeyCode::Enter);
    let state = app.pump(state, "the stage and the close", |state| {
        state.operating.is_none() && state.resolver.is_none() && state.status_read.is_none()
    });
    let state = quiet(&app, state);
    assert_eq!(stash_list(path).len(), 2, "A and B are both still listed");
    assert!(state.clash.is_none());
    drop(state);
    app.close();
}

#[test]
fn relaunch_with_a_clash_open_names_the_stash() {
    let scratch = a_stash_that_clashes();
    let path = scratch.path();
    let (app, state) = App::open(path);
    let state = popped(&app, state);
    drop(state);
    app.close();

    let (app, state) = App::open(path);
    let state = app.pump(state, "the launch finds the clash", |state| {
        state.clash.is_some()
    });
    let state = quiet(&app, state);
    let clash = state.clash.as_ref().expect("the clash");
    assert_eq!(clash.origin, LiveOrigin::PreviousRun);
    assert_eq!(
        graft::update::banner::stop_line(&state).as_deref(),
        Some("stash@{0} clash from last run \u{b7} 1 file conflicted")
    );
    let state = app.pump(state, "the changes pane's list", |state| {
        graft::update::banner::cursor_conflict(state).is_some()
    });
    assert!(state.detail.shown, "the launch opens the changes pane");
    assert_eq!(
        graft::update::banner::cursor_conflict(&state).as_deref(),
        Some(Path::new(NOTES)),
        "⏎ from here opens the resolver"
    );
    drop(state);
    app.close();
}

/// The graph's bottom row at `columns`x`rows`, trimmed at the end.
fn bottom_row(state: &AppState, columns: u16, rows: u16) -> (Vec<String>, String) {
    let drawn = frame_at(state, columns, rows, Some(Clock::Fixed(FORK_NOW)));
    let last = drawn.last().expect("a bottom row").trim_end().to_owned();
    (drawn, last)
}

/// Review 72+72b M1: with the resolver shut (`esc`, then the graph) the
/// clash has no banner, so the stop row names it before the bar's tail keys
/// — at 80 as at 120. The exact row is asserted, then captured at 80x24.
#[test]
fn the_stop_line_words_a_clash_at_80_and_120() {
    let scratch = a_stash_that_clashes();
    let (app, state) = App::open(scratch.path());
    let state = popped(&app, state);
    let state = app.key(&state, KeyCode::Escape);
    let mut state = quiet(&app, state);
    state.detail.focus = DetailFocus::Graph;
    let ladder = graft::update::banner::stop_line_wordings(&state).expect("a clash ladder");
    assert_eq!(
        ladder.wordings,
        [
            "stash@{0} clashed \u{b7} 1 file conflicted",
            "stash@{0} clashed \u{b7} 1 conflicted",
            "stash@{0} clashed",
            "stash clashed",
        ]
    );
    assert!(
        ladder.before_keys,
        "a clash's reason outranks the tail keys"
    );

    // At 80 the bar's ` · ` joins and its right margin (ADR-0058 §4,
    // m8-polish-and-ship/06) cost the cells the fullest wording needed
    // beside the floor, so the ladder's second rung stands there.
    for (columns, rows, lead) in [
        (
            80u16,
            24u16,
            " stash@{0} clashed \u{b7} 1 conflicted \u{b7} ",
        ),
        (
            120,
            40,
            " stash@{0} clashed \u{b7} 1 file conflicted \u{b7} ",
        ),
    ] {
        let (drawn, last) = bottom_row(&state, columns, rows);
        assert!(
            drawn.iter().any(|row| row.starts_with(lead)),
            "{columns}x{rows}:\n{}",
            drawn.join("\n")
        );
        if columns == 80 {
            assert!(last.starts_with(lead), "80x24 bottom row: {last:?}");
            assert!(!last.ends_with('\u{2026}'), "nothing cut: {last:?}");
            check_shot(
                "m6/stash-clash-stop-line-80x24.txt",
                &format!("{STOP_LINE_HEADER}\n{}\n", drawn.join("\n")),
            );
        }
    }
    drop(state);
    app.close();
}

/// Review 72+72b M1, the relaunch at 80: the origin drops first, the name
/// never.
#[test]
fn the_relaunch_stop_line_at_80_keeps_the_name() {
    let scratch = a_stash_that_clashes();
    let path = scratch.path();
    let (app, state) = App::open(path);
    let state = popped(&app, state);
    drop(state);
    app.close();

    let (app, state) = App::open(path);
    let state = app.pump(state, "the launch finds the clash", |state| {
        state.clash.is_some()
    });
    let mut state = quiet(&app, state);
    state.detail.focus = DetailFocus::Graph;
    let (drawn, _) = bottom_row(&state, 80, 24);
    assert!(
        drawn.iter().any(
            |row| row.starts_with(" stash@{0} clashed \u{b7} 1 conflicted \u{b7} ")
                || row
                    .starts_with(" stash@{0} clash from last run \u{b7} 1 file conflicted \u{b7} ")
        ),
        "80x24:\n{}",
        drawn.join("\n")
    );
    drop(state);
    app.close();
}

const STOP_LINE_HEADER: &str = "\
# m6/stash-clash-stop-line-80x24.txt: drawn by `view::view` from a real
# AppState (crates/graft/tests/it/stash_clash.rs,
# the_stop_line_words_a_clash_at_80_and_120, asserted). Not a pty capture: a
# character grid with no colour (CLAUDE.md rule 5, colour unverified).
#
# What right looks like: `S p y` clashed in notes.md, then `esc` shut the
# resolver and the graph has the keys, at 80 columns.
#  - The graph screen; no banner, no resolver.
#  - The bottom hint row begins ` stash@{0} clashed · 1 file conflicted ·`,
#    then the keys that fit; nothing cut mid-word.
";

#[test]
fn a_clash_raises_no_banner_and_c_a_do_nothing_new() {
    let scratch = a_stash_that_clashes();
    let (app, state) = App::open(scratch.path());
    let state = popped(&app, state);
    let state = app.key(&state, KeyCode::Escape);
    let state = quiet(&app, state);
    assert!(graft::update::banner::banner_view(&state).is_none());
    assert!(state.stopped.is_none(), "a clash is not a stop");
    assert!(state.live_entries.is_empty(), "nor a live operation");
    for key in ['c', 'a'] {
        let (after, commands) = press_for_commands(&state, KeyPress::plain(KeyCode::Char(key)));
        assert!(
            !commands
                .iter()
                .any(|command| matches!(command, Command::Resume(..))),
            "`{key}` resumed nothing: {commands:?}"
        );
        assert!(after.finish.is_none(), "`{key}` opened no finish");
        assert!(
            after
                .confirm
                .as_ref()
                .is_none_or(|pending| !pending.view.title.contains("bort")),
            "`{key}` asked to abort nothing"
        );
    }
    drop(state);
    app.close();
}

/// Radar pin (scout Q9): the stash is still listed during a clash, so its
/// row is still a stash row, and selecting it asks for no evil-merge probe.
#[test]
fn selecting_the_stash_row_during_a_clash_probes_nothing() {
    let scratch = a_stash_that_clashes();
    let path = scratch.path();
    let stash = stash_list(path).first().cloned().expect("a stash");
    let (app, state) = App::open(path);
    let state = popped(&app, state);
    let state = app.key(&state, KeyCode::Escape);
    let mut state = quiet(&app, state);
    state.detail.focus = DetailFocus::Graph;
    for _ in 0..20 {
        let on = state
            .history
            .as_ref()
            .and_then(graft::history::History::selected)
            .map(|row| row.commit.id.to_string());
        if on.as_deref() == Some(stash.as_str()) {
            break;
        }
        let (after, commands) = press_for_commands(&state, KeyPress::plain(KeyCode::Char('j')));
        let lands_on_stash = after
            .history
            .as_ref()
            .and_then(graft::history::History::selected)
            .is_some_and(|row| row.commit.id.to_string() == stash);
        if lands_on_stash {
            assert!(
                !commands
                    .iter()
                    .any(|command| matches!(command, Command::EvilProbe { .. })),
                "the stash row asked for an evil probe: {commands:?}"
            );
            return;
        }
        state = app.key(&state, KeyCode::Char('j'));
    }
    panic!("the cursor never reached the stash row");
}
