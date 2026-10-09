use crate::spelled::Spelled as _;
use graft_core::capabilities::RenderingTier;
use graft_core::event::{Event, KeyCode, KeyPress, Modifiers, MouseButton, MouseInput, MouseKind};
use graft_core::job::{JobFailure, JobId};

use graft_core::commit::{Commit, Identity, Timestamp};
use graft_core::oid::Oid;
use graft_git::history::Order;
use graft_git::version::SystemGit;
use graft_graph::{Follow, Row};

use super::geometry::HEADER_ROWS;
use super::mouse::WHEEL_LINES;
use super::*;
use std::ops::Range;
use std::path::PathBuf;
use std::sync::Arc;

use crate::history::{
    History, HistoryUpdate, Phase, Promotion, RowRequest, View, WalkFacts, Wanted, Window,
};
use crate::repository::{Head, RepositoryFacts};
use crate::search::{Search, SearchState};
use graft_ui::graph_screen::{CommitGraphLine, Divider, Hint, HintContext, HintKey, OrderNotice};
use graft_ui::keymap::{ActionId, KeyChord, Keymap};

use super::detail::Loading as EvilLoading;
use super::detail::{DetailSpec, DetailState, LOCAL_JOB_BIT, Loading, LocalJobs};
use crate::search::Query;
use graft_git::diff::{DiffOptions, FileEntry, FileList, FileStatus};
use graft_ops::mistake_radar::probes::EvilVerdict;
use graft_term::sanitize::sanitize_line;
use graft_ui::detail::{DetailFocus, DiffView, FileListView, FileRow, FileRowKind, FileStatusView};
use std::sync::Arc as DetailArc;

fn identity() -> Identity {
    Identity {
        name: b"Test".to_vec(),
        email: b"test@example.invalid".to_vec(),
        when: Timestamp {
            seconds: 0,
            offset_seconds: 0,
        },
    }
}

pub(super) fn key(code: KeyCode) -> AppEvent {
    AppEvent::Core(Event::Key(KeyPress::plain(code)))
}

/// A walk in flight whose window holds a plain commit (row 0) and a
/// merge (row 1), the cursor on row `cursor`.
pub(super) fn merge_on_screen(id: JobId, cursor: usize) -> (AppState, Oid) {
    let commit = |byte: u8, parents: Vec<Oid>| Row {
        commit: Commit {
            id: Oid::from_bytes(&[byte; Oid::SHA1_BYTES]).expect("20 bytes is an oid"),
            parents,
            author: identity(),
            committer: identity(),
            message: b"a commit".to_vec(),
        },
        lane: 0,
        incoming: Vec::new(),
        outgoing: Vec::new(),
        lanes: Vec::new(),
        marker: None,
    };
    let parent = |byte| Oid::from_bytes(&[byte; Oid::SHA1_BYTES]).expect("an oid");
    let plain = commit(1, vec![parent(2)]);
    let merge = commit(2, vec![parent(3), parent(4)]);
    let merge_id = merge.commit.id;
    let window = Window {
        request: 1,
        first: 0,
        rows: vec![plain, merge].into(),
        laid_out: 2,
        done: false,
        promoted: false,
    };
    let (mut state, _) = update(
        &awaiting(id),
        AppEvent::Progress(id, Progress::History(HistoryUpdate::Window(window))),
    );
    state.history.as_mut().expect("a walk").cursor = cursor;
    // Refs read, no stash: the evil probe waits for them (m6/66).
    state.refs = Some(Arc::new(crate::refs::RefBadges::default()));
    (state, merge_id)
}

/// The view the last `ShowRows` asks for.
pub(super) fn asked_view(commands: &[Command]) -> View {
    // The evil-merge probe rides along on any event while the cursor rests
    // on a merge (M2 review M3), so the row request is picked out rather
    // than expected to be the only command.
    let mut rows = commands.iter().filter_map(|command| match command {
        Command::ShowRows(_, request) => Some(request.view.clone()),
        _ => None,
    });
    let view = rows
        .next()
        .unwrap_or_else(|| panic!("expected one row request, got {commands:?}"));
    assert!(
        rows.next().is_none(),
        "expected one row request, got {commands:?}"
    );
    view
}

pub(super) fn core(event: Event) -> AppEvent {
    AppEvent::Core(event)
}

fn facts(path: &str, branch: &str) -> RepositoryFacts {
    RepositoryFacts {
        path: PathBuf::from(path),
        bare: false,
        head: Head::Branch(branch.as_bytes().to_vec()),
        mailmap: graft_git::mailmap::Mailmap::default(),
        commit_graph: crate::repository::CommitGraphFact::Present { commits: 1 },
    }
}

