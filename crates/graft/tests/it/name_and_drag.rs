//! `b`, `t` and the stash overlay's `b` over real `git`, and the drag that
//! ends in a drop menu (m5-history-and-ghost-preview/10).
//!
//! `update::name_prompt`'s and `update::drag`'s own tests settle what a key
//! and a button *mean* over scripted state. None of that is evidence that a
//! branch exists, or that a real button press over a real frame lands on the
//! badge that is drawn there. These press the keys and the button, then read
//! the repository back with `for-each-ref`, `symbolic-ref` and `git stash
//! list`, and check that `u` puts each of them back.
#![allow(
    clippy::expect_used,
    reason = "a test fixture that cannot be built has nothing to test; the panic names why"
)]

use std::path::Path;

use graft::session::Clock;
use graft::update::AppState;
use graft_core::event::{KeyCode, MouseButton, MouseKind};

use crate::app_harness::{App, git, git_as_graft};
use crate::context_menu::{on_mains_row, settled};
use crate::preview::{FORK_NOW, forked, frame_at, opened, rows_drawn};
use graft_core::on_disk::CURRENT_DIR;

// ---------------------------------------------------------------------------
// The name prompt, against git
// ---------------------------------------------------------------------------

/// Every ref in the repository, read out of git rather than out of
/// `AppState`.
fn refs(repo: &Path) -> Vec<String> {
    git(repo, &["for-each-ref", "--format=%(refname)"])
        .lines()
        .map(str::to_owned)
        .collect()
}

fn head_ref(repo: &Path) -> String {
    git(repo, &["symbolic-ref", "HEAD"]).trim().to_owned()
}

/// What the undo journal holds, as the app's own reader sees it. A create
/// that journals twice, or not at all, breaks `u` — and neither shows up in
/// `for-each-ref`.
fn journal(repo: &Path) -> Vec<graft_core::journal::JournalEntry> {
    let path = repo.join(format!(".git/{CURRENT_DIR}/undo-journal.jsonl"));
    if !path.exists() {
        return Vec::new();
    }
    graft_ops::journal::Journal::open(&path)
        .expect("the journal the writes just made is readable")
        .entries()
        .to_vec()
}

fn type_in(app: &App, state: AppState, text: &str) -> AppState {
    let mut state = state;
    for character in text.chars() {
        state = app.key(&state, KeyCode::Char(character));
    }
    state
}

/// The commit id the cursor is on, which is where `b` and `t` promise to
/// put the new ref.
fn under_the_cursor(state: &AppState) -> String {
    state
        .history
        .as_ref()
        .and_then(graft::history::History::selected)
        .map(|row| row.commit.id.to_string())
        .expect("a commit row under the cursor")
}

/// `b`, a name, `⇥`, `Enter`: a branch exists at the cursor's commit and
/// `HEAD` is on it — both read out of git. Then `u` deletes it and `U`
/// makes it again.
#[test]
fn b_creates_and_checks_out() {
    let scratch = forked();
    let repo = scratch.path();
    let (app, state) = App::open(repo);
    let state = on_mains_row(&app, state);
    let at = under_the_cursor(&state);

    let before = journal(repo).len();
    let state = app.key(&state, KeyCode::Char('b'));
    assert!(state.name_prompt.is_some(), "`b` opened the name prompt");
    let state = type_in(&app, state, "release");
    // `⇥` asks for the checkout as well.
    let state = app.key(&state, KeyCode::Tab);
    let state = app.key(&state, KeyCode::Enter);
    let state = app.settle_write(state, "the branch create");

    assert!(
        refs(repo).contains(&"refs/heads/release".to_owned()),
        "git lists the branch: {:?}",
        refs(repo)
    );
    assert_eq!(head_ref(repo), "refs/heads/release", "and `HEAD` is on it");
    let entries = journal(repo);
    assert_eq!(entries.len(), before + 1, "exactly one journal entry");
    assert!(
        entries
            .last()
            .is_some_and(|entry| entry.description.contains("release")),
        "and it names the branch: {:?}",
        entries.last()
    );
    assert_eq!(
        git(repo, &["rev-parse", "refs/heads/release"]).trim(),
        at,
        "at the row the cursor was on"
    );

    let state = app.key(&state, KeyCode::Char('u'));
    let state = app.settle_write(state, "undoing the branch create");
    assert!(
        !refs(repo).contains(&"refs/heads/release".to_owned()),
        "`u` deleted it: {:?}",
        refs(repo)
    );

    let state = app.key(&state, KeyCode::Char('U'));
    let state = app.settle_write(state, "redoing the branch create");
    assert!(
        refs(repo).contains(&"refs/heads/release".to_owned()),
        "`U` made it again: {:?}",
        refs(repo)
    );
    drop(state);
    app.close();
}

