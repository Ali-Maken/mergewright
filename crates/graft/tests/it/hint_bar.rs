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

use graft::config::Loaded;
use graft::session::Session;
use graft::update::AppState;
use graft_core::capabilities::{
    Capabilities, CellSize, CellSizeSource, ColorDepth, ProbeOutcome, RenderingTier, Rgb,
    Supported, Theme as TerminalTheme, ThemeHint, ThemeSource, TmuxStatus,
};
use graft_core::event::KeyCode;
use ratatui::Terminal;
use ratatui::backend::TestBackend;

use crate::app_harness::{App, changes_pane, git, git_as_graft};

/// A repository with two commits on the current branch and a dirty tracked
/// file, so that every row context the graph can be in is one key away: the
/// tip (the current branch's badge), the commit under it (a plain row) and
/// the uncommitted-changes row above them.
fn repo() -> graft_fixtures::Scratch {
    let scratch = graft_fixtures::copy_into_temp(graft_fixtures::Shape::Trivial);
    let path = scratch.path();
    std::fs::write(path.join("first.txt"), "one\n").expect("the scratch repo is writable");
    git(path, &["add", "first.txt"]);
    git_as_graft(path, &["commit", "-m", "add first"]);
    std::fs::write(path.join("second.txt"), "two\n").expect("the scratch repo is writable");
    git(path, &["add", "second.txt"]);
    git_as_graft(path, &["commit", "-m", "add second"]);
    std::fs::write(path.join("first.txt"), "one\nedited\n").expect("the scratch repo is writable");
    scratch
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
    let session = Session::settle(&Loaded::default(), capabilities());
    let mut state = state.clone();
    state.size = graft::update::Size { columns, rows };
    let mut terminal = Terminal::new(TestBackend::new(columns, rows)).expect("test backend opens");
    terminal
        .draw(|frame| graft::view::view(&session, &state, frame))
        .expect("drawing into a test backend cannot fail");
    let buffer = terminal.backend().buffer();
    (0..buffer.area.height)
        .map(|y| {
            (0..buffer.area.width)
                .map(|x| buffer[(x, y)].symbol())
                .collect::<String>()
                .trim_end()
                .to_owned()
        })
        .collect()
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
/// built (`m merge`, `r rebase` are the design note's, not this build's).
#[test]
fn the_tip_row_hints_the_current_branch_context() {
    let scratch = repo();
    let (app, state) = App::open(scratch.path());
    let state = opened(&app, state);

    assert_eq!(footer(&state, 80, 24), " ⏎ open  c commit  P push  . more");
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

    assert_eq!(footer(&state, 80, 24), " ⏎ open  c commit  y sha  . more");
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

    assert_eq!(
        footer(&state, 80, 24),
        " ⏎ changes  c commit  S stash  . more"
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

    // Stacked below: a 120-cell bar, the six-key tier, `<> resize` among
    // them (m5-03 review, Major 4 — the divider hint the old list dropped).
    assert_eq!(
        hint_bar(&state, 120, 40).as_deref(),
        Some(" space stage  d discard  c commit  <> resize  ⌃A all  e edit  . more")
    );
    // Beside the graph: the graph column is ~93 cells, so the three-key
    // tier — the three that write, and `.` for the rest.
    assert_eq!(
        hint_bar(&state, 160, 48).as_deref(),
        Some(" space stage  d discard  c commit  . more")
    );
    // Wider, still side by side: six again.
    assert_eq!(
        hint_bar(&state, 200, 60).as_deref(),
        Some(" space stage  d discard  c commit  <> resize  ⌃A all  e edit  . more")
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
            let drawn = frame(state, columns, rows);
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
                 # row, unless the pane is stacked under it. Three keys plus `. more` under\n\
                 # 100 columns, six plus `. more` from 100, the context's own keys and then\n\
                 # the screen's from 160 (ADR-0037 §4) — never all hidden, never a key that\n\
                 # does nothing, and `.` leads to the rest.\n"
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