/// Job `id`'s answer to the first request, with `n` rows laid out. The
/// rows themselves do not matter here, only whether they land, so they are
/// copies of one row.
pub(super) fn rows(id: JobId, n: usize) -> AppEvent {
    let row = Row {
        commit: Commit {
            id: Oid::from_bytes(&[7; Oid::SHA1_BYTES]).expect("20 bytes is an oid"),
            parents: Vec::new(),
            author: identity(),
            committer: identity(),
            message: b"a commit".to_vec(),
        },
        lane: 0,
        incoming: Vec::new(),
        outgoing: Vec::new(),
        lanes: Vec::new(),
        marker: None,
    };
    AppEvent::Progress(
        id,
        Progress::History(HistoryUpdate::Window(Window {
            request: 1,
            first: 0,
            rows: vec![row; n.min(3)].into(),
            laid_out: n,
            done: false,
            promoted: false,
        })),
    )
}

/// The state of an application with a walk in flight.
pub(super) fn awaiting(id: JobId) -> AppState {
    let (next, commands) = walk_started(&AppState::default(), id);

    // A default state is zero rows tall, which shows one row: the first
    // screen, and one more screen below it.
    assert_eq!(
        commands,
        vec![Command::ShowRows(id, request(1, Wanted::Rows(0..4)))],
        "a walk that has started is asked for its first screen"
    );
    assert_eq!(next.awaiting, Some(id));
    assert_eq!(next.history.as_ref().map(|history| history.asked), Some(1));
    next
}

/// Where the walk on screen is, and how many rows it has.
pub(super) fn progress(state: &AppState) -> (Phase, usize) {
    let history = state.history.as_ref().expect("a walk was asked for");
    (history.phase.clone(), history.rows)
}

pub(super) fn request(id: u64, wanted: Wanted) -> RowRequest {
    RowRequest {
        id,
        wanted,
        view: View::default(),
        promote: None,
    }
}

pub(super) fn sized(columns: u16, rows: u16) -> AppEvent {
    core(Event::Resize { columns, rows })
}

/// Job `id`'s answer to request `request`: `span` rows, `laid_out` so far.
pub(super) fn rows_for(id: JobId, request: u64, span: Range<usize>, laid_out: usize) -> AppEvent {
    let AppEvent::Progress(_, Progress::History(HistoryUpdate::Window(window))) =
        rows(id, span.len())
    else {
        unreachable!("`rows` answers with a window");
    };
    let one = window.rows[0].clone();
    AppEvent::Progress(
        id,
        Progress::History(HistoryUpdate::Window(Window {
            request,
            first: span.start,
            rows: vec![one; span.len()].into(),
            laid_out,
            done: false,
            promoted: false,
        })),
    )
}

/// A provisional walk and the topological sort it asked for, started as
/// job `sort`.
pub(super) fn sorting(walk: JobId, sort: JobId) -> AppState {
    let (next, commands) = update(
        &awaiting(walk),
        from(walk, started(Order::Topological, Order::CommitDate)),
    );
    assert!(
        commands.contains(&Command::PromoteHistory),
        "a provisional walk asks for its order: {commands:?}"
    );
    let (next, _) = update(&next, AppEvent::Started(sort, Task::PromoteHistory));
    assert_eq!(next.promoting, Some(sort));
    next
}

pub(super) fn order() -> std::sync::Arc<[graft_core::commit_handle::CommitHandle]> {
    vec![graft_core::commit_handle::CommitHandle(7)].into()
}

/// [`order`] as a sort's answer, from a sort nobody waited for.
pub(super) fn sorted_order() -> SortedOrder {
    SortedOrder {
        order: order(),
        took: std::time::Duration::ZERO,
    }
}

mod pane_space;

mod first_launch;
mod place_footers;

mod keymap_edge;

mod keymap_rules;

mod keymap_words;

mod palette_live;

mod pixel_preview;

mod commit_graph_offer;

/// Critical 1 of the m3/06 review: a global binding must never fire while a
/// text field has the keyboard. Typing `update` as a stash message launched
/// an undo at the `u` — a journaled write nobody asked for, from inside a
/// field the same milestone had just added.
///
/// One test per text-entry surface in the app, each driven through
/// [`update`] so the whole key path runs, because the defect was in the
/// *order* of that path and a handler called directly cannot see it.
mod a_text_field_keeps_its_own_letters;