/// `t` makes a **lightweight** tag: a ref straight at the commit, with no
/// tag object behind it. An annotated tag needs a message box, which is
/// M7's.
#[test]
fn t_creates_a_lightweight_tag() {
    let scratch = forked();
    let repo = scratch.path();
    let (app, state) = App::open(repo);
    let state = on_mains_row(&app, state);
    let at = under_the_cursor(&state);

    let before = journal(repo).len();
    let state = app.key(&state, KeyCode::Char('t'));
    assert!(state.name_prompt.is_some(), "`t` opened the name prompt");
    let state = type_in(&app, state, "v2.0.0");
    let state = app.key(&state, KeyCode::Enter);
    let state = app.settle_write(state, "the tag create");

    assert!(
        refs(repo).contains(&"refs/tags/v2.0.0".to_owned()),
        "git lists the tag: {:?}",
        refs(repo)
    );
    assert_eq!(
        git(repo, &["cat-file", "-t", "refs/tags/v2.0.0"]).trim(),
        "commit",
        "lightweight: the ref names the commit, not a tag object"
    );
    assert_eq!(git(repo, &["rev-parse", "refs/tags/v2.0.0"]).trim(), at);
    assert_eq!(
        journal(repo).len(),
        before + 1,
        "exactly one journal entry: {:?}",
        journal(repo)
    );

    let state = app.key(&state, KeyCode::Char('u'));
    let state = app.settle_write(state, "undoing the tag create");
    assert!(
        !refs(repo).contains(&"refs/tags/v2.0.0".to_owned()),
        "`u` deleted it: {:?}",
        refs(repo)
    );

    let state = app.key(&state, KeyCode::Char('U'));
    let state = app.settle_write(state, "redoing the tag create");
    assert!(
        refs(repo).contains(&"refs/tags/v2.0.0".to_owned()),
        "`U` made it again: {:?}",
        refs(repo)
    );
    drop(state);
    app.close();
}

/// The stash overlay's `b`: name the branch, see the picture, confirm, and
/// the entry is a branch. `u` puts the stash back.
/// One commit, one tracked file, and one stash called `wip` waiting on it.
fn a_repo_with_a_stash() -> graft_fixtures::Scratch {
    let scratch = graft_fixtures::copy_into_temp(graft_fixtures::Shape::Trivial);
    let repo = scratch.path();
    std::fs::write(repo.join("sample.txt"), "one\n").expect("the scratch repo is writable");
    git(repo, &["add", "sample.txt"]);
    git_as_graft(repo, &["commit", "-m", "add sample"]);
    std::fs::write(repo.join("sample.txt"), "one\ntwo\n").expect("the scratch repo is writable");
    git(repo, &["stash", "push", "-m", "wip"]);
    scratch
}

/// The stash overlay open, with its list on hand.
fn stash_overlay(repo: &Path) -> (App, AppState) {
    let (app, state) = App::open(repo);
    let state = app.pump(state, "the working-tree status", |state| {
        state.working_tree.is_some()
    });
    let state = app.key(&state, KeyCode::Char('S'));
    let state = app.pump(state, "the stash list", |state| {
        !state.stash.rows().is_empty()
    });
    (app, state)
}

