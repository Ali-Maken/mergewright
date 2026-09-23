//! The resolver's keys and answers over hand-built states — the races and
//! refusals a real repository cannot be made to hit on cue (m6-05 review).

use std::path::Path;
use std::sync::Arc;

use graft_core::event::{Event, KeyCode, KeyPress};
use graft_core::job::JobId;
use graft_ui::conflict_resolver::{Layout, ScreenRow};

use super::scroll_block_into_view;
use crate::update::{AppEvent, AppState, Command, Product, Size, Status, update};

const WALK: JobId = JobId::new(4_100);

/// The resolver open on a two-block document, at a size it draws in.
fn resolver_on_screen(walk: JobId) -> AppState {
    sized(crate::update::tests::resolver_on_screen(walk))
}

/// The resolver open on a `deleted by them` card, at a size it draws in.
fn resolver_card_on_screen(walk: JobId) -> AppState {
    sized(crate::update::tests::resolver_card_on_screen(walk))
}

fn sized(state: AppState) -> AppState {
    AppState {
        size: Size {
            columns: 120,
            rows: 48,
        },
        ..state
    }
}

fn press(state: &AppState, code: KeyCode) -> (AppState, Vec<Command>) {
    update(state, AppEvent::Core(Event::Key(KeyPress::plain(code))))
}

/// The status line's reason and keys, or a panic naming what it was.
fn sticky(state: &AppState) -> (&str, Option<&str>) {
    match &state.status {
        Status::Sticky { reason, keys } => (reason.as_str(), keys.as_deref()),
        other => panic!("expected a sticky line, got {other:?}"),
    }
}

fn open_path(state: &AppState) -> &Path {
    &state.resolver.as_ref().expect("the resolver is open").path
}

// ------------------------------------------------ Major 1: a whole-file card

/// `n` on a card crosses to the next conflicted file, as it does from a
/// document's last block — before, it returned silently, and the only way
/// off a card was `Esc`.
#[test]
fn n_on_a_card_crosses_to_the_next_file() {
    let (state, commands) = press(&resolver_card_on_screen(WALK), KeyCode::Char('n'));
    assert_eq!(open_path(&state), Path::new("later.rs"));
    assert!(
        commands
            .iter()
            .any(|command| matches!(command, Command::OpenConflict(_, path) if path == Path::new("later.rs"))),
        "{commands:?}"
    );
}

/// On the last conflicted file there is nowhere to cross to, and the line
/// says so with the keys a card does answer — not `⏎`, which it does not.
#[test]
fn n_on_the_last_card_says_no_more() {
    let mut state = resolver_card_on_screen(WALK);
    state.working_tree = Some(Arc::new(graft_git::status::WorkingTreeStatus {
        entries: vec![graft_git::status::StatusEntry {
            path: "gone.txt".into(),
            original_path: None,
            staged: None,
            unstaged: None,
            untracked: false,
            unmerged: true,
            unmerged_how: None,
            submodule: false,
        }],
    }));
    let (state, _) = press(&state, KeyCode::Char('n'));
    assert_eq!(open_path(&state), Path::new("gone.txt"));
    assert_eq!(
        sticky(&state),
        ("no more conflicts", Some("o ours  t theirs"))
    );
}

/// `d` on a card is refused in words, before any box: `checkout --merge`
/// has no markers to bring back for a whole-file shape, and the box would
/// promise them (ticket scope 3).
#[test]
fn d_on_a_card_is_refused_in_words() {
    let (state, commands) = press(&resolver_card_on_screen(WALK), KeyCode::Char('d'));
    assert!(state.confirm.is_none(), "no box");
    assert!(
        !commands
            .iter()
            .any(|command| matches!(command, Command::Operate(..))),
        "{commands:?}"
    );
    assert_eq!(
        sticky(&state).0,
        "gone.txt: deleted by them — no markers to take from"
    );
}

// ------------------------------------- Major 2: a base load and a take race

/// `B` then the id of the `LoadBase` it queued.
fn base_loading(state: &AppState) -> (AppState, JobId) {
    let (state, commands) = press(state, KeyCode::Char('B'));
    let id = commands
        .iter()
        .find_map(|command| match command {
            Command::LoadBase(id, ..) => Some(*id),
            _ => None,
        })
        .unwrap_or_else(|| panic!("`B` queued no LoadBase: {commands:?}"));
    (state, id)
}

