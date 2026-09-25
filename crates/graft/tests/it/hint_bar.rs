//! The hint bar, from a real repository through the real app
//! (`hint-bar-redesign`, ADR-0037 §4; m5-03 review, Critical 1 and Major 5).
//!
//! Everything else about the bar is tested in `graft_ui`, on a `HintContext`
//! set by hand. That proves the footer draws a context correctly and proves
//! nothing about which context the app *picks* — which is where the changes
//! pane lost its own keys for a whole milestone: `view::rows::hint_context`
//! tested `on_working_tree` before `on_changes_pane()`, and the pane is open
//! with both true.
//!
//! So these drive the whole path — real `git`, real job pool, real keys,
//! real frame — and read the footer off the drawn buffer.
#![allow(
    clippy::expect_used,
    reason = "a test fixture that cannot be built has nothing to test; the panic names why"
)]

use std::path::Path;

use graft::update::AppState;
use graft_core::event::KeyCode;

use crate::app_harness::{App, changes_pane, git};

/// A repository with two commits on the current branch and a dirty tracked
/// file, so that every row context the graph can be in is one key away: the
/// tip (the current branch's badge), the commit under it (a plain row) and
/// the uncommitted-changes row above them.
fn repo() -> graft_fixtures::Scratch {
    let scratch = graft_fixtures::copy_into_temp(graft_fixtures::Shape::Trivial);
    let path = scratch.path();
    std::fs::write(path.join("first.txt"), "one\n").expect("the scratch repo is writable");
    git(path, &["add", "first.txt"]);
    dated_commit(path, "add first", 1);
    std::fs::write(path.join("second.txt"), "two\n").expect("the scratch repo is writable");
    git(path, &["add", "second.txt"]);
    dated_commit(path, "add second", 2);
    std::fs::write(path.join("first.txt"), "one\nedited\n").expect("the scratch repo is writable");
    scratch
}

/// The instant [`repo`]'s first commit is stamped at, Unix seconds; one
/// minute per commit after it. Without it the fixture was built at
/// wall-clock time, so the hint-bar captures' commit ids and ages differed
/// on every regeneration.
const HINTS_EPOCH: i64 = 1_609_459_200;

/// The instant the hint-bar captures' relative dates are drawn against.
const HINTS_NOW: i64 = HINTS_EPOCH + 10 * 60;

/// One fixture commit, stamped `minute` minutes after [`HINTS_EPOCH`].
fn dated_commit(repo: &Path, subject: &str, minute: i64) {
    let when = format!("@{} +0000", HINTS_EPOCH + minute * 60);
    graft_fixtures::git::run_at(repo, &["commit", "-m", subject], &when)
        .unwrap_or_else(|why| panic!("committing {subject:?}: {why}"));
}

/// Every row of the frame `view::view` draws for `state` at this size.
fn frame(state: &AppState, columns: u16, rows: u16) -> Vec<String> {
    crate::captures::frame(state, columns, rows)
}

/// The footer's bottom line: the frame's last row, as drawn. Right while
/// the detail pane is closed or beside the graph; see [`hint_bar`] for the
/// general case.
fn footer(state: &AppState, columns: u16, rows: u16) -> String {
    frame(state, columns, rows)
        .last()
        .expect("a frame has rows")
        .clone()
}

/// The hint bar as drawn, or `None` when the frame has none.
///
/// The bar is the last row of the *graph* area, which is the frame's last
/// row only while the detail pane is closed or beside the graph: stacked
/// below it (`DetailLayout::Below`, the layout at 120 columns) the pane's
/// own rows come after. So it is found by its doorway instead of by
/// position — every list the width rule cuts carries `. more`, though since
/// m5/28 it leads the bare keys rather than ending the row (ADR-0039), so
/// this looks for it anywhere on the line.
fn hint_bar(state: &AppState, columns: u16, rows: u16) -> Option<String> {
    frame(state, columns, rows)
        .into_iter()
        .rev()
        .find(|line| line.contains(". more"))
}

