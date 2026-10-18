//! The branch picker over scripted state, through `update` itself so the
//! key routing is what is tested: `C` opens it and asks for the list; the
//! list lands by id; every letter is typed; `⏎` goes through the switch
//! rule; `Tab` seeks; `Esc` closes and cancels. What it does to a real
//! repository is walked in `tests/it/m7_walk/picker.rs`.

use std::path::PathBuf;
use std::sync::Arc;

use mergewright_core::event::{Event, KeyCode, KeyPress, Modifiers};
use mergewright_core::job::JobId;
use mergewright_core::oid::Oid;
use mergewright_core::ref_name::RefName;
use mergewright_git::repo::HeadState;
use mergewright_git::status::{Change, StatusEntry, WorkingTreeStatus};
use mergewright_ops::operation::{CheckoutTarget, OnEdits, Operation, Tip};
use mergewright_ops::switch_list::{Relation, SwitchKind, SwitchList, SwitchRow};

use super::*;
use crate::update::events::{AppEvent, Product};
use crate::update::preview::PreviewSubject;
use crate::update::update;

fn oid(byte: u8) -> Oid {
    Oid::from_bytes(&[byte; Oid::SHA1_BYTES]).expect("20 bytes is an oid")
}

fn name(full: &str) -> RefName {
    RefName::new(full.as_bytes().to_vec()).expect("a valid ref name")
}

fn press(state: &AppState, code: KeyCode) -> (AppState, Vec<Command>) {
    update(
        state,
        AppEvent::Core(Event::Key(KeyPress {
            code,
            modifiers: Modifiers::NONE,
        })),
    )
}

fn row(full: &str, kind: SwitchKind, tip: u8, when: i64) -> SwitchRow {
    SwitchRow {
        name: name(full),
        kind,
        tip: oid(tip),
        subject: format!("subject of {full}"),
        author: "Sara Lee".to_owned(),
        when,
        relation: Relation::None,
        current: false,
    }
}

/// `feature` before; `login-page` and `log-tidy` local; `origin/sara/login`
/// remote-only; `main` here. Tips: `feature` 0x03 (row 0 of the graph),
/// `main` 0x02 (row 1), the rest outside the three loaded rows.
fn list() -> SwitchList {
    SwitchList {
        previous: Some(row("refs/heads/feature", SwitchKind::Local, 3, 50)),
        rows: vec![
            SwitchRow {
                relation: Relation::Tracks(name("refs/remotes/origin/login-page")),
                ..row("refs/heads/login-page", SwitchKind::Local, 9, 40)
            },
            row("refs/heads/catalog", SwitchKind::Local, 8, 30),
            row(
                "refs/remotes/origin/sara/login",
                SwitchKind::RemoteOnly {
                    remote: "origin".to_owned(),
                    local: "sara/login".to_owned(),
                },
                7,
                20,
            ),
            SwitchRow {
                current: true,
                ..row("refs/heads/main", SwitchKind::Local, 2, 10)
            },
        ],
    }
}

/// Three commits, `main` (0x02) checked out, `feature` at 0x03; clean; the
/// cursor on row 2 (0x01).
fn fixture() -> AppState {
    let refs = crate::refs::badges(
        &[
            (name("refs/heads/main"), oid(2)),
            (name("refs/heads/feature"), oid(3)),
        ],
        &HeadState::Branch(name("refs/heads/main")),
        &std::collections::HashMap::new(),
        &[],
        |_, _| {
            Ok(mergewright_git::history::AheadBehind {
                ahead: 0,
                behind: 0,
            })
        },
    )
    .expect("no ahead/behind walk can fail here");
    AppState {
        history: Some(crate::view::tests::history_of(3, 2)),
        refs: Some(Arc::new(refs)),
        working_tree: Some(Arc::new(WorkingTreeStatus { entries: vec![] })),
        size: crate::update::Size {
            columns: 120,
            rows: 40,
        },
        ..AppState::default()
    }
}

fn with_edits(mut state: AppState) -> AppState {
    state.working_tree = Some(Arc::new(WorkingTreeStatus {
        entries: vec![StatusEntry {
            path: PathBuf::from("a.rs"),
            original_path: None,
            staged: None,
            unstaged: Some(Change::Modified),
            untracked: false,
            unmerged: false,
            unmerged_how: None,
            submodule: None,
        }],
    }));
    state
}

