//! m8-polish-and-ship/17 (G-look polish): captures for the user's eye.

#![allow(
    clippy::expect_used,
    reason = "a test fixture that cannot be built has nothing to test; the panic names why"
)]

use graft::update::AppState;
use graft_core::event::KeyCode;
use graft_ui::detail::DetailFocus;

use crate::app_harness::{App, git};

/// A two-commit repo whose tip changes only whitespace in one line and a
/// word in another, so `w` visibly changes the diff.
pub(crate) fn repo() -> graft_fixtures::Scratch {
    let scratch = graft_fixtures::copy_into_temp(graft_fixtures::Shape::Trivial);
    let path = scratch.path();
    std::fs::write(path.join("notes.txt"), "alpha beta\ngamma\n").expect("writable");
    git(path, &["add", "notes.txt"]);
    graft_fixtures::git::run_at(path, &["commit", "-m", "add notes"], "@1609459260 +0000")
        .expect("commit");
    std::fs::write(path.join("notes.txt"), "alpha   beta\ndelta\n").expect("writable");
    git(path, &["add", "notes.txt"]);
    graft_fixtures::git::run_at(path, &["commit", "-m", "reflow notes"], "@1609459320 +0000")
        .expect("commit");
    scratch
}

pub(crate) fn opened(app: &App, state: AppState) -> AppState {
    app.pump(state, "the first rows and the refs", |state| {
        state.refs.is_some()
            && state.working_tree.is_some()
            && state.system_git.is_some()
            && state
                .history
                .as_ref()
                .and_then(|history| history.window.as_ref())
                .is_some_and(|window| !window.rows.is_empty())
    })
}

/// Item 8: with `w` on, the detail pane's title row ends
/// `· whitespace ignored` (muted; colour unverified, rule 5).
#[test]
fn capture_whitespace_ignored_title_for_docs_shots() {
    let scratch = repo();
    let (app, state) = App::open(scratch.path());
    let state = opened(&app, state);
    let tip = app.key(&state, KeyCode::Enter);
    let mut tip = app.pump(tip, "the tip's detail", |state| state.detail.shown);
    for _ in 0..3 {
        if tip.detail.focus == DetailFocus::Files {
            break;
        }
        tip = app.key(&tip, KeyCode::Tab);
    }
    let on = app.key(&tip, KeyCode::Char('w'));
    assert!(on.detail.options.ignore_whitespace, "`w` turned it on");
    let on = crate::captures::settled_frame(&app, on);
    let sized = app.resize(&on, 120, 36);
    let drawn = crate::captures::frame_at(
        &sized,
        120,
        36,
        Some(graft::session::Clock::Fixed(1_609_459_200 + 600)),
    );
    assert!(
        drawn
            .iter()
            .any(|line| line.contains("\u{b7} whitespace ignored")),
        "{drawn:#?}"
    );
    let mut out = String::from(
        "# m8/polish-whitespace-120x36: drawn by `view::view` from a real AppState\n\
         # (crates/graft/tests/it/look_polish.rs). Not a pty capture: a character grid,\n\
         # no colour (CLAUDE.md rule 5 — colour unverified).\n\
         #\n\
         # The tip's detail pane with `w` on. Right: the pane's title row ends\n\
         # `· whitespace ignored` (muted), right after the title.\n\n",
    );
    for line in &drawn {
        out.push_str(line);
        out.push('\n');
    }
    app.close();
    crate::captures::check_shot("m8/polish-whitespace-120x36.txt", &out);
}

/// `header` then `drawn`, one line each: a capture file's body.
fn shot(header: &str, drawn: &[String]) -> String {
    let mut out = String::from(header);
    for line in drawn {
        out.push_str(line);
        out.push('\n');
    }
    out
}