/// Opened, with the first rows, the refs and the working-tree status on
/// hand — everything `hint_context` reads.
fn opened(app: &App, state: AppState) -> AppState {
    app.pump(state, "the first rows, the refs and the status", |state| {
        state.refs.is_some()
            && state.working_tree.is_some()
            // The footer's commit-graph line waits on `git --version`.
            && state.system_git.is_some()
            && state
                .history
                .as_ref()
                .and_then(|history| history.window.as_ref())
                .is_some_and(|window| !window.rows.is_empty())
    })
}

/// The cursor starts on the tip, which carries the current branch's badge:
/// the footer hints that row's own keys, and none of the keys no group has
/// built (`b branch`, `t tag` are the design note's, not this build's;
/// `m merge` and `r rebase` were too until group 08 bound them).
#[test]
fn the_tip_row_hints_the_current_branch_context() {
    let scratch = repo();
    let (app, state) = App::open(scratch.path());
    let state = opened(&app, state);

    let bar = footer(&state, 80, 24);
    assert!(
        bar.starts_with(" ⏎ open  c commit  P push  . more") && !bar.ends_with(". more"),
        "{bar:?}"
    );
    app.close();
}

/// One row down is an ordinary commit: a different list, from the same
/// frame path.
#[test]
fn a_plain_row_hints_the_plain_context() {
    let scratch = repo();
    let (app, state) = App::open(scratch.path());
    let state = opened(&app, state);
    let state = app.key(&state, KeyCode::Char('j'));

    let bar = footer(&state, 80, 24);
    assert!(
        // `b branch` and `t tag` went live in
        // m5-history-and-ghost-preview/10, in the hint-bar design note's own
        // order for a plain row; `c commit` dropped to its bare key.
        bar.starts_with(" ⏎ open  b branch  r rebase  . more") && !bar.ends_with(". more"),
        "{bar:?}"
    );
    app.close();
}

/// `k` from the top row selects the uncommitted-changes row, and the footer
/// follows it.
#[test]
fn the_uncommitted_row_hints_the_working_tree_context() {
    let scratch = repo();
    let (app, state) = App::open(scratch.path());
    let state = opened(&app, state);
    let state = app.key(&state, KeyCode::Char('k'));
    assert!(state.on_working_tree, "`k` selects the uncommitted row");

    let bar = footer(&state, 80, 24);
    assert!(
        bar.starts_with(" ⏎ changes  c commit  S stash  . more") && !bar.ends_with(". more"),
        "{bar:?}"
    );
    app.close();
}

