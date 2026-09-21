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
use std::time::Duration;

use graft::update::AppState;
use graft_core::capabilities::{
    Capabilities, CellSize, CellSizeSource, ColorDepth, ProbeOutcome, RenderingTier, Rgb,
    Supported, Theme as TerminalTheme, ThemeHint, ThemeSource, TmuxStatus,
};
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

/// A plain truecolour terminal that answered every probe. `Capabilities` has
/// no `Default` on purpose (an unprobed terminal is not a neutral one), so
/// the literal lives here the way `frames.rs` keeps its own.
pub fn capabilities() -> Capabilities {
    Capabilities {
        tier: RenderingTier::Unicode,
        pixel_protocol: None,
        kitty_graphics: Supported::No,
        sixel: Supported::No,
        iterm2_images: false,
        kitty_keyboard: Supported::No,
        synchronized_output: Supported::Yes,
        cell_size: CellSize {
            width: 9,
            height: 18,
            source: CellSizeSource::Reported,
        },
        theme: ThemeHint {
            theme: TerminalTheme::Dark,
            source: ThemeSource::Background(Rgb {
                red: 0x10,
                green: 0x10,
                blue: 0x14,
            }),
        },
        color_depth: ColorDepth::TrueColor,
        no_color: false,
        is_tty: true,
        over_ssh: false,
        tmux: TmuxStatus::Outside,
        probe: ProbeOutcome::Answered,
        probe_budget: Duration::from_millis(250),
    }
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
/// own rows come after. So it is found by its trailer instead of by
/// position — every list the width rule cuts ends in `. more`.
fn hint_bar(state: &AppState, columns: u16, rows: u16) -> Option<String> {
    frame(state, columns, rows)
        .into_iter()
        .rev()
        .find(|line| line.ends_with(". more"))
}

/// Opened, with the first rows, the refs and the working-tree status on
/// hand — everything `hint_context` reads.
fn opened(app: &App, state: AppState) -> AppState {
    app.pump(state, "the first rows, the refs and the status", |state| {
        state.refs.is_some()
            && state.working_tree.is_some()
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
        bar.starts_with(" ⏎ open  c commit  P push") && bar.ends_with(". more"),
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
        bar.starts_with(" ⏎ open  r rebase  c commit") && bar.ends_with(". more"),
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
        bar.starts_with(" ⏎ changes  c commit  S stash") && bar.ends_with(". more"),
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
        beside, " space stage  d discard  c commit  <> resize  ⌃A all  e edit  ⇥ graph  . more",
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

/// A gap this ticket found and did not fix, kept as a failing test rather
/// than a paragraph: under 100 columns the detail pane takes the whole
/// screen behind a `[Graph] Detail` tab strip
/// (`update::detail::effective_layout` → `DetailLayout::Tab`), the graph
/// screen is not drawn at all, and with it goes the hint bar — so at 80
/// columns, with the changes pane open, **every** key is hidden. That is
/// the one outcome ADR-0037 §4 rules out, and 80 columns is an everyday
/// width in an editor's terminal.
///
/// Fixing it means giving the tab layout a footer of its own, which changes
/// the pane's height for every pane and is a design decision, not this fix
/// round's (see the ticket's NOTES).
#[test]
#[ignore = "known gap: the Tab detail layout draws no hint bar (m5-03 fix round)"]
fn the_tab_layout_still_hides_every_key() {
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
/// Run by hand: `cargo test -p graft --test it
/// hint_bar::capture_hint_bar_for_docs_shots -- --ignored --nocapture`.
#[test]
#[ignore = "writes docs/shots/m5-hints-*.txt; run by hand to regenerate"]
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
    // The scratch repository lives under a different temporary directory
    // every run, and the header row draws its path: replaced so that
    // regenerating the captures shows only what actually changed.
    let home = scratch.path().display().to_string();

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
            let bar = drawn.iter().rev().find(|line| line.ends_with(". more"));
            let mut out = format!(
                "# m5-hints-{name}-{label}.txt: {columns}x{rows}, drawn by `view::view` from\n\
                 # a real AppState — a real repository, opened over the real job pool, with\n\
                 # the keys pressed through `update` (crates/graft/tests/it/hint_bar.rs,\n\
                 # capture_hint_bar_for_docs_shots, #[ignore]d). Not a pty capture: the\n\
                 # terminal is ratatui's TestBackend, so this is a character grid with no\n\
                 # colour in it (CLAUDE.md rule 5 — colour ships unverified). The scratch\n\
                 # repository's temporary path is replaced by `/code/api` so the capture is\n\
                 # the same from one run to the next; nothing else is touched.\n\
                 #\n\
                 # Here {what}.\n\
                 #\n\
                 # Judge by eye: the hint bar is the graph's bottom row — the frame's last\n\
                 # row, unless the pane is stacked under it. `. more` is always last and\n\
                 # is never dropped (ADR-0037 §4).\n\
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
                 # Floor: the three most relevant never fall below their short rung, the\n\
                 # bar is never all hidden, never fewer than three hints, never a key\n\
                 # that does nothing, and `.` leads to everything the width dropped. The\n\
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
                out.push_str(&line.replace(&home, "/code/api"));
                out.push('\n');
            }
            let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../docs/shots")
                .join(format!("m5-hints-{name}-{label}.txt"));
            write(&path, &out);
            println!("wrote {} ({} bytes)", path.display(), out.len());
        }
    }
    app.close();
}

/// What a capture says when the frame it recorded has no hint bar at all:
/// the truth, and that it is a gap rather than the shape to judge by eye
/// (CLAUDE.md rule 1).
const MISSING_BAR: &str = "#\n\
     # NOTE: this frame has no hint bar at all. Under 100 columns the detail\n\
     # pane takes the whole screen behind the `[Graph] Detail` tabs, and the\n\
     # graph screen — the bar with it — is not drawn. A keyless screen is the\n\
     # one outcome ADR-0037 §4 rules out, so this is a known gap, not what\n\
     # right looks like (hint_bar::the_tab_layout_still_hides_every_key,\n\
     # #[ignore]d).\n";

fn write(path: &Path, text: &str) {
    std::fs::write(path, text).unwrap_or_else(|why| panic!("writing {}: {why}", path.display()));
}
