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

/// On the last conflicted file there is nowhere to cross to: the card's
/// notice row says so (m6-conflicts/61), in the words a document uses and
/// with no keys in them. The side keys a card answers — not `⏎`, which it
/// does not — stay in the bar under it.
#[test]
fn a_card_says_no_more_this_way_and_its_bar_keeps_the_side_keys() {
    use crate::update::resolver::walk::NO_MORE_THIS_WAY;

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
    let before = state.status.clone();
    let (state, _) = press(&state, KeyCode::Char('n'));
    assert_eq!(open_path(&state), Path::new("gone.txt"));
    assert_eq!(
        state.resolver.as_ref().expect("open").notice,
        Some((NO_MORE_THIS_WAY.to_owned(), false))
    );
    assert_eq!(state.status, before, "the status line is not touched");
    let frame = frame_text(&state);
    let bar = frame.lines().last().expect("a bottom row");
    assert!(!bar.contains("no more conflicts"), "{bar}");
    // No operation in progress with the resolver open: a stash pop's roles.
    assert!(bar.contains("o yours"), "{bar}");
    assert!(bar.contains("t stashed"), "{bar}");
}

/// `resolver_card_on_screen` as the only conflicted file, mid-rebase.
fn last_card_in_a_rebase() -> AppState {
    use crate::update::resolver::fixtures::{commit, oid, operation, refs};
    let mut state = resolver_card_on_screen(WALK);
    state.refs = Some(refs(false));
    state.in_progress = Some(operation(
        graft_git::state::InProgress::Rebase,
        Some(commit(oid(3), "Add OAuth provider")),
    ));
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
    state
}

/// Review m6-amend-10-22 M2 (`walk.rs`): in a rebase git's `ours` is
/// upstream, so the card's bar under `n`'s notice offers the rebase's own
/// role words.
#[test]
fn n_on_the_last_card_in_a_rebase_uses_the_side_names() {
    let (state, _) = press(&last_card_in_a_rebase(), KeyCode::Char('n'));
    assert_eq!(
        state
            .resolver
            .as_ref()
            .and_then(|open| open.notice.as_ref())
            .map(|(words, _)| words.as_str()),
        Some(crate::update::resolver::walk::NO_MORE_THIS_WAY)
    );
    let frame = frame_text(&state);
    let bar = frame.lines().last().expect("a bottom row");
    assert!(bar.contains("o upstream"), "{bar}");
    assert!(bar.contains("t yours"), "{bar}");
}

/// Review m6-amend-10-22 M2 (`restart.rs`): `d` on a card in a rebase is
/// refused with the rebase's role words on its keys.
#[test]
fn d_on_a_card_in_a_rebase_uses_the_side_names() {
    let (state, _) = press(&last_card_in_a_rebase(), KeyCode::Char('d'));
    assert!(state.confirm.is_none(), "no box");
    assert_eq!(sticky(&state).1, Some("o upstream  t yours"));
}

/// No string the resolver's update code writes names a side `ours` or
/// `theirs` outright: every side word comes from `side_names`' roles, which
/// a rebase swaps (ADR-0043 §4). Scans the non-test source for literals;
/// `{ours}` placeholders are the side names themselves and pass.
#[test]
fn no_resolver_status_carries_a_bare_ours_or_theirs() {
    let sources = [
        ("resolver.rs", include_str!("../resolver.rs")),
        ("answers.rs", include_str!("answers.rs")),
        ("base.rs", include_str!("base.rs")),
        ("geometry.rs", include_str!("geometry.rs")),
        ("names.rs", include_str!("names.rs")),
        ("restart.rs", include_str!("restart.rs")),
        ("scroll.rs", include_str!("scroll.rs")),
        ("take.rs", include_str!("take.rs")),
        ("walk.rs", include_str!("walk.rs")),
        ("whole_file.rs", include_str!("whole_file.rs")),
    ];
    let bare = |literal: &str| {
        let unplaced = literal.replace("{ours}", "").replace("{theirs}", "");
        unplaced
            .split(|c: char| !c.is_alphanumeric() && c != '_')
            .any(|word| word == "ours" || word == "theirs")
    };
    for (name, source) in sources {
        let code = source.split("#[cfg(test)]").next().unwrap_or(source);
        for (number, line) in code.lines().enumerate() {
            if line.trim_start().starts_with("//") {
                continue;
            }
            // Odd-numbered pieces between `"` are the literals on the line.
            for literal in line.split('"').skip(1).step_by(2) {
                assert!(
                    !bare(literal),
                    "{name}:{}: {literal:?} names a side outright",
                    number + 1
                );
            }
        }
    }
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

// `a_take_or_a_base_in_flight_keeps_its_own_words` became
// `advance::a_take_while_another_job_runs_keeps_its_own_words`: a take in
// flight now queues the next one (m6-conflicts/15).

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
        theirs: Ok(None),
    });
    state.banner_focus = false;
    // The last file: nothing else unmerged.
    state.working_tree = Some(file_list::status(&[("conflicted.rs", true)]));
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
    // And the file's own read: done, read-only (m6-conflicts/31).
    words_and_c::done_read_lands(&state, &commands)
}