/// **Critical 1.** With the changes pane open the footer hints the keys the
/// pane answers — `space stage`, `d discard` — and not `⏎ changes`, which
/// would tell the user to open the pane they are standing in. The pane opens
/// *on* the uncommitted-changes row and the cursor stays there, so this is
/// the state where `on_working_tree` and `on_changes_pane()` are both true:
/// the ordering bug this fixes.
///
/// Three widths because the pane lands somewhere different at each, and the
/// bar is only ever as wide as the graph column beside it: at 120 the pane
/// is stacked below and the bar spans the terminal; at 160 it is beside the
/// graph, leaving the graph — and so the bar — under 100 cells.
#[test]
fn the_changes_pane_hints_the_keys_that_write() {
    let scratch = repo();
    let (app, state) = App::open(scratch.path());
    let state = opened(&app, state);
    let state = changes_pane(&app, state);
    assert!(
        state.on_working_tree && state.on_changes_pane(),
        "the pane opens on the uncommitted row and the cursor stays there"
    );

    // Every width leads with the three keys that write, and `<> resize`
    // sits fourth while a pane is open (m5-03 review, Major 4 — the
    // divider hint the old list dropped).
    for (columns, rows) in [(120u16, 40u16), (160, 48), (200, 60)] {
        let bar = hint_bar(&state, columns, rows).unwrap_or_default();
        assert!(
            bar.starts_with(" space stage  d discard  c commit  <> resize")
                && bar.ends_with(". more"),
            "at {columns}x{rows}: {bar:?}"
        );
    }

    // **The defect this ticket fixed, at the width that showed it.** Beside
    // the graph the bar has about 93 cells, and the old `width_tier` read
    // that as "narrow" and capped the list at three — so the 160-column
    // terminal taught three keys where the 120-column one taught six. The
    // bar counts from its own row now (ADR-0039, 2026-09-21 amendment).
    let beside = hint_bar(&state, 160, 48).unwrap_or_default();
    let drawn = beside
        .split("  ")
        .filter(|part| !part.trim().is_empty())
        .count();
    assert!(
        drawn > 4,
        "93 cells fit more than three hints and `. more`: {beside:?}"
    );
    // Counting parts would still pass with half the write keys gone
    // (m5/14-22 review, Minor 1), so name them: 93 cells fit the changes
    // pane's whole vocabulary, in the order the tier hands it out.
    assert_eq!(
        beside, " space stage  d discard  c commit  <> resize  ⌃A all  e edit  ⇥ diff  . more",
        "every key the pane answers, beside the graph"
    );

    for (columns, rows) in [(120u16, 40u16), (160, 48), (200, 60)] {
        let bar = hint_bar(&state, columns, rows).unwrap_or_default();
        assert!(
            !bar.contains("⏎ changes"),
            "the pane is already open at {columns}x{rows}: {bar:?}"
        );
    }
    app.close();
}

/// Below 100 columns the shown tab owns the whole screen
/// (`update::detail::effective_layout` → `DetailLayout::Tab`) and the graph
/// screen — footer and all — is not drawn behind it. So the detail pane
/// reserves a footer row of its own and draws **its** keys into it
/// (m5-history-and-ghost-preview/29): at 80 columns with the changes pane
/// open the bar is present, and it hints the pane's vocabulary rather than
/// the graph's. Every key hidden is the one outcome ADR-0037 §4 rules out,
/// and 80 columns is an everyday width in an editor's terminal.
#[test]
fn the_tab_layout_draws_the_panes_own_keys() {
    let scratch = repo();
    let (app, state) = App::open(scratch.path());
    let state = opened(&app, state);
    let state = changes_pane(&app, state);

    let bar = hint_bar(&state, 80, 24);
    assert!(
        bar.is_some(),
        "the bar is never all hidden (ADR-0037 §4); the frame was {:?}",
        frame(&state, 80, 24)
    );
    let bar = bar.unwrap_or_default();
    assert!(
        bar.starts_with(" space stage  d discard  c commit"),
        "the pane that owns the screen owns the keys: {bar:?}"
    );
    // The pane's own row, not one borrowed from a graph that is not drawn:
    // it is the frame's last row, under the pane.
    let drawn = frame(&state, 80, 24);
    assert_eq!(
        drawn.last().map(String::as_str),
        Some(bar.as_str()),
        "the reserved row is the frame's last: {drawn:?}"
    );
    app.close();
}