/// An operation live over `state` whose `c` in the graph or the changes
/// pane is `c` (m6-conflicts/52), the banner unfocused: a merge for
/// `Finish`, a rebase Graft began for `Continue`. For `Refuses`, a revert
/// the shell began where the banner is to be one `⇥` away, else a bisect —
/// which has no banner at all.
pub(super) fn operation_live_as(
    state: &mut AppState,
    c: graft_ui::graph_screen::LiveC,
    banner: bool,
) {
    use graft_git::state::InProgress;
    use graft_ui::graph_screen::LiveC;
    let kind = match c {
        LiveC::Finish => InProgress::Merge,
        LiveC::Continue | LiveC::ContinueAmends => InProgress::Rebase,
        LiveC::Refuses if banner => InProgress::Revert,
        LiveC::Refuses => InProgress::Bisect,
    };
    state.in_progress = Some(graft_git::state::InProgressDetail {
        kind,
        step: None,
        head_name: None,
        onto: None,
        theirs: Ok(None),
    });
    if matches!(c, LiveC::Continue | LiveC::ContinueAmends) {
        state.live_entries = vec![graft_core::journal::EntryId::new(1, [0; 10])];
    }
    // A rewrite's `edit` stop (07b): the stop carries the edit-stop read.
    if c == LiveC::ContinueAmends {
        state.stopped = Some(crate::update::Stop {
            origin: crate::update::LiveOrigin::ThisRun,
            again: false,
            kind,
            paths_at_stop: 0,
            rewrite: Some(Ok(crate::update::banner::RewriteStop {
                progress: graft_ops::rewrite::RewriteProgress {
                    step: (1, 2),
                    current: None,
                    pending: Vec::new(),
                    branch: None,
                    begun_in_graft: true,
                    followers: Vec::new(),
                    fork: None,
                },
                edit: Some(graft_ops::rewrite::EditStop {
                    stopped: graft_core::oid::Oid::from_bytes(&[7; 20]).expect("20 bytes"),
                    head_on_it: true,
                }),
            })),
        });
    }
    state.banner_focus = false;
    // Checked, not trusted: `live_operation` reads "Graft began it" off
    // `live_entries`, and a bisect has no banner.
    let live = crate::update::commit_editor::live_operation(state).expect("an operation is live");
    assert_eq!(
        crate::update::commit_editor::pane_c_acts(live),
        c != LiveC::Refuses,
        "{live:?} is not a `{c:?}` state"
    );
    assert_eq!(
        crate::update::banner::banner_view(state).is_some(),
        kind != InProgress::Bisect
    );
}

/// The detail pane open on the cursor's commit, with a file list and a long
/// diff in it, focused where `opens` says: `⏎` opens the diff from the
/// files list, and has nothing left to open once the diff has it
/// (`detail::keys::enter_opens`).
pub(crate) fn commit_pane(walk: JobId, opens: bool) -> AppState {
    let (mut state, _) = merge_on_screen(walk, MERGE_ROW);
    let spec = state.selected_spec().expect("the cursor is on a commit");
    let focus = if opens {
        DetailFocus::Files
    } else {
        DetailFocus::Diff
    };
    show_pane(&mut state, spec, focus);
    let (view, raw) = pane_files();
    state.detail.files = Loading::Ready(view);
    state.detail.list = Some(raw);
    let (diff, diff_raw) = long_diff("a.rs");
    state.detail.diff = Loading::Ready(diff);
    state.detail.diff_raw = Some(diff_raw);
    state
}

/// The resolver open on a real two-block conflict, the state every one of
/// `resolver_list`'s live keys has to act in: a cursor on the first block so
/// `o`/`t`/`b` write, a second open block so `n` moves, a mergetool
/// configured so `M` hands over instead of refusing, and a working tree that
/// lists the file as unmerged so the title and `⏎` have a file to name —
/// and a second unmerged file after it, so `j` has a row to step to
/// (m6-conflicts/17).
pub(super) fn resolver_on_screen(walk: JobId) -> AppState {
    const CONFLICTED: &[u8] = b"one\n\
        <<<<<<< HEAD\nours a\n=======\ntheirs a\n>>>>>>> topic\n\
        two\n\
        <<<<<<< HEAD\nours b\n=======\ntheirs b\n>>>>>>> topic\n\
        three\n";
    let file = graft_git::conflict::parse(CONFLICTED).expect("two open blocks");
    let (mut state, _) = merge_on_screen(walk, MERGE_ROW);
    state.working_tree = Some(Arc::new(graft_git::status::WorkingTreeStatus {
        entries: vec![
            status_entry("conflicted.rs", true),
            status_entry("later.rs", true),
        ],
    }));
    state.mergetool = Some("vimdiff".to_string());
    state.resolver = Some(resolver::ResolverState {
        path: PathBuf::from("conflicted.rs"),
        opened_from: resolver::OpenedFrom::Pane,
        file: Some(Arc::new(file)),
        whole_file: None,
        how: None,
        cursor_block: Some(0),
        top: 0,
        base_shown: false,
        job: None,
        reading: false,
        base_job: None,
        staging: None,
        restarting: None,
        notice: None,
        files: crate::update::detail::Loading::Idle,
        set_job: None,
        seen: std::collections::BTreeMap::new(),
        done: None,
        in_flight: None,
        queued: std::collections::VecDeque::new(),
        land_last: false,
        dropped: 0,
        settling: None,
        whole_side: None,
        finish_owed: false,
    });
    state
}

