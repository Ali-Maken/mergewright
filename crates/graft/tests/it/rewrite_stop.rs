//! A rewrite's stop over a real repository (m7-power-tools/07b): Graft's
//! rewrite stops and Graft dies (the process that ran it is gone; the plan
//! and `rebase-merge/` stay on disk), the next Graft opens on the stop.
//!
//! The same rules over plain data are `update/banner/rewrite_stop_tests.rs`.

use std::path::Path;

use graft::update::AppState;
use graft::update::banner::banner_view;
use graft_core::event::KeyCode;
use graft_core::oid::Oid;
use graft_ops::apply::{ApplyError, Context, apply};
use graft_ops::operation::{FactsScope, Operation};
use graft_ops::rewrite::{RewriteRequest, TodoAction, plan};
use graft_ui::graph_screen::banner_line;

use crate::app_harness::{App, git, press_for_commands};
use crate::m7_walk::rebase_line::line_repo;

const COLUMNS: u16 = 120;
const ROWS: u16 = 40;

fn id(repo: &Path, rev: &str) -> Oid {
    Oid::from_hex(git(repo, &["rev-parse", rev]).trim())
        .unwrap_or_else(|why| panic!("a commit id: {why:?}"))
}

/// Graft's own rewrite of `from`'s line, run the way the job thread runs
/// it, with `edit` on the row whose subject is `edit` (if any) and the base
/// moved to `onto` (if given). It stops, and nothing of this process's is
/// left but what is on disk — a kill, as the next Graft sees it.
fn graft_rewrite_stops(repo: &Path, from: &str, edit: Option<&str>, onto: Option<&str>) {
    let handle = graft_git::repo::RepoHandle::open(repo)
        .unwrap_or_else(|why| panic!("the repo opens: {why:?}"));
    let version = graft_git::version::system_git()
        .unwrap_or_else(|why| panic!("git is on PATH: {why:?}"))
        .version();
    let facts = graft_ops::facts::read_facts(&handle, version, &(), &FactsScope::default())
        .unwrap_or_else(|why| panic!("the facts read: {why:?}"));
    let mut params = plan(
        &handle,
        &version,
        &RewriteRequest::Open {
            from: id(repo, from),
        },
        &facts,
        None,
        &(),
    )
    .unwrap_or_else(|why| panic!("it plans: {why:?}"));
    if let Some(subject) = edit {
        let row = params
            .todo
            .iter_mut()
            .find(|row| row.subject == subject)
            .unwrap_or_else(|| panic!("no row {subject}"));
        row.action = TodoAction::Edit;
    }
    if let Some(onto) = onto {
        params.onto = id(repo, onto);
    }
    let journal_file = graft_ops::journal::journal_path(&handle)
        .unwrap_or_else(|why| panic!("the journal path: {why:?}"));
    let mut journal = graft_ops::journal::Journal::open(&journal_file)
        .unwrap_or_else(|why| panic!("the journal opens: {why:?}"));
    let mut ctx = Context::open(
        &handle,
        version,
        &mut journal,
        1_600_000_000_000,
        [7; 10],
        &(),
    )
    .unwrap_or_else(|why| panic!("the context opens: {why:?}"));
    match apply(&mut ctx, &Operation::Rewrite(params)) {
        Err(ApplyError::Conflicted { .. }) => {}
        Err(other) => panic!("expected a stop, got {}", other.explain()),
        Ok(applied) => panic!("expected a stop, it finished: {:?}", applied.entry.status),
    }
}

/// Graft opened on the stop: the stop landed with its rewrite read, the
/// journal's live entry is in, and the history and refs are drawn.
fn relaunched(repo: &Path) -> (App, AppState) {
    let (app, state) = App::open_with_clock(
        repo,
        graft::session::Clock::Fixed(graft_fixtures::CORPUS_NOW),
    );
    let state = app.resize(&state, COLUMNS, ROWS);
    let state = app.pump(state, "the stop, read", |state| {
        state
            .stopped
            .as_ref()
            .is_some_and(|stop| stop.rewrite.is_some())
            && !state.live_entries.is_empty()
            && state.refs.is_some()
            && state.history.as_ref().is_some_and(|h| h.window.is_some())
    });
    (app, state)
}