#[test]
fn stash_b_opens_prompt_then_preview_then_creates() {
    let scratch = a_repo_with_a_stash();
    let repo = scratch.path();
    let (app, state) = stash_overlay(repo);

    let before = journal(repo).len();
    let state = app.key(&state, KeyCode::Char('b'));
    assert!(state.name_prompt.is_some(), "`b` opened the name prompt");
    assert!(state.stash.shown, "the overlay is still behind the box");

    let state = type_in(&app, state, "rescued");
    let state = app.key(&state, KeyCode::Enter);
    assert!(state.name_prompt.is_none(), "the box closed");
    assert!(!state.stash.shown, "the overlay stepped out of the way");
    let state = app.pump(state, "the plan for `stash branch`", |state| {
        state
            .preview
            .as_ref()
            .is_some_and(|preview| matches!(preview.plan, graft::update::detail::Loading::Ready(_)))
    });
    assert!(
        git(repo, &["stash", "list"]).contains("wip"),
        "planning is a read: the entry is still on the list"
    );

    // The picture is the confirmation step: `→` off `Cancel` onto `Confirm`,
    // then `Enter`. The write runs only now.
    let state = app.key(&state, KeyCode::Right);
    let state = app.key(&state, KeyCode::Enter);
    let state = app.settle_write(state, "`git stash branch`");
    assert!(
        refs(repo).contains(&"refs/heads/rescued".to_owned()),
        "the branch exists: {:?}",
        refs(repo)
    );
    assert_eq!(head_ref(repo), "refs/heads/rescued");
    assert!(
        git(repo, &["stash", "list"]).trim().is_empty(),
        "and the entry is gone: {:?}",
        git(repo, &["stash", "list"])
    );
    assert_eq!(
        journal(repo).len(),
        before + 1,
        "exactly one journal entry: {:?}",
        journal(repo)
    );

    let state = app.key(&state, KeyCode::Char('u'));
    let state = app.settle_write(state, "undoing `stash branch`");
    assert!(
        git(repo, &["stash", "list"]).contains("wip"),
        "`u` put the stash back: {:?}",
        git(repo, &["stash", "list"])
    );

    let state = app.key(&state, KeyCode::Char('U'));
    let state = app.settle_write(state, "redoing `stash branch`");
    assert!(
        git(repo, &["stash", "list"]).trim().is_empty(),
        "`U` made the branch again and took the entry back: {:?}",
        git(repo, &["stash", "list"])
    );
    drop(state);
    app.close();
}

// ---------------------------------------------------------------------------
// The drag, over the real frame
// ---------------------------------------------------------------------------

/// Presses the left button at every cell of the top few rows until one picks
/// a badge up, and answers with the state, the column and the screen row it
/// was found on.
///
/// **What this proves, and what it does not.** It presses cells until the
/// hit-test says a badge is under one, so what follows is a real gesture
/// through `update` carrying a real branch. It reads no drawn cell, though,
/// so a hit-test measuring geometry the frame never drew would still be
/// found here — at the wrong column — and this would pass. It used to claim
/// otherwise (m5/33, review M1). The agreement between the drawn cells and
/// the measured ones is pinned by
/// `graft::view::tests::a_drag_lands_on_the_badge_the_frame_drew_under_the_pointer`,
/// which finds the badge in the rendered buffer first and then presses
/// exactly where it was drawn.
fn grab_a_badge(app: &App, state: &AppState, columns: u16) -> (AppState, u16, u16) {
    for screen_row in 1..8_u16 {
        for column in 0..columns {
            let pressed = app.mouse(
                state,
                MouseKind::Down(MouseButton::Left),
                column,
                screen_row,
            );
            if pressed.drag.is_some() {
                return (pressed, column, screen_row);
            }
        }
    }
    panic!("no draggable badge in the top rows of a {columns}-column frame");
}