/// `C`, then the list lands.
fn opened(state: &AppState) -> AppState {
    let (state, commands) = press(state, KeyCode::Char('C'));
    let id = read_id(&commands);
    let (state, _) = update(
        &state,
        AppEvent::Produced(id, Product::SwitchList(Ok(Arc::new(list())))),
    );
    state
}

fn read_id(commands: &[Command]) -> JobId {
    commands
        .iter()
        .find_map(|command| match command {
            Command::ReadSwitchList(id) => Some(*id),
            _ => None,
        })
        .unwrap_or_else(|| panic!("no read asked for: {commands:?}"))
}

fn typed(state: &AppState, text: &str) -> AppState {
    text.chars().fold(state.clone(), |state, character| {
        press(&state, KeyCode::Char(character)).0
    })
}

fn picker(state: &AppState) -> &PickerState {
    state.picker.as_ref().expect("the picker is up")
}

fn names(state: &AppState) -> Vec<String> {
    picker(state)
        .rows()
        .iter()
        .map(|row| String::from_utf8_lossy(row.name.shorthand()).into_owned())
        .collect()
}

fn operated(commands: &[Command]) -> Option<&Operation> {
    commands.iter().find_map(|command| match command {
        Command::Operate(_, op, ..) => Some(op.as_ref()),
        _ => None,
    })
}

fn checkout_to(op: &Operation) -> &CheckoutTarget {
    match op {
        Operation::Checkout(params) => &params.to,
        other => panic!("not a checkout: {other:?}"),
    }
}

/// Down to the row named `short`.
fn on_row(state: &AppState, short: &str) -> AppState {
    let at = names(state)
        .iter()
        .position(|name| name == short)
        .unwrap_or_else(|| panic!("no row {short}: {:?}", names(state)));
    (0..at).fold(state.clone(), |state, _| press(&state, KeyCode::Down).0)
}

#[test]
fn c_opens_the_picker() {
    let (state, commands) = press(&fixture(), KeyCode::Char('C'));
    let open = picker(&state);
    assert!(open.query.is_empty());
    assert_eq!(open.list, Loading::Pending(read_id(&commands)));
    assert!(state.text_field_open(), "a text field: globals stand aside");
    assert!(state.modal_owns_keys());
    let state = opened(&fixture());
    assert_eq!(
        names(&state),
        [
            "feature",
            "login-page",
            "catalog",
            "origin/sara/login",
            "main"
        ],
        "before first, here last"
    );
}

/// m8/09e: was `c_needs_the_graph_focused_and_nothing_over_it` — a pane's
/// focus no longer stops `C` (`update::keys::tests::c_opens_the_picker_from_the_panes`);
/// an overlay still does.
#[test]
fn c_needs_nothing_over_the_graph() {
    let mut state = fixture();
    state.journal.shown = true;
    let (state, _) = press(&state, KeyCode::Char('C'));
    assert!(state.picker.is_none(), "the journal is up");
}

/// The commit editor's `C` stays the editor's (`commit_editor.rs`): the
/// picker is routed ahead of it, and must let it through.
#[test]
fn c_in_the_commit_editor_is_typed_there() {
    let mut state = fixture();
    state.commit_editor = Some(Box::new(crate::update::commit_editor::CommitEditor::new()));
    let (state, commands) = press(&state, KeyCode::Char('C'));
    assert!(state.picker.is_none());
    assert!(
        !commands
            .iter()
            .any(|c| matches!(c, Command::ReadSwitchList(_)))
    );
    assert_eq!(
        state
            .commit_editor
            .as_ref()
            .map(|editor| editor.subject.text().to_owned()),
        Some("C".to_owned())
    );
}

#[test]
fn a_stale_list_is_dropped() {
    let (state, commands) = press(&fixture(), KeyCode::Char('C'));
    let id = read_id(&commands);
    let older = JobId::new(999_999);
    let (state, _) = update(
        &state,
        AppEvent::Produced(older, Product::SwitchList(Ok(Arc::new(list())))),
    );
    assert_eq!(picker(&state).list, Loading::Pending(id));
}