/// The notice offers `c`, so the key under it must be one where `c`
/// continues (acceptance finding F4: from the pane, `c` once opened the
/// commit editor). Since m6-conflicts/31 that is the resolver itself: it
/// stays open after the last file, whichever way it was opened, and the
/// keyboard stays where it was. Rewritten for m6-conflicts/60: opened from
/// the pane, the merge's summary opens on its own too; `Esc` from it leaves
/// the finish row, and `c` there opens it again — never the commit editor.
#[test]
fn all_resolved_from_the_pane_offers_c_where_c_continues() {
    let state = last_file_staged_from_the_pane();
    assert_eq!(open_path(&state), Path::new("conflicted.rs"), "it stays");
    let finish = state
        .finish
        .as_ref()
        .expect("the summary opened on its own");
    assert_eq!(finish.return_to, crate::update::finish::ReturnTo::Resolver);
    assert!(state.commit_editor.is_none());
    assert!(!state.banner_focus, "banner_focus unchanged");
    let (state, _) = press(&state, KeyCode::Escape);
    assert!(state.finish.is_none(), "Esc closed the summary");
    assert_eq!(state.resolver.as_ref().expect("open").notice, None);
    assert_eq!(
        crate::view::resolver::finish_row(&state).map(|row| row.full.as_str().to_owned()),
        Some("✓ every file done — c review and finish the merge".to_owned()),
        "the finish row offers c"
    );
    let (state, commands) = press(&state, KeyCode::Char('c'));
    assert!(
        state.commit_editor.is_none(),
        "c did not open the commit editor"
    );
    // A merge's `c` shows the finish summary, whose `y` commits
    // (m6-conflicts/19): nothing is written by `c` itself.
    assert!(state.finish.is_some(), "c opened the finish summary");
    assert!(
        commands
            .iter()
            .any(|command| matches!(command, Command::ReadFinishSummary(_))),
        "c on the merge reads the summary: {commands:?}"
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

/// The status line after a take of block 0 lands, in `kind`.
fn took_line(kind: graft_git::state::InProgress) -> String {
    use crate::update::resolver::fixtures::{commit, oid, operation, refs};
    let mut state = resolver_on_screen(WALK);
    state.refs = Some(refs(false));
    state.in_progress = Some(operation(kind, Some(commit(oid(3), "Add OAuth provider"))));
    let open = state.resolver.as_mut().expect("the resolver is open");
    open.cursor_block = Some(0);
    open.in_flight = Some((
        crate::update::TakeAt::Block(0),
        graft_git::conflict::Take::Theirs,
    ));
    let mut file = (**open.file.as_ref().expect("a document")).clone();
    file.take(0, graft_git::conflict::Take::Theirs)
        .expect("block 0 exists");
    super::answers::taken::take_landed(&mut state, &Arc::new(file), &mut Vec::new(), &mut || {
        JobId::new(4_600)
    });
    match &state.status {
        Status::Done { what: words, .. } => words.clone(),
        other => panic!("expected a done status, got {other:?}"),
    }
}

/// ADR-0043 §4: a take is worded by the side's one name.
#[test]
fn took_line_uses_the_side_name() {
    use graft_git::state::InProgress;
    let merge = took_line(InProgress::Merge);
    assert!(merge.starts_with("took theirs · feature in "), "{merge}");
    let rebase = took_line(InProgress::Rebase);
    assert!(
        rebase.starts_with("took yours · 0303030 Add OAuth provider in "),
        "{rebase}"
    );
}

/// The resolver's bar as `key verb` strings: the context the footer draws
/// from (`view::rows::resolver_context`), through the same table.
fn bar(state: &AppState) -> Vec<String> {
    use graft_ui::graph_screen::{Divider, hints_for};
    hints_for(
        &crate::view::rows::resolver_context(state),
        Divider::Hidden,
        200,
    )
    .iter()
    .map(|hint| format!("{} {}", hint.key, hint.verb))
    .collect()
}

// Kept here, not in `file_list.rs`: `update_cannot_reach_the_world` skips
// only files named `tests.rs`, and this one draws through ratatui.
/// Every row `state` draws at its own size, joined by newlines.
fn frame_text(state: &AppState) -> String {
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    let (columns, rows) = (state.size.columns, state.size.rows);
    let mut terminal =
        Terminal::new(TestBackend::new(columns, rows)).expect("TestBackend reports its size");
    terminal
        .draw(|frame| crate::view::view(&crate::session::for_tests::session(), state, frame))
        .expect("TestBackend cannot fail to draw");
    let buffer = terminal.backend().buffer();
    (0..rows)
        .map(|y| {
            (0..columns)
                .map(|x| buffer[(x, y)].symbol())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

mod advance;
mod cross;
mod file_keys;
mod file_list;
mod finish_here;
mod opens_itself;
mod take_all;

/// m6-conflicts/46: `B` on a file of one block says `1 conflict`.
#[test]
fn base_shown_words_are_singular_at_one() {
    use super::base::base_shown_words;
    assert_eq!(base_shown_words(1), "base shown for 1 conflict");
    assert_eq!(base_shown_words(3), "base shown for 3 conflicts");
}
mod three_panes;
mod words_and_c;