/// Which drawn row carries `main`'s badge.
fn mains_row(state: &AppState) -> usize {
    let refs = state.refs.as_deref().expect("the refs have arrived");
    let window = state
        .history
        .as_ref()
        .and_then(|history| history.window.as_ref())
        .expect("a drawn window");
    window
        .rows
        .iter()
        .position(|row| {
            refs.on(row.commit.id).iter().any(|label| {
                matches!(label, graft::refs::RefLabel::Branch { name, .. } if name == b"main")
            })
        })
        .unwrap_or_else(|| panic!("no row carries main: {:?}", rows_drawn(state)))
}

/// The app at `columns`x`rows`, settled, with `feature`'s badge picked up,
/// dragged onto `main`'s row and released there — three real mouse events
/// through `update`.
fn dropped_onto_main(app: &App, state: AppState, columns: u16, rows: u16) -> AppState {
    let state = app.resize(&opened(app, state), columns, rows);
    let state = settled(app, state);

    let (state, column, from_screen_row) = grab_a_badge(app, &state, columns);
    let from_row = state
        .drag
        .as_ref()
        .map(|drag| drag.from_row)
        .expect("a badge is in the air");
    let onto = mains_row(&state);
    assert_ne!(onto, from_row, "the badge came off a different row");
    // The drawn rows are consecutive, so the target's screen row is the
    // press's, moved by the distance between the two row indices.
    let onto_screen_row = from_screen_row + u16::try_from(onto - from_row).expect("a small window");

    let state = app.mouse(
        &state,
        MouseKind::Drag(MouseButton::Left),
        column,
        onto_screen_row,
    );
    assert_eq!(
        state.drag.as_ref().and_then(|drag| drag.over),
        Some(onto),
        "the badge is over main's row"
    );
    let state = app.mouse(
        &state,
        MouseKind::Up(MouseButton::Left),
        column,
        onto_screen_row,
    );
    assert!(
        state.context_menu.is_some(),
        "the release opened the drop menu: {:?}",
        state.status
    );
    state
}

/// The same gesture stopped **one event earlier**: the badge is over
/// `main`'s row and the button is still down.
///
/// This is the state the user described as showing nothing
/// (m5-history-and-ghost-preview/35B): *"when I'm dragging there is no
/// indication that I have held something, until I release and a popup shows
/// up."* Every drag test before this one either asserted `AppState` or
/// captured the frame *after* release.
fn holding_over_main(app: &App, state: AppState, columns: u16, rows: u16) -> AppState {
    let state = app.resize(&opened(app, state), columns, rows);
    let state = settled(app, state);

    let (state, column, from_screen_row) = grab_a_badge(app, &state, columns);
    let from_row = state
        .drag
        .as_ref()
        .map(|drag| drag.from_row)
        .expect("a badge is in the air");
    let onto = mains_row(&state);
    let onto_screen_row = from_screen_row + u16::try_from(onto - from_row).expect("a small window");
    let state = app.mouse(
        &state,
        MouseKind::Drag(MouseButton::Left),
        column,
        onto_screen_row,
    );
    assert_eq!(
        state.drag.as_ref().and_then(|drag| drag.over),
        Some(onto),
        "the badge is over main's row, button still down"
    );
    state
}

/// The gesture end to end: a menu with a title and a disabled row, and
/// `Enter` on an enabled verb landing in a Ghost Preview rather than a
/// write.
#[test]
fn a_drag_onto_a_row_ends_in_a_preview() {
    let scratch = forked();
    let repo = scratch.path();
    let before = git(repo, &["for-each-ref", "--format=%(refname) %(objectname)"]);
    let (app, state) = App::open(repo);
    let state = dropped_onto_main(&app, state, 80, 24);

    let open = state.context_menu.as_ref().expect("the drop menu is open");
    assert!(
        open.menu.title.is_some(),
        "the drop menu says what went where"
    );
    assert!(
        open.menu.items.iter().any(|item| item.disabled.is_some()),
        "a verb that cannot run is on the list with its reason: {:?}",
        open.menu
            .items
            .iter()
            .map(|item| item.label.as_str().to_owned())
            .collect::<Vec<_>>()
    );
    let steps = open
        .menu
        .items
        .iter()
        .position(|item| item.disabled.is_none())
        .expect("at least one verb can run");

    let mut state = state;
    for _ in 0..steps {
        state = app.key(&state, KeyCode::Char('j'));
    }
    let state = app.key(&state, KeyCode::Enter);
    assert!(state.context_menu.is_none(), "the menu closed");
    assert!(state.preview.is_some(), "a picture opened instead");
    let state = app.pump(state, "the drop verb's plan", |state| {
        state
            .preview
            .as_ref()
            .is_some_and(|preview| matches!(preview.plan, graft::update::detail::Loading::Ready(_)))
    });
    assert_eq!(
        git(repo, &["for-each-ref", "--format=%(refname) %(objectname)"]),
        before,
        "planning is a read: no ref moved"
    );
    drop(state);
    app.close();
}