/// 17b decision 1: `Space` on the corpus octopus folds it; once the count
/// answers, its row ends ` +3` (the three topics it brought in), muted.
#[test]
fn capture_folded_merge_count_for_docs_shots() {
    let scratch = graft_fixtures::copy_into_temp(graft_fixtures::Shape::OctopusMerge);
    let (app, state) = App::open(scratch.path());
    let state = opened(&app, state);
    let state = app.resize(&state, 120, 36);
    let clock = Some(graft::session::Clock::Fixed(graft_fixtures::CORPUS_NOW));
    let folded = app.key(&state, KeyCode::Char(' '));
    let folded = app.pump(folded, "the fold's count", |state| {
        crate::captures::frame_at(state, 120, 36, clock)
            .iter()
            .any(|line| line.contains("Merge branches 'topic-1', 'topic-2' and 'topic-3' +3 "))
    });
    let folded = crate::captures::settled_frame(&app, folded);
    let drawn = crate::captures::frame_at(&folded, 120, 36, clock);
    assert!(
        drawn.iter().any(|line| line.contains("1 merge folded")),
        "{drawn:#?}"
    );
    let out = shot(
        "# m8/polish-fold-120x36: drawn by `view::view` from a real AppState\n\
         # (crates/graft/tests/it/look_polish.rs). Not a pty capture: a character grid,\n\
         # no colour (CLAUDE.md rule 5 — colour unverified).\n\
         #\n\
         # The corpus octopus folded with `Space`. Right: the merge's row reads\n\
         # `Merge branches 'topic-1', 'topic-2' and 'topic-3' +3` — one space, then\n\
         # `+3` (muted), the three topic commits the fold hides; the header says\n\
         # `1 merge folded`. While counting the mark reads `+`; a failure `+?`.\n\n",
        &drawn,
    );
    app.close();
    crate::captures::check_shot("m8/polish-fold-120x36.txt", &out);
}

/// m8/22c (09h's capture box): `?` with the Map open and the graph focused.
/// HERE lists the Map's own keys, EVERYWHERE the Global keys that act there;
/// none of `c commit`, `R`, `m merge`, `/ Find…` (dead at the graph).
#[test]
fn help_at_the_map_lists_only_live_keys() {
    let scratch = repo();
    let (app, state) = App::open(scratch.path());
    let state = opened(&app, state);
    let state = app.resize(&state, 120, 36);
    let map = app.key(&state, KeyCode::Char('M'));
    let map = crate::captures::settled_frame(&app, map);
    let help = app.key(&map, KeyCode::Char('?'));
    let help = crate::captures::settled_frame(&app, help);
    let clock = Some(graft::session::Clock::Fixed(1_609_459_200 + 600));
    let drawn = crate::captures::frame_at(&help, 120, 36, clock);
    for heading in ["HERE", "EVERYWHERE"] {
        assert!(
            drawn.iter().any(|line| line.contains(heading)),
            "{heading}: {drawn:#?}"
        );
    }
    let out = shot(
        "# m8/help-at-map-120x36: drawn by `view::view` from a real AppState\n\
         # (crates/graft/tests/it/look_polish.rs). Not a pty capture: a character grid,\n\
         # no colour (CLAUDE.md rule 5 — colour unverified).\n\
         #\n\
         # The Map open, the graph focused, `?` pressed. Right: HERE lists the Map's own\n\
         # keys; EVERYWHERE the Global keys that act here; no `c commit`, `R`, `m merge`,\n\
         # `/ Find…`.\n\n",
        &drawn,
    );
    app.close();
    crate::captures::check_shot("m8/help-at-map-120x36.txt", &out);
}

/// 17b decision 3: blame's default ignores whitespace, so its title says
/// nothing about it; after `w` it ends `· whitespace counted`.
#[test]
fn capture_blame_whitespace_counted_for_docs_shots() {
    use crate::m7_walk::blame::{blame, blame_ready, drawn, on_line_22, session};

    let scratch = session();
    let (app, state) = on_line_22(scratch.path(), 120, 36);
    let state = blame(&app, &state);
    let before = drawn(&state, 120, 36);
    assert!(
        before[0].contains(" BLAME \u{b7} ") && !before[0].contains("whitespace"),
        "{:?}",
        before[0]
    );
    let counted = app.key(&state, KeyCode::Char('w'));
    let counted = blame_ready(&app, counted);
    let after = drawn(&counted, 120, 36);
    assert!(
        after[0].trim_end().ends_with("\u{b7} whitespace counted"),
        "{:?}",
        after[0]
    );
    let out = shot(
        "# m8/polish-blame-whitespace-counted-120x36: drawn by `view::view` from a real\n\
         # AppState (crates/graft/tests/it/look_polish.rs). Not a pty capture: a character\n\
         # grid, no colour (CLAUDE.md rule 5 — colour unverified).\n\
         #\n\
         # Blame of src/auth/session.rs after `w`. Right: the title ends\n\
         # `· whitespace counted` (muted). At blame's default (whitespace ignored)\n\
         # the title has no whitespace words at all — see docs/shots/m7/blame-*.txt.\n\n",
        &after,
    );
    app.close();
    crate::captures::check_shot("m8/polish-blame-whitespace-counted-120x36.txt", &out);
}