/// The `LoadBase` answer, carrying the document the resolver had when `B`
/// was pressed.
fn base_answer(state: &AppState, id: JobId) -> AppEvent {
    let file = state
        .resolver
        .as_ref()
        .and_then(|open| open.file.clone())
        .expect("a document");
    AppEvent::Produced(id, Product::BaseLoaded(Ok((file, 2))))
}

fn takes(commands: &[Command]) -> usize {
    commands
        .iter()
        .filter(|command| matches!(command, Command::TakeSide(..)))
        .count()
}

/// A take is held while a base load is in flight: the load carries the
/// document as it was at `B`, and landing after a take would put the taken
/// block back on screen while the file on disk has it taken.
#[test]
fn a_take_waits_for_the_base_load() {
    let (state, id) = base_loading(&resolver_on_screen(WALK));
    let before = state.clone();
    let (state, commands) = press(&state, KeyCode::Char('o'));
    assert_eq!(takes(&commands), 0, "no take while the base loads");
    assert_eq!(sticky(&state).0, "still loading the base");

    let (state, _) = update(&state, base_answer(&before, id));
    let open = state.resolver.as_ref().expect("open");
    assert!(open.base_shown && open.base_job.is_none());
    let (_, commands) = press(&state, KeyCode::Char('o'));
    assert_eq!(takes(&commands), 1, "the take goes once the base is in");
}

/// `e` hands the terminal over with a base load in flight: the re-read the
/// hand-back starts owns the document, and the base answer — carrying the
/// document from before the edit — is dropped rather than landing over it.
#[test]
fn a_hand_back_drops_the_base_in_flight() {
    let (state, id) = base_loading(&resolver_on_screen(WALK));
    let answer = base_answer(&state, id);
    let (state, commands) = update(
        &state,
        AppEvent::Produced(JobId::new(4_200), Product::HandedOver(Ok(()))),
    );
    assert!(
        commands
            .iter()
            .any(|command| matches!(command, Command::OpenConflict(..))),
        "the hand-back re-reads: {commands:?}"
    );
    assert!(
        state
            .resolver
            .as_ref()
            .is_some_and(|open| open.base_job.is_none())
    );

    let (after, _) = update(&state, answer);
    assert_eq!(
        after.resolver, state.resolver,
        "the stale base answer changed nothing"
    );
}

// ---------------------------------------- Major 3: `d` over a running write

/// `d` while a write runs is refused in words, before the box: confirming
/// would launch a second git write over the first.
#[test]
fn d_is_refused_while_a_write_runs() {
    let mut state = resolver_on_screen(WALK);
    state.operating = Some(JobId::new(4_300));
    let (state, _) = press(&state, KeyCode::Char('d'));
    assert!(state.confirm.is_none(), "no box");
    assert_eq!(sticky(&state).0, "a write is already running");
}

/// A write that began while the box was up is caught at the accept too:
/// the box is answered, nothing is launched, the first write keeps its id.
#[test]
fn start_over_is_refused_if_a_write_began_under_the_box() {
    let (state, _) = press(&resolver_on_screen(WALK), KeyCode::Char('d'));
    assert!(state.confirm.is_some(), "the box is up");
    let mut state = state;
    let running = JobId::new(4_301);
    state.operating = Some(running);
    let (state, _) = press(&state, KeyCode::Right);
    let (state, commands) = press(&state, KeyCode::Enter);
    assert!(state.confirm.is_none(), "the box was answered");
    assert!(
        !commands
            .iter()
            .any(|command| matches!(command, Command::Operate(..))),
        "{commands:?}"
    );
    assert_eq!(state.operating, Some(running));
    assert!(
        state
            .resolver
            .as_ref()
            .is_some_and(|open| open.restarting.is_none())
    );
    assert_eq!(sticky(&state).0, "a write is already running");
}

// ------------------------------- m6-07 F7: a refusal words what is in flight

/// The resolver as `⏎` leaves it: the opening read queued, nothing parsed.
fn opening_read() -> AppState {
    let mut state = resolver_on_screen(WALK);
    state.resolver = None;
    let mut commands = Vec::new();
    let mut ids = 4_400;
    let opened = super::open_resolver(
        &mut state,
        &mut commands,
        std::path::PathBuf::from("conflicted.rs"),
        super::OpenedFrom::Pane,
        &mut || {
            ids += 1;
            JobId::new(ids)
        },
    );
    assert!(opened, "the resolver opened");
    assert!(
        state
            .resolver
            .as_ref()
            .is_some_and(|open| open.job.is_some() && open.file.is_none()),
        "the opening read is in flight"
    );
    state
}