// ---------------------------------------------------------------------------
// The captures
// ---------------------------------------------------------------------------

/// The drop menu at both sizes, asserted against what is committed under
/// `docs/shots/`.
///
/// The **user** judges these by eye (CLAUDE.md rule 6); this only keeps what
/// he judged from changing behind his back. Regenerate on purpose with
/// `MWR_UPDATE_SHOTS=1`, and read the diff — that run is red either way,
/// see `graft_fixtures::capture`.
#[test]
fn capture_the_drop_menu_for_docs_shots() {
    let mut drifted = Vec::new();
    for (columns, rows) in [(80_u16, 24_u16), (160, 48)] {
        let scratch = forked();
        let (app, state) = App::open(scratch.path());
        let state = dropped_onto_main(&app, state, columns, rows);
        let out = capture(&state, columns, rows, &drop_menu_caption(columns, rows));
        app.close();
        if let Err(why) = graft_fixtures::check(&shot(columns, rows, "drop-menu"), &out) {
            drifted.push(why);
        }
    }
    assert!(drifted.is_empty(), "{}", drifted.join("\n\n"));
}

/// **Mid-drag**, before the button comes up: the one frame nothing captured
/// until m5-history-and-ghost-preview/35.
#[test]
fn capture_a_drag_in_flight_for_docs_shots() {
    let mut drifted = Vec::new();
    for (columns, rows) in [(80_u16, 24_u16), (160, 48)] {
        let scratch = forked();
        let (app, state) = App::open(scratch.path());
        let state = holding_over_main(&app, state, columns, rows);
        let out = capture(&state, columns, rows, &dragging_caption(columns, rows));
        app.close();
        if let Err(why) = graft_fixtures::check(&shot(columns, rows, "dragging"), &out) {
            drifted.push(why);
        }
    }
    assert!(drifted.is_empty(), "{}", drifted.join("\n\n"));
}

fn dragging_caption(columns: u16, rows: u16) -> String {
    format!(
        "# m5-ui-dragging-{columns}x{rows}.txt: drawn by `view::view` from a real\n\
         # AppState over a real repository (`main` three commits, `feature` forked off\n\
         # the first with two of its own, `feature` checked out), opened over the real\n\
         # job pool, `feature`'s badge pressed and dragged onto `main`'s row — the\n\
         # button is **still down**, two real mouse events through `update`, no release\n\
         # (crates/graft/tests/it/name_and_drag.rs, capture_a_drag_in_flight_for_docs_shots).\n\
         # A golden snapshot: that test fails if the frame drifts. Regenerate on\n\
         # purpose with MWR_UPDATE_SHOTS=1. The terminal is ratatui's TestBackend, so\n\
         # this is a character grid with no colour attributes in it — CLAUDE.md rule 5,\n\
         # colour ships unverified, and the raised background on two of these rows is\n\
         # **not** in this file. The three marks below are shapes for exactly that\n\
         # reason.\n\
         #\n\
         # What right looks like:\n\
         #  - `main`'s row carries `→` in the marker column: this is where the badge\n\
         #    would land (m8-polish-and-ship/07: never `▸`, the cursor's mark).\n\
         #  - The row `feature`'s badge came off is faded — the badge is in the air,\n\
         #    not on that row. Here that is also the row the cursor is on (the badge\n\
         #    was picked up from under it), so the fade replaces the cursor's own\n\
         #    gutter `▸`: one row, one state. Drag from a row the cursor is not on\n\
         #    and both marks are drawn, gutter `▸` and slot `→`, on different rows.\n\
         #  - The footer reads `release drop on a row  esc cancel`.\n\
         #  - Nothing else has moved: no menu, no box, the graph unchanged.\n"
    )
}