fn words(state: &AppState) -> String {
    banner_line(
        &banner_view(state).unwrap_or_else(|| panic!("a rebase is live")),
        u16::MAX,
    )
}

fn frame(state: &AppState) -> String {
    let lines = crate::captures::frame_at(
        state,
        COLUMNS,
        ROWS,
        Some(graft::session::Clock::Fixed(graft_fixtures::CORPUS_NOW)),
    );
    format!("{}\n", lines.join("\n"))
}

fn rebase_merge(repo: &Path) -> bool {
    repo.join(".git/rebase-merge").is_dir()
}

/// `feature-a`'s line (a1 a2 ← b1 ← c1), `R` from a1 with a1 set to `edit`.
fn stopped_on_a1() -> tempfile::TempDir {
    let dir = line_repo();
    graft_rewrite_stops(dir.path(), "feature-a~1", Some("a1"), None);
    dir
}

#[test]
fn a_relaunch_mid_rewrite_shows_the_chain() {
    let dir = stopped_on_a1();
    let (app, state) = relaunched(dir.path());
    let line = words(&state);
    assert!(
        line.starts_with("REBASING  step 1 of 4 \u{b7} edit "),
        "{line}"
    );
    assert!(
        line.contains(" a1 \u{b7} feature-a, branch 1 of 3 \u{b7} "),
        "{line}"
    );
    // An `edit` stop: the graph has the keyboard (the relaunch site).
    assert!(!state.banner_focus);
    let body = frame(&state);
    assert!(body.contains("HEAD \u{b7} stopped here"), "{body}");
    assert!(body.contains("a amend it"), "{body}");
    drop(state);
    app.close();
}

/// Rule 6: the user judges it. Right: the header reads `REBASING  step 1 of
/// 4 · edit <sha7> a1 · feature-a, branch 1 of 3` and as much of the hint as
/// fits; `a1`'s row carries `HEAD · stopped here`; the bar leads with
/// `a amend it`. Colour unverified (rule 5).
#[test]
fn capture_edit_stop() {
    let dir = stopped_on_a1();
    let (app, state) = relaunched(dir.path());
    crate::captures::check_shot("m7/rewrite-edit-stop-120x40.txt", &frame(&state));
    drop(state);
    app.close();
}

/// A rebase begun in the shell has no plan of Graft's: no chain words, and
/// the edit stop still gives the graph the keyboard.
#[test]
fn a_shell_begun_rebase_has_no_chain_words() {
    let dir = line_repo();
    let repo = dir.path();
    git(
        repo,
        &[
            "-c",
            "sequence.editor=sed -i.bak -e 1s/^pick/edit/",
            "rebase",
            "-q",
            "-i",
            "--update-refs",
            "feature-a~2",
        ],
    );
    assert!(rebase_merge(repo), "git stopped on the edit");
    let (app, state) = App::open(repo);
    let state = app.pump(state, "the stop, read", |state| {
        state
            .stopped
            .as_ref()
            .is_some_and(|stop| stop.rewrite.is_some())
    });
    let line = words(&state);
    assert!(line.starts_with("REBASING  step 1 of "), "{line}");
    assert!(line.contains(" \u{b7} edit "), "{line}");
    assert!(!line.contains("branch 1 of"), "{line}");
    assert!(!state.banner_focus);
    drop(state);
    app.close();
}