#[test]
fn typing_filters_segment_prefix_first() {
    let state = typed(&opened(&fixture()), "log");
    assert_eq!(
        names(&state),
        ["login-page", "origin/sara/login", "catalog"],
        "`log` starts a segment of the first two; only a substring of catalog"
    );
    assert_eq!(
        picker(&state).selected,
        0,
        "typing puts the cursor on the best"
    );
    let state = press(&state, KeyCode::Backspace).0;
    assert_eq!(picker(&state).query.text(), "lo");
}

#[test]
fn the_picker_owns_every_letter() {
    let state = typed(&opened(&fixture()), "jkqCu");
    assert_eq!(picker(&state).query.text(), "jkqCu");
    assert!(state.quit.is_none(), "q typed, not quit");
    assert!(state.operating.is_none(), "u typed, not undo");
}

#[test]
fn ctrl_c_still_quits() {
    let state = opened(&fixture());
    let (state, _) = update(
        &state,
        AppEvent::Core(Event::Key(KeyPress {
            code: KeyCode::Char('c'),
            modifiers: Modifiers::CONTROL,
        })),
    );
    assert!(state.quit.is_some());
}

#[test]
fn enter_on_a_clean_tree_switches_at_once() {
    let state = on_row(&opened(&fixture()), "login-page");
    let (state, commands) = press(&state, KeyCode::Enter);
    let op = operated(&commands).expect("launched at once");
    assert_eq!(
        checkout_to(op),
        &CheckoutTarget::Branch("refs/heads/login-page".to_owned())
    );
    assert!(state.preview.is_none());
    assert!(state.picker.is_none(), "the picker closed");
}

#[test]
fn enter_on_before_goes_back() {
    let (state, commands) = press(&opened(&fixture()), KeyCode::Enter);
    assert_eq!(
        checkout_to(operated(&commands).expect("launched")),
        &CheckoutTarget::Branch("refs/heads/feature".to_owned()),
        "`C ⏎` is the previous branch"
    );
    assert!(state.picker.is_none());
}

#[test]
fn a_remote_only_branch_creates_its_tracking_branch() {
    let state = on_row(&opened(&fixture()), "origin/sara/login");
    let (_, commands) = press(&state, KeyCode::Enter);
    assert_eq!(
        checkout_to(operated(&commands).expect("launched")),
        &CheckoutTarget::Track {
            remote: Tip {
                name: "refs/remotes/origin/sara/login".to_owned(),
                oid: oid(7),
            },
            local: "sara/login".to_owned(),
        }
    );
}

#[test]
fn enter_with_edits_opens_the_set_aside_preview() {
    let state = on_row(&opened(&with_edits(fixture())), "login-page");
    let (state, commands) = press(&state, KeyCode::Enter);
    assert!(operated(&commands).is_none(), "nothing written yet");
    let preview = state.preview.as_ref().expect("the preview opened");
    match &preview.subject {
        PreviewSubject::Operation { op, opened_by } => {
            assert_eq!(*opened_by, "C");
            assert!(matches!(
                op.as_ref(),
                Operation::Checkout(params) if params.on_edits == OnEdits::SetAside
            ));
        }
        other @ PreviewSubject::RevertTo(_) => panic!("{other:?}"),
    }
    assert!(picker(&state).under_preview, "hidden, not closed");
    assert!(!state.text_field_open(), "the preview has the keys");
}

#[test]
fn esc_on_that_preview_returns_to_the_picker_with_the_query() {
    let state = typed(&opened(&with_edits(fixture())), "cat");
    let (state, _) = press(&state, KeyCode::Enter);
    assert!(state.preview.is_some());
    let (state, _) = press(&state, KeyCode::Escape);
    assert!(state.preview.is_none());
    let open = picker(&state);
    assert!(!open.under_preview, "shown again");
    assert_eq!(open.query.text(), "cat");
    assert_eq!(names(&state), ["catalog"]);
}