/// The name prompt with its option line and a refusal under the field: `b`
/// on `main`'s row, with `main` typed into it — a name that is taken.
#[test]
fn capture_the_name_prompt_for_docs_shots() {
    let scratch = forked();
    let (app, state) = App::open(scratch.path());
    let state = settled(&app, on_mains_row(&app, state));
    let state = app.key(&state, KeyCode::Char('b'));
    let state = type_in(&app, state, "main");
    assert!(
        state.name_prompt.is_some(),
        "the prompt is open for the picture"
    );
    let out = capture(&state, 80, 24, &name_prompt_caption());
    app.close();

    let checked = graft_fixtures::check(&shot(80, 24, "name-prompt"), &out);
    assert!(checked.is_ok(), "{}", checked.expect_err("it drifted"));
}

/// The stash's `b` → name → `Enter` preview: its footer names `y confirm`
/// only — `b` again does not confirm (m6-conflicts/11). Regenerate on
/// purpose with `MWR_UPDATE_SHOTS=1`.
#[test]
fn capture_fix_preview_verb_name_prompt_for_docs_shots() {
    // Dated, so the stash's id and age are the same on every run.
    let scratch = graft_fixtures::copy_into_temp(graft_fixtures::Shape::Trivial);
    let repo = scratch.path();
    let when = format!("@{} +0000", FORK_NOW - 120);
    std::fs::write(repo.join("sample.txt"), "one\n").expect("the scratch repo is writable");
    git(repo, &["add", "sample.txt"]);
    graft_fixtures::git::run_at(repo, &["commit", "-m", "add sample"], &when)
        .unwrap_or_else(|why| panic!("committing: {why}"));
    std::fs::write(repo.join("sample.txt"), "one\ntwo\n").expect("the scratch repo is writable");
    graft_fixtures::git::run_at(repo, &["stash", "push", "-m", "wip"], &when)
        .unwrap_or_else(|why| panic!("stashing: {why}"));
    let (app, state) = stash_overlay(repo);
    let state = app.key(&state, KeyCode::Char('b'));
    let state = type_in(&app, state, "rescued");
    let state = app.key(&state, KeyCode::Enter);
    let state = app.pump(state, "the plan for `stash branch`", |state| {
        state
            .preview
            .as_ref()
            .is_some_and(|preview| matches!(preview.plan, graft::update::detail::Loading::Ready(_)))
    });
    let caption = "# fix-preview-verb-name-prompt-120x40.txt: 120x40, drawn by `view::view` from a\n\
         # real AppState over a real repository with one stash entry; the stash overlay\n\
         # opened (`S`), `b` pressed, `rescued` typed, `Enter`, the real plan landed\n\
         # (crates/graft/tests/it/name_and_drag.rs,\n\
         # capture_fix_preview_verb_name_prompt_for_docs_shots). TestBackend character\n\
         # grid: colour ships unverified (CLAUDE.md rule 5).\n\
         #\n\
         # What right looks like:\n\
         #  - The box's bottom border reads `y confirm  ←/→ choose  ⏎ act  esc cancel`\n\
         #    (no `b` in it: pressing `b` again does not confirm).\n\
         #  - The buttons row shows `▸[ Cancel ]`.\n";
    let out = capture(&state, 120, 40, caption);
    app.close();
    assert!(
        out.contains("╰ y confirm  ←/→ choose  ⏎ act  esc cancel ─"),
        "{out}"
    );
    assert!(out.contains("▸[ Cancel ]"), "{out}");
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/shots/m6/fix-preview-verb-name-prompt-120x40.txt");
    let checked = graft_fixtures::check(&path, &out);
    assert!(checked.is_ok(), "{}", checked.expect_err("it drifted"));
}