/// A key on the resolver before its opening read lands is refused with
/// what is actually happening — no take was pressed, so "still writing the
/// last take" was a lie (acceptance finding F7).
#[test]
fn a_key_during_the_opening_read_says_it_is_reading() {
    for key in ['o', 't', 'b', 'B', 'd'] {
        let (state, commands) = press(&opening_read(), KeyCode::Char(key));
        assert_eq!(takes(&commands), 0, "{key}: no take");
        assert!(state.confirm.is_none(), "{key}: no box");
        assert_eq!(sticky(&state).0, "still reading the file", "{key}");
    }
}

/// The re-read after `e`/`M` hands back is a read too, though the old
/// document is still on screen.
#[test]
fn a_key_during_the_hand_back_reread_says_it_is_reading() {
    let (state, _) = update(
        &resolver_on_screen(WALK),
        AppEvent::Produced(JobId::new(4_200), Product::HandedOver(Ok(()))),
    );
    let (state, commands) = press(&state, KeyCode::Char('o'));
    assert_eq!(takes(&commands), 0);
    assert_eq!(sticky(&state).0, "still reading the file");
}

/// A take in flight keeps its own words, and `B` over a base load says the
/// base is loading, not that a take is being written.
#[test]
fn a_take_or_a_base_in_flight_keeps_its_own_words() {
    let (state, commands) = press(&resolver_on_screen(WALK), KeyCode::Char('o'));
    assert_eq!(takes(&commands), 1);
    let (state, _) = press(&state, KeyCode::Char('t'));
    assert_eq!(sticky(&state).0, "still writing the last take");

    let (state, _) = base_loading(&resolver_on_screen(WALK));
    let mut state = state;
    state.resolver.as_mut().expect("open").base_shown = false;
    let (state, _) = press(&state, KeyCode::Char('B'));
    assert_eq!(sticky(&state).0, "still loading the base");
}

// ----------------------------------- m6-07 F1: the modal owns `/`, `n`, `N`

/// `/` over the resolver opens no search prompt behind it: the resolver is
/// modal (§2.1), and the search's keys were answered before the modal chain.
#[test]
fn slash_opens_no_search_behind_the_resolver() {
    let (state, _) = press(&resolver_on_screen(WALK), KeyCode::Char('/'));
    assert!(
        state.search_prompt.is_none(),
        "no prompt behind the resolver"
    );
    assert!(state.resolver.is_some(), "the resolver is still open");
}

/// With a search from before the resolver still live, `n` walks the
/// resolver's blocks — it does not step the search behind it.
#[test]
fn n_walks_blocks_even_with_a_search_live() {
    let mut state = resolver_on_screen(WALK);
    let query = crate::search::Query::parse("root").expect("a non-empty query");
    let mut search = crate::search::Search::new(query, 0, (0, 0));
    search.state = crate::search::SearchState::Done { hits: 0 };
    state.search = Some(search);
    let before = state.search.clone();
    let (state, _) = press(&state, KeyCode::Char('n'));
    assert_eq!(
        state.resolver.as_ref().and_then(|open| open.cursor_block),
        Some(1),
        "n moved to the second block"
    );
    assert_eq!(state.search, before, "the search was not stepped");
}

// ------------------------- m6-07 F4: `c continue` only where `c` continues

/// The last file resolved in a stopped merge, the resolver opened from the
/// changes pane: the `Stage` `⏎` launched lands.
fn last_file_staged_from_the_pane() -> AppState {
    let mut state = resolver_on_screen(WALK);
    state.in_progress = Some(graft_git::state::InProgressDetail {
        kind: graft_git::state::InProgress::Merge,
        step: None,
        head_name: None,
        onto: None,
    });
    state.banner_focus = false;
    let staged = JobId::new(4_500);
    state.resolver.as_mut().expect("open").staging = Some(staged);
    let mut commands = Vec::new();
    let mut ids = 4_500;
    super::resolved_advance(&mut state, &mut commands, staged, &mut || {
        ids += 1;
        JobId::new(ids)
    });
    // The re-read the `Stage` asked for lands: nothing is unmerged.
    state.working_tree = Some(Arc::new(graft_git::status::WorkingTreeStatus {
        entries: Vec::new(),
    }));
    state
}