/// Every pane context that can own the Tab layout draws the bar, not just
/// the changes pane (m5-history-and-ghost-preview/29). Three states the
/// user reaches with one key each, below 100 columns:
///
/// - the Detail tab on the tip, whose row carries the current branch's
///   badge (`HintContext::BranchRow { current: true }`);
/// - the Detail tab on an ordinary commit one row down
///   (`HintContext::PlainRow`);
/// - the changes pane (`HintContext::ChangesPane`), which
///   [`the_tab_layout_draws_the_panes_own_keys`] pins in detail.
///
/// And the fourth arm of the same layout: the **Graph** tab, where the
/// graph screen is drawn and carries its own footer — the pane reserves
/// nothing there, so the row is not spent twice.
///
/// `Journal` and `Stash` are unreachable today (those overlays draw their
/// own footer) and `Banner` is modal: a focused banner already draws its
/// keys on the bottom row, which
/// `update::tests::the_focused_banner_keeps_the_only_hint_row` covers.
#[test]
fn every_tab_context_draws_a_bar() {
    let scratch = repo();
    let (app, state) = App::open(scratch.path());
    let opened_state = opened(&app, state);

    // The Detail tab on the tip: the current branch's row.
    let tip = app.key(&opened_state, KeyCode::Enter);
    let tip = app.pump(tip, "the tip's detail", |state| state.detail.shown);
    let bar = hint_bar(&tip, 80, 24);
    assert!(
        bar.is_some(),
        "the Detail tab on the tip draws a bar; the frame was {:?}",
        frame(&tip, 80, 24)
    );

    // One row down is an ordinary commit.
    let plain = app.key(&opened_state, KeyCode::Char('j'));
    let plain = app.key(&plain, KeyCode::Enter);
    let plain = app.pump(plain, "a plain row's detail", |state| state.detail.shown);
    assert!(
        hint_bar(&plain, 80, 24).is_some(),
        "the Detail tab on a plain row draws a bar; the frame was {:?}",
        frame(&plain, 80, 24)
    );

    // The changes pane.
    let pane = changes_pane(&app, opened_state.clone());
    assert!(
        hint_bar(&pane, 80, 24).is_some(),
        "the changes pane draws a bar; the frame was {:?}",
        frame(&pane, 80, 24)
    );

    // The Graph tab: the graph screen is on frame and its own footer is the
    // bar, so the pane reserves no second row.
    let graph_tab = app.key(&pane, KeyCode::Tab);
    assert!(
        hint_bar(&graph_tab, 80, 24).is_some(),
        "the Graph tab keeps the graph's own footer; the frame was {:?}",
        frame(&graph_tab, 80, 24)
    );
    app.close();
}

/// **Major 2.** A failure that carries no keys of its own still names the
/// key that clears it — `p` on a branch with no upstream is the everyday
/// way to land one.
#[test]
fn a_failure_still_names_the_key_that_clears_it() {
    let scratch = repo();
    let (app, state) = App::open(scratch.path());
    let state = opened(&app, state);
    let state = app.pump(state, "the remotes", |state| {
        !matches!(
            state.remotes,
            graft::update::detail::Loading::Idle | graft::update::detail::Loading::Pending(_)
        )
    });
    let state = app.key(&state, KeyCode::Char('p'));
    assert!(
        matches!(state.status, graft::update::Status::Failed(_)),
        "a repository with no remote cannot pull: {:?}",
        state.status
    );

    let line = footer(&state, 80, 24);
    assert!(line.ends_with(" · esc dismiss"), "{line:?}");
    assert!(
        !line.trim().is_empty(),
        "the bar is never keyless (ADR-0037 §4)"
    );
    app.close();
}