fn shot(columns: u16, rows: u16, what: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/shots")
        .join(format!("m5-ui-{what}-{columns}x{rows}.txt"))
}

/// A caption and the frame under it — the shape every capture in
/// `docs/shots/` has (ADR-0037).
fn capture(state: &AppState, columns: u16, rows: u16, caption: &str) -> String {
    let mut out = caption.to_owned();
    out.push('\n');
    for line in frame_at(state, columns, rows, Some(Clock::Fixed(FORK_NOW))) {
        out.push_str(line.trim_end());
        out.push('\n');
    }
    out
}

/// Where the frame came from, and what the user is checking in it.
fn drop_menu_caption(columns: u16, rows: u16) -> String {
    format!(
        "# m5-ui-drop-menu-{columns}x{rows}.txt: drawn by `view::view` from a real\n\
         # AppState over a real repository (`main` three commits, `feature` forked off\n\
         # the first with two of its own, `feature` checked out), opened over the real\n\
         # job pool, `feature`'s badge pressed, dragged onto `main`'s row and released\n\
         # there — three real mouse events through `update`\n\
         # (crates/graft/tests/it/name_and_drag.rs, capture_the_drop_menu_for_docs_shots).\n\
         # A golden snapshot: that test fails if the frame drifts. Regenerate on\n\
         # purpose with MWR_UPDATE_SHOTS=1. The terminal is ratatui's TestBackend, so\n\
         # this is a character grid with no colour attributes in it — CLAUDE.md rule 5,\n\
         # colour ships unverified.\n\
         #\n\
         # What right looks like:\n\
         #  - A title row above the verbs reads `feature → main`, so the menu says what\n\
         #    went where without the user having to remember the gesture.\n\
         #  - Every verb names both branches, never `this` or `that`.\n\
         #  - A verb that cannot run is dim and carries its reason after ` — `.\n\
         #  - `Reset` names its mode after a comma (`, mixed` / `, hard`), never in\n\
         #    parentheses: a trailing `(k)` is reserved for a live key.\n\
         #  - The menu sits inside the graph pane rather than running off it, and\n\
         #    covers whole columns of the graph (ADR-0040) — no half-covered column\n\
         #    shows beside it.\n\
         #  - The menu opens directly under `main`'s row — the row the badge landed\n\
         #    on, which is still readable above it.\n\
         #"
    )
}

fn name_prompt_caption() -> String {
    "# m5-ui-name-prompt-80x24.txt: drawn by `view::view` from a real AppState over\n\
     # a real repository (`main` three commits, `feature` forked off the first with\n\
     # two of its own), opened over the real job pool, cursor moved to `main`'s row,\n\
     # `b` pressed and `main` typed into the field\n\
     # (crates/graft/tests/it/name_and_drag.rs, capture_the_name_prompt_for_docs_shots).\n\
     # A golden snapshot: that test fails if the frame drifts. Regenerate on purpose\n\
     # with MWR_UPDATE_SHOTS=1. The terminal is ratatui's TestBackend, so this is a\n\
     # character grid with no colour attributes in it — CLAUDE.md rule 5, colour\n\
     # ships unverified.\n\
     #\n\
     # What right looks like:\n\
     #  - The top border says what is being named and where: `New branch at <sha7>`.\n\
     #  - The typed name is on the field row, with the cursor after it.\n\
     #  - The option line offers the checkout, and says `⇥` is what flips it.\n\
     #  - The refusal under the field reads `exists` — `main` is taken. A name that\n\
     #    is fine so far shows no line there at all: an empty box never opens with a\n\
     #    warning already on it.\n\
     #  - The bottom border carries the keys that end it.\n\
     #  - The box covers whole columns of the graph (ADR-0040); no half-covered\n\
     #    column shows beside it.\n\
     #"
    .to_owned()
}