#[test]
fn y_on_that_preview_closes_the_picker() {
    let state = on_row(&opened(&with_edits(fixture())), "login-page");
    let (mut state, _) = press(&state, KeyCode::Enter);
    let preview = state.preview.as_ref().expect("the preview opened");
    let op = preview.subject.operation().expect("an operation").clone();
    let job = preview.job;
    let plan = mergewright_ops::preview::PreviewPlan {
        files: None,
        description: op.describe(),
        commands: vec!["git switch login-page".to_owned()],
        ghosts: Vec::new(),
        ref_moves: Vec::new(),
        head: None,
        worktree: mergewright_ops::preview::WorktreeEffect::Updated {
            paths: vec![PathBuf::from("a.rs")],
            more: false,
        },
        conflicts: mergewright_ops::preview::ConflictForecast::Clean,
        undo: mergewright_ops::preview::UndoNote::Undoable,
        against: Vec::new(),
        set_aside: None,
        put_back: None,
        already_in: None,
        published: Vec::new(),
        left_behind: Vec::new(),
        left_behind_more: 0,
        forks: Vec::new(),
        forks_more: 0,
        ..Default::default()
    };
    crate::update::preview::preview_produced(
        &mut state,
        job,
        Ok(Arc::new(crate::update::preview::Planned {
            plan,
            anchors: Vec::new(),
            runs: None,
        })),
    );
    let (state, commands) = press(&state, KeyCode::Char('y'));
    assert!(operated(&commands).is_some(), "y wrote: {:?}", state.status);
    assert!(state.preview.is_none());
    assert!(state.picker.is_none(), "closed, not shown again");
}

#[test]
fn enter_while_a_write_runs_refuses_in_the_box() {
    let mut state = opened(&fixture());
    // A write in flight, as `operate::claim_write` records one.
    state.operating = Some(JobId::new(900));
    let before = state.status.clone();
    let (state, commands) = press(&state, KeyCode::Enter);
    assert!(operated(&commands).is_none());
    let note = picker(&state)
        .note
        .clone()
        .expect("the words are in the box");
    assert_eq!(note, "wait for the write to finish");
    assert_eq!(state.status, before, "not on the status line");
    // Tab still works.
    let (state, _) = press(&state, KeyCode::Tab);
    assert!(state.picker.is_none());
}

#[test]
fn tab_puts_the_cursor_on_the_tip_and_writes_nothing() {
    let (state, commands) = press(&opened(&fixture()), KeyCode::Tab);
    assert!(state.picker.is_none());
    assert!(operated(&commands).is_none());
    assert_eq!(
        state
            .history
            .as_ref()
            .and_then(crate::history::History::selected)
            .map(|row| row.commit.id),
        Some(oid(3)),
        "feature's tip, row 0"
    );
}

#[test]
fn esc_closes_and_cancels_the_read() {
    let (state, commands) = press(&fixture(), KeyCode::Char('C'));
    let id = read_id(&commands);
    let (state, commands) = press(&state, KeyCode::Escape);
    assert!(state.picker.is_none());
    assert_eq!(commands, vec![Command::CancelJob(id)]);
    let state = opened(&fixture());
    let (state, commands) = press(&state, KeyCode::Escape);
    assert!(state.picker.is_none());
    assert!(commands.is_empty(), "nothing in flight to cancel");
}

#[test]
fn a_failed_read_says_so_in_the_box() {
    let (state, commands) = press(&fixture(), KeyCode::Char('C'));
    let (state, _) = update(
        &state,
        AppEvent::Produced(
            read_id(&commands),
            Product::SwitchList(Err("fatal: bad\nthings".to_owned())),
        ),
    );
    assert_eq!(
        picker(&state).list,
        Loading::Failed("fatal: bad\nthings".to_owned())
    );
    let view = crate::view::picker::picker_view(&state).expect("drawn");
    assert_eq!(
        view.body,
        mergewright_ui::picker::PickerBody::Failed(mergewright_term::sanitize_line("fatal: bad things"))
    );
    let (state, _) = press(&state, KeyCode::Enter);
    assert!(state.picker.is_some(), "⏎ on nothing does nothing");
}

// --- The mouse (ticket 10, Decided: click selects, double-click switches) ---

fn click(state: &AppState, column: u16, row: u16) -> (AppState, Vec<Command>) {
    use mergewright_core::event::{MouseButton, MouseInput, MouseKind};
    update(
        state,
        AppEvent::Core(Event::Mouse(MouseInput {
            kind: MouseKind::Down(MouseButton::Left),
            column,
            row,
            modifiers: Modifiers::NONE,
        })),
    )
}