/// The resolver open on a whole-file card (`deleted by them`), the state
/// every key a card's bar draws has to act in (m6-05 review, Major 1): a
/// second conflicted file after it so `n` has somewhere to cross to, and a
/// mergetool so `M` hands over.
pub(super) fn resolver_card_on_screen(walk: JobId) -> AppState {
    let (mut state, _) = merge_on_screen(walk, MERGE_ROW);
    state.working_tree = Some(Arc::new(graft_git::status::WorkingTreeStatus {
        entries: vec![
            status_entry("gone.txt", true),
            status_entry("later.rs", true),
        ],
    }));
    state.mergetool = Some("vimdiff".to_string());
    state.resolver = Some(resolver::ResolverState {
        path: PathBuf::from("gone.txt"),
        opened_from: resolver::OpenedFrom::Pane,
        file: None,
        whole_file: Some(resolver::WholeFileShape {
            how: graft_git::status::UnmergedHow::DeletedByThem,
            binary: false,
            on_disk: Some(crate::jobs::conflict::OnDisk {
                size: 11,
                head: Arc::from(vec![b"kept by us".to_vec()]),
                cut: false,
            }),
            start_over: crate::jobs::conflict::StartOver::Cannot {
                why: graft_ops::operation::Refusal::NotMarkerShape {
                    path: std::path::PathBuf::from("gone.txt"),
                    how: graft_git::status::UnmergedHow::DeletedByThem,
                }
                .explain(&crate::update::op_keys::OpKeys(
                    graft_ui::keymap::Keymap::shipped(),
                )),
            },
        }),
        how: Some(graft_git::status::UnmergedHow::DeletedByThem),
        cursor_block: None,
        top: 0,
        base_shown: false,
        job: None,
        reading: false,
        base_job: None,
        staging: None,
        restarting: None,
        notice: None,
        files: crate::update::detail::Loading::Idle,
        set_job: None,
        seen: std::collections::BTreeMap::new(),
        done: None,
        in_flight: None,
        queued: std::collections::VecDeque::new(),
        land_last: false,
        dropped: 0,
        settling: None,
        whole_side: None,
        finish_owed: false,
    });
    state
}

// Helpers more than one child uses, defined beside their first users
// (m8/21b-5 split this file by topic); the children reach them through here.
use dead_keys::{bar_hints, dead_keys_on};
use detail_pane::{diff_of, list_commands, two_files};
use doorway::pinned_harness;
use prelude::working_tree_read;
use resolver_screens::resolver_for;
use screens::walk_started;
use search::{found, on_hand, typed};
use sweep::{
    ALSO_INERT, Press, SWEPT_WIDTHS, actions_of, answered, did_something, nothing_happened,
    outcome, presses_for, settled, states_for,
};
use sweep_boxes::{
    EVERY_OVERLAY, Shown, box_over_the_graph, finish_screen_for, overlay_over, picker_over,
    two_branch_map,
};
use sweep_entries::{badge_in_the_air, journal_entry, stash_entry, status_entry};
use sweep_screens::{changes_pane_screen, history_screens, screen_for};
use sweep_states::{
    MERGE_ROW, changes_pane_open, head_spec, holds_search, holds_stash_row, long_diff,
    on_the_message, one_stash, pane_files, search_standing, show_pane, stash_under_cursor,
    stopped_operation,
};
use walk::{from, started};
use working_tree_row::dirty_on_screen;

mod bar_keys;
mod bar_over_overlay;
mod copy_and_compare;
mod cursor_drawn;
mod dead_keys;
mod detail_pane;
mod divider_and_wheel;
mod doorway;
mod evil_probe_retry;
mod graph_toggles;
mod hint_coverage;
mod job_ids;
mod jobs_and_refresh;
mod opening;
mod pointer;
mod prelude;
mod prelude_reads;
mod purity;
mod quit_focus_resize;
mod resolver_screens;
mod screens;
mod search;
mod sweep;
mod sweep_boxes;
mod sweep_entries;
mod sweep_screens;
mod sweep_states;
mod tab_strip_geometry;
mod walk;
mod walk_sort;
mod working_tree_row;