/// The line offers `c continue`, so the key under it must be the banner's:
/// the design note closes the last file "to the banner". Before, a resolver
/// opened from the pane closed to the pane, and `c` there opened the commit
/// editor (acceptance finding F4, `resolver/answers.rs:235-240`).
#[test]
fn all_resolved_from_the_pane_offers_c_where_c_continues() {
    let state = last_file_staged_from_the_pane();
    assert!(
        state.resolver.is_none(),
        "the last file closed the resolver"
    );
    assert_eq!(
        state.status,
        Status::Done("all conflicts resolved · c continue".to_owned())
    );
    assert!(state.banner_focus, "the keyboard is on the banner");
    let (state, commands) = press(&state, KeyCode::Char('c'));
    assert!(
        state.commit_editor.is_none(),
        "c did not open the commit editor"
    );
    assert!(
        commands.iter().any(|command| matches!(
            command,
            Command::Resume(_, graft_ops::apply::Resume::Continue)
        )),
        "c continued the merge: {commands:?}"
    );
}

// ------------------------------------------------------------ scrolling

/// A hundred rows with a block header every ten.
fn layout_of(rows: usize, headers: &[usize]) -> Layout {
    Layout {
        rows: (0..rows)
            .map(|row| match headers.iter().position(|at| *at == row) {
                Some(block) => ScreenRow::Header { block },
                None => ScreenRow::Plain {
                    line_no: row + 1,
                    text: graft_term::sanitize_line("a line"),
                },
            })
            .collect(),
        block_row: headers.to_vec(),
        columns: 120,
    }
}

/// A take lands on a block the window has scrolled past: the window
/// moves, and it moves so the header is the **first** row, since
/// everything the block is about is below it.
#[test]
fn take_scrolls_the_block_into_view() {
    let layout = layout_of(120, &[10, 50, 90]);

    assert_eq!(
        scroll_block_into_view(&layout, 2, 0, 20),
        90,
        "block 2's header is the first row of the window"
    );
    assert_eq!(
        scroll_block_into_view(&layout, 0, 5, 20),
        5,
        "already on screen: the window does not move at all"
    );
    assert_eq!(
        scroll_block_into_view(&layout, 0, 40, 20),
        10,
        "scrolling back is the same rule in the other direction"
    );
    assert_eq!(
        scroll_block_into_view(&layout, 9, 7, 20),
        7,
        "a block that is not there moves nothing"
    );
}

/// Never past the end: the last screenful is as far as the window goes,
/// or the rows below the document would be blank.
#[test]
fn the_window_stops_at_the_last_screenful() {
    let layout = layout_of(100, &[10, 95]);
    assert_eq!(scroll_block_into_view(&layout, 1, 0, 20), 80);
}

// ------------- m6-04-08 review, Major 2: one write on this file at a time

/// The resolver on a document whose every block is taken, so `⏎` is not
/// refused for open blocks and a re-take (`t` over a taken block) is live.
fn all_taken() -> AppState {
    let mut state = resolver_on_screen(WALK);
    let open = state.resolver.as_mut().expect("open");
    let mut file = open.file.as_deref().expect("a document").clone();
    file.take(0, graft_git::conflict::Take::Ours)
        .expect("block 0");
    file.take(1, graft_git::conflict::Take::Ours)
        .expect("block 1");
    open.file = Some(Arc::new(file));
    state
}

/// The git writes (`⏎`'s `Stage`) and fs writes (a block take) queued.
fn writes(commands: &[Command]) -> usize {
    takes(commands)
        + commands
            .iter()
            .filter(|command| matches!(command, Command::Operate(..)))
            .count()
}

/// `t` then `⏎` before the take lands: the stage would `git add` a file
/// the take is still rewriting, and the index could hold the old take while
/// the screen and the disk show the new one.
#[test]
fn enter_waits_for_a_take_in_flight() {
    let (state, first) = press(&all_taken(), KeyCode::Char('t'));
    assert_eq!(takes(&first), 1, "the re-take was queued");
    let (state, second) = press(&state, KeyCode::Enter);
    assert_eq!(writes(&second), 0, "no stage over a take: {second:?}");
    assert!(state.operating.is_none(), "no git write began");
    assert_eq!(sticky(&state).0, "still writing the last take");
}

/// `⏎` then `t` before the stage lands: the take would rewrite the file
/// `git add` is reading.
#[test]
fn a_take_waits_for_the_stage_in_flight() {
    let (state, first) = press(&all_taken(), KeyCode::Enter);
    assert_eq!(writes(&first), 1, "the stage was queued: {first:?}");
    let (state, second) = press(&state, KeyCode::Char('t'));
    assert_eq!(writes(&second), 0, "no take over a stage: {second:?}");
    assert_eq!(sticky(&state).0, "a write is already running");
}