/// The captures under `docs/shots/`, from this same real path — the user
/// judges the bar by eye at 80 and 120 columns (CLAUDE.md rule 6).
/// Regenerate on purpose: `GRAFT_UPDATE_SHOTS=1 cargo test -p graft --test it hint_bar::capture_hint_bar_for_docs_shots`.
#[test]
#[allow(
    clippy::too_many_lines,
    reason = "four scenarios and the header each capture explains itself with, not logic"
)]
fn capture_hint_bar_for_docs_shots() {
    let scratch = repo();
    let (app, state) = App::open(scratch.path());
    let opened_state = opened(&app, state);

    let tip = opened_state.clone();
    let plain = app.key(&tip, KeyCode::Char('j'));
    let uncommitted = app.key(&tip, KeyCode::Char('k'));
    let pane = changes_pane(&app, opened_state);
    // The pane open on the tip with its **diff** focused: `⏎` has nothing
    // left to open there and `S`, `J`, `M`, `O` need the graph to have the
    // keyboard, so none of the five is drawn (m5/31).
    let detail = app.key(&tip, KeyCode::Enter);
    let detail = app.pump(detail, "the tip's detail", |state| state.detail.shown);
    let detail = app.key(&detail, KeyCode::Enter);
    let mut drifted = Vec::new();

    for (name, state, what) in [
        (
            "branch",
            &tip,
            "the cursor is on the tip, which carries the current branch's badge",
        ),
        ("plain", &plain, "the cursor is on an ordinary commit row"),
        (
            "uncommitted",
            &uncommitted,
            "the cursor is on the uncommitted-changes row, the pane closed",
        ),
        (
            "changes",
            &pane,
            "the changes pane is open on the working tree",
        ),
        (
            "detail",
            &detail,
            "the pane is open on the tip with its diff focused: at 80 the Detail tab owns the screen, wider it sits beside the graph",
        ),
    ] {
        for (label, columns, rows) in [
            ("80x24", 80u16, 24u16),
            ("120x40", 120, 40),
            ("160x48", 160, 48),
        ] {
            let sized = app.resize(state, columns, rows);
            let drawn = crate::captures::frame_at(
                &sized,
                columns,
                rows,
                Some(graft::session::Clock::Fixed(HINTS_NOW)),
            );
            // The bar is the last row of the *graph* area, which is the
            // frame's last row only while the pane is closed or beside the
            // graph.
            let bar = drawn.iter().rev().find(|line| line.contains(". more"));
            let mut out = format!(
                "# m5-hints-{name}-{label}.txt: {columns}x{rows}, drawn by `view::view` from\n\
                 # a real AppState — a real repository, opened over the real job pool, with\n\
                 # the keys pressed through `update` (crates/graft/tests/it/hint_bar.rs,\n\
                 # capture_hint_bar_for_docs_shots, asserted). Not a pty capture: the\n\
                 # terminal is ratatui's TestBackend, so this is a character grid with no\n\
                 # colour in it (CLAUDE.md rule 5 — colour ships unverified). The scratch\n\
                 # repository's temporary path is replaced by `/code/api` so the capture is\n\
                 # the same from one run to the next; nothing else is touched.\n\
                 #\n\
                 # Here {what}.\n\
                 #\n\
                 # Judge by eye: the hint bar is the bottom row of whatever owns the\n\
                 # screen — the graph's, or (below 100 columns, with the Detail tab up)\n\
                 # the pane's own reserved row, which costs the pane one row of content\n\
                 # and is what keeps every key from being hidden. `. more` is never dropped\n\
                 # (ADR-0037 §4) and sits at the head of the bare keys — the end of the\n\
                 # row where there are none (ADR-0039, m5/28).\n\
                 #\n\
                 # What fills the rest is a **ladder, taken per hint** (ADR-0039's third\n\
                 # amendment, 2026-09-21). A hint has up to three rungs — full `y sha`,\n\
                 # short `F3 rend`, bare key `x`. The bar fills from the most relevant\n\
                 # end, and each hint takes the best rung that still leaves room for the\n\
                 # hints ahead of it; a hint that will not fit even at its bare key is\n\
                 # dropped. So the shape to expect is readable verbs at the left thinning\n\
                 # into a tail of bare keys at the right, the tail turning back into\n\
                 # verbs as the row gets wider. Nothing steps down as a group: one hint's\n\
                 # rung says nothing about its neighbour's.\n\
                 #\n\
                 # The short rungs come from a table the user wrote, not a scheme. A verb\n\
                 # it does not name has no short rung and goes straight from full to bare\n\
                 # key: `m merge`, `r rebase` and `S stash` are the three to watch. A\n\
                 # squeezed `S stsh` anywhere in these captures is a bug.\n\
                 #\n\
                 # Floor: the three most relevant never fall below their floor rung,\n\
                 # which is their short rung where the table above names one and their\n\
                 # full verb where it does not. The bar is never all hidden, never fewer\n\
                 # than three hints, never a key that does nothing, and `.` leads to\n\
                 # everything the width dropped. The\n\
                 # room is counted off the bar's own width, not the screen's, so a\n\
                 # 160-column frame with the pane beside it fills the ~92 cells the bar\n\
                 # is left rather than falling back to three.\n\
                 #\n\
                 # The open question, and why this is a capture rather than a test\n\
                 # (CLAUDE.md rule 6): does a tail of bare keys after readable leading\n\
                 # verbs earn its place, or should those hints simply be dropped? Built\n\
                 # with the bare-key rung in, for the user to answer by eye.\n\
                 #\n\
                 # The app is resized to the capture's size before the frame is drawn:\n\
                 # one state photographed at three sizes is a picture of at most one of\n\
                 # them.\n"
            );
            if bar.is_none() {
                out.push_str(MISSING_BAR);
            }
            out.push('\n');
            for line in &drawn {
                out.push_str(line);
                out.push('\n');
            }
            let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../docs/shots")
                .join(format!("m5-hints-{name}-{label}.txt"));
            if let Err(why) = graft_fixtures::check(&path, &out) {
                drifted.push(why);
            }
        }
    }
    app.close();
    assert!(drifted.is_empty(), "{}", drifted.join("\n\n"));
}