/// The screen row the drawn frame shows `name` on, and a column inside the
/// box — read off the real frame, so the click and the drawing are checked
/// against each other, not against a number this test assumed.
fn drawn_at(state: &AppState, name: &str) -> (u16, u16) {
    let frame = crate::view::tests::draw(state, state.size.columns, state.size.rows);
    let row = frame
        .lines()
        .position(|line| line.contains(&format!(" {name} ")))
        .unwrap_or_else(|| panic!("`{name}` is drawn in the box\n{frame}"));
    (
        state.size.columns / 2,
        u16::try_from(row).expect("a screen row"),
    )
}

fn at(state: &AppState, seconds: i64) -> AppState {
    let mut state = state.clone();
    state.harness.clock = crate::session::Clock::Fixed(seconds);
    state
}

#[test]
fn a_click_on_a_row_selects_it_and_writes_nothing() {
    let state = at(&opened(&fixture()), 1_000);
    let (column, row) = drawn_at(&state, "catalog");
    let (state, commands) = click(&state, column, row);
    let picked = picker(&state).selected_row().expect("a row");
    assert_eq!(picked.name.as_bytes(), b"refs/heads/catalog");
    assert!(operated(&commands).is_none(), "one click writes nothing");
    assert!(state.picker.is_some(), "the picker stays up");
}

#[test]
fn a_double_click_on_a_row_switches_to_it() {
    let state = at(&opened(&fixture()), 1_000);
    let (column, row) = drawn_at(&state, "login-page");
    let (state, _) = click(&state, column, row);
    let (state, commands) = click(&state, column, row);
    assert_eq!(
        checkout_to(operated(&commands).expect("launched at once")),
        &CheckoutTarget::Branch("refs/heads/login-page".to_owned())
    );
    assert!(state.picker.is_none(), "the picker closed, as on ⏎");
}

#[test]
fn two_slow_clicks_are_two_selections() {
    let state = at(&opened(&fixture()), 1_000);
    let (column, row) = drawn_at(&state, "login-page");
    let (state, _) = click(&state, column, row);
    let (state, commands) = click(&at(&state, 1_001), column, row);
    assert!(
        operated(&commands).is_none(),
        "a second apart is not a double"
    );
    assert!(state.picker.is_some());
}

#[test]
fn a_click_outside_the_box_reaches_nothing_behind_it() {
    let state = at(&opened(&fixture()), 1_000);
    let cursor = state.history.as_ref().expect("a walk").cursor;
    // The first graph row, above the box.
    let (state, commands) = click(&state, 10, 1);
    assert_eq!(state.history.as_ref().expect("a walk").cursor, cursor);
    assert!(state.picker.is_some(), "the picker is modal");
    assert!(operated(&commands).is_none());
}

// ── At a live stop (review m7-10-to-07c Major 1) ───────────────────────

/// The fixture at a rewrite's `edit` stop, the graph with the keyboard —
/// where `C` opens the picker while an operation is live.
fn at_a_live_stop() -> AppState {
    let mut state = fixture();
    crate::update::tests::operation_live_as(
        &mut state,
        mergewright_ui::graph_screen::LiveC::ContinueAmends,
        true,
    );
    state
}

#[test]
fn tab_in_the_picker_at_a_live_stop_is_the_pickers() {
    let open = opened(&at_a_live_stop());
    assert!(shown(&open), "C opens it at an edit stop");
    let (state, commands) = press(&open, KeyCode::Tab);
    assert!(!state.banner_focus, "the banner did not take the Tab");
    assert!(state.picker.is_none(), "Tab closed the picker");
    assert!(operated(&commands).is_none());
    assert_eq!(
        state
            .history
            .as_ref()
            .and_then(crate::history::History::selected)
            .map(|row| row.commit.id),
        Some(oid(3)),
        "the cursor on feature's tip"
    );
}

#[test]
fn letters_in_the_picker_at_a_live_stop_are_typed() {
    let open = opened(&at_a_live_stop());
    let state = typed(&open, "csa");
    assert_eq!(picker(&state).query.text(), "csa");
    assert!(!state.banner_focus);
    assert!(state.confirm.is_none(), "no skip or abort box");
    assert!(state.operating.is_none(), "nothing continued");
}

mod current_row;
mod seek;