/// `a`, `a`, `c` from the relaunch: two amends at the stop, the message
/// kept, then the rewrite runs home over the amended commit.
#[test]
fn a_twice_then_c_amends_the_stopped_commit_and_the_rewrite_goes_home() {
    let dir = stopped_on_a1();
    let repo = dir.path();
    let (app, state) = relaunched(repo);
    let stopped = id(repo, "HEAD");

    let mut state = state;
    for round in ["a1 once\n", "a1 twice\n"] {
        std::fs::write(repo.join("a1.txt"), round).unwrap_or_else(|why| panic!("write: {why:?}"));
        git(repo, &["add", "a1.txt"]);
        let asked = app.key(&state, KeyCode::Char('a'));
        assert!(asked.commit_editor.is_none(), "no editor");
        let asked = app.pump(asked, "the amend queued", |state| state.operating.is_some());
        state = app.settle_write(asked, "the amend");
        assert!(
            matches!(&state.status, graft::update::Status::Done { what, .. } if what == "amended the stopped commit"),
            "{:?}",
            state.status
        );
        assert!(rebase_merge(repo), "still stopped");
        assert_eq!(
            git(repo, &["log", "-1", "--format=%s"]).trim(),
            "a1",
            "the message is kept"
        );
        assert_eq!(id(repo, "HEAD^"), id(repo, &format!("{stopped}^")));
    }

    let state = app.pump(state, "the files re-read", |state| {
        state.status_read.is_none()
    });
    let state = app.key(&state, KeyCode::Char('c'));
    let state = app.pump(state, "the rewrite home", |state| {
        state.operating.is_none() && state.in_progress.is_none()
    });
    assert!(!rebase_merge(repo), "the rewrite finished");
    assert_eq!(
        git(repo, &["show", "feature-c:a1.txt"]),
        "a1 twice\n",
        "the line is built on the amended commit"
    );
    drop(state);
    app.close();
}

/// Settled 1 (regression, green on arrival): `Esc` never aborts a rewrite
/// stop — on the graph, on the banner, in the resolver. After each, git is
/// still stopped and the journal's live entry is still the rewrite's.
#[test]
fn esc_never_aborts_a_rewrite_stop() {
    fn esc_keeps_it(repo: &Path, state: &AppState, where_: &str) -> AppState {
        let (after, commands) =
            press_for_commands(state, graft_core::event::KeyPress::plain(KeyCode::Escape));
        assert!(
            !format!("{commands:?}").contains("Abort"),
            "Esc {where_}: {commands:?}"
        );
        assert!(rebase_merge(repo), "Esc {where_}: git is still stopped");
        assert!(
            !after.live_entries.is_empty(),
            "Esc {where_}: the live entry stays"
        );
        after
    }

    // An `edit` stop: the graph, then the banner.
    let dir = stopped_on_a1();
    let repo = dir.path();
    let (app, state) = relaunched(repo);
    let state = esc_keeps_it(repo, &state, "on the graph");
    let state = crate::resolver::on_the_banner(&app, state);
    let state = esc_keeps_it(repo, &state, "on the banner");
    drop(state);
    app.close();

    // A conflict stop of Graft's rewrite: the resolver.
    let dir = conflicted();
    let repo = dir.path();
    graft_rewrite_stops(repo, "feature-a", None, Some("main"));
    let (app, state) = relaunched(repo);
    let state = crate::resolver::resolver_on(&app, state, "f.txt");
    let state = esc_keeps_it(repo, &state, "in the resolver");
    drop(state);
    app.close();
}

/// `main` and `feature-a` (one commit) change `f.txt`'s one line apart.
fn conflicted() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap_or_else(|why| panic!("a temporary directory: {why:?}"));
    let repo = dir.path();
    let run = |args: &[&str]| {
        graft_fixtures::git::run_at(repo, args, "2020-01-01T01:00:00+00:00")
            .unwrap_or_else(|why| panic!("git {args:?}: {why}"));
    };
    let commit = |text: &str, subject: &str| {
        std::fs::write(repo.join("f.txt"), text).unwrap_or_else(|why| panic!("write: {why:?}"));
        run(&["add", "-A"]);
        run(&["commit", "-q", "-m", subject]);
    };
    run(&["init", "-q", "-b", "main"]);
    commit("base\n", "m1");
    run(&["switch", "-q", "-c", "feature-a"]);
    commit("theirs\n", "a1");
    run(&["switch", "-q", "main"]);
    commit("ours\n", "m2");
    run(&["switch", "-q", "feature-a"]);
    dir
}