/// What a capture says when the frame it recorded has no hint bar at all:
/// the truth, and that it is a gap rather than the shape to judge by eye
/// (CLAUDE.md rule 1).
///
/// No scenario here reaches it since m5-history-and-ghost-preview/29 gave
/// the sub-100-column tab layout a reserved row of its own. It stays
/// because a capture must be able to say so if one ever does — a picture
/// that quietly omits the bar is a picture that lies.
const MISSING_BAR: &str = "#\n\
     # NOTE: this frame has no hint bar at all. That is the one outcome\n\
     # ADR-0037 §4 rules out, so it is a defect rather than the shape to\n\
     # judge by eye: every layout carries a bar of its own\n\
     # (hint_bar::the_tab_layout_draws_the_panes_own_keys,\n\
     # hint_bar::every_tab_context_draws_a_bar).\n";

/// **The Detail tab draws no key the focused pane refuses** (m5/31, review
/// Q2). Two presses of `⏎` from a fresh screen: the first opens the pane on
/// the tip, the second moves focus from the file list to the diff — and
/// there `detail::keys::enter_opens` is false, so `⏎` is consumed and does
/// nothing. The bar drew `⏎ open` anyway until this ticket.
///
/// Before the fix, at 80 columns, the bar read
/// `⏎ open  c commit  P push  . more  p  f  b  t  u  S  J  /  q  F2  F3  x  +  j/k  space`;
/// the four keys that need the graph to have the keyboard (`S`, `J`, `M`,
/// `O`) were dead there too. The doorway stays: `.` still leads to
/// everything hidden, which is what makes filtering honest (ADR-0039).
/// **The Detail tab with an overlay behind it draws the keys that still
/// work, not the graph's** (m5/42; the review's Q1, which m5/36 confirmed).
///
/// Driven the way the user reaches it: open the pane, `Tab` back to the
/// graph, `J` to open the journal browser over it, `Tab` to the Detail tab.
/// The graph is gone from under the bar — `journal::shown_key` claims every
/// key `detail_key` has not already taken — so a bar of `b t c m r y` would
/// be nine dead verbs, which is exactly what it was.
///
/// Only the frame can prove this: the context is picked in `view::rows` and
/// drawn in `view::frame::draw_tab_hints`, and nothing but a drawn buffer
/// shows that the two met.
#[test]
fn the_bar_behind_an_overlay_draws_what_the_pane_still_answers() {
    use graft_ui::detail::DetailFocus;

    let scratch = repo();
    let (app, state) = App::open(scratch.path());
    let state = opened(&app, state);

    let pane = app.key(&state, KeyCode::Enter);
    let mut pane = app.pump(pane, "the tip's detail", |state| state.detail.shown);
    // Back to the graph, which is what `J` asks for (`journal_key` reads
    // `AppState::graph_has_focus`).
    for _ in 0..3 {
        if pane.detail.focus == DetailFocus::Graph {
            break;
        }
        pane = app.key(&pane, KeyCode::Tab);
    }
    assert_eq!(
        pane.detail.focus,
        DetailFocus::Graph,
        "`Tab` cycles back to the graph with the pane open"
    );
    let journal = app.key(&pane, KeyCode::Char('J'));
    assert!(
        journal.journal.shown,
        "`J` opens the browser over the graph"
    );
    let detail_tab = app.key(&journal, KeyCode::Tab);
    assert!(
        detail_tab.journal.shown && detail_tab.detail.focus == DetailFocus::Files,
        "`Tab` shows the Detail tab with the browser still open"
    );

    // The bar is the frame's last row in the `Tab` layout, and it carries no
    // `. more`: `.` reaches the browser, which swallows it.
    let drawn = frame(&detail_tab, 80, 24);
    let bar = drawn.last().cloned().unwrap_or_default();
    assert_eq!(
        bar.trim_end(),
        " ⏎ open  j/k move  u undo  / search",
        "the keys the pane and the globals still answer, quoted so the change is legible \
         in review: {drawn:?}"
    );
    for dead in [
        "b", "t", "c", "m", "r", "y", "S", "J", "M", "q", "x", "+", ".",
    ] {
        assert!(
            !bar.split_whitespace().any(|word| word == dead),
            "`{dead}` reaches the browser, which does nothing with it: {bar:?}"
        );
    }

    // And the overlay still claims them: what changed is the bar, not what
    // the browser answers (`update::journal::an_open_browser_swallows_the_\
    // graphs_own_keys` is the same claim at the handler).
    let pressed = app.key(&detail_tab, KeyCode::Char('c'));
    assert!(
        pressed.commit_editor.is_none() && pressed.journal.shown,
        "`c` is swallowed by the browser, not passed to the graph"
    );
    app.close();
}

#[test]
fn the_detail_tab_hides_the_keys_a_focused_pane_does_not_answer() {
    let scratch = repo();
    let (app, state) = App::open(scratch.path());
    let state = opened(&app, state);

    // The graph's own row, where `⏎ open` is the key that opens the pane.
    let on_the_graph = hint_bar(&state, 80, 24).expect("the graph row draws a bar");
    assert!(
        on_the_graph.contains("⏎ open"),
        "the filter must not reach the row that answers `⏎`: {on_the_graph:?}"
    );

    let pane = app.key(&state, KeyCode::Enter);
    let pane = app.pump(pane, "the tip's detail", |state| state.detail.shown);
    let diff = app.key(&pane, KeyCode::Enter);
    assert_eq!(
        diff.detail.focus,
        graft_ui::detail::DetailFocus::Diff,
        "the second `⏎` moves focus to the diff"
    );

    let bar = hint_bar(&diff, 80, 24).expect("the Detail tab draws a bar");
    assert_eq!(
        bar, " c commit  P push  p pull  . more  f  b  t  u  /  q  F2  F3  x  +  j/k  space  1",
        "the bar after the filter, quoted so the change is legible in review"
    );
    assert!(
        !bar.contains('⏎'),
        "`⏎` is consumed and does nothing with the diff focused: {bar:?}"
    );
    for dead in ["S", "J", "M", "O"] {
        assert!(
            !bar.split_whitespace().any(|word| word == dead),
            "`{dead}` needs the graph to have the keyboard: {bar:?}"
        );
    }
    assert!(
        bar.contains(". more"),
        "the doorway is never dropped, so nothing filtered is lost: {bar:?}"
    );
    app.close();
}
