//! `update` tests: the shared fixtures other modules name by path
//! (`crate::update::tests::key` etc.), re-exported by the parent. Moved out of
//! `update/tests.rs` (m8/21b-7, move-only); the screens are in `fixtures/screens.rs`.

use super::*;

pub(crate) mod reveal;
mod screens;
pub(crate) use screens::{
    commit_pane, operation_live_as, resolver_card_on_screen, resolver_on_screen,
};

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

pub(crate) fn key(code: KeyCode) -> AppEvent {
    AppEvent::Core(Event::Key(KeyPress::plain(code)))
}

/// A walk in flight whose window holds a plain commit (row 0) and a
/// merge (row 1) folding a branch behind it, the cursor on row `cursor`.
pub(crate) fn merge_on_screen(id: JobId, cursor: usize) -> (AppState, Oid) {
    let commit = |byte: u8, parents: Vec<Oid>| Row {
        drawn_parents: parents.clone(),
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
        // The merge folds a branch behind it, as most merges into a trunk do.
        fold: [
            crate::history::RowFold::default(),
            crate::history::RowFold {
                folds: Some(true),
                inside: None,
            },
        ]
        .into(),
        sought: None,
    };
    let (mut state, _) = update(
        &awaiting(id),
        AppEvent::Progress(id, Progress::History(HistoryUpdate::Window(window))),
    );
    state.history.as_mut().expect("a walk").cursor = cursor;
    // Its count already known: a count asked at rest (ease-pass 06) would
    // ride along on every event. `update::fold_count`'s tests clear it.
    state
        .fold_counts
        .insert(merge_id, crate::update::fold_count::FoldCount::Counted(1));
    // Refs read, no stash: the evil probe waits for them (m6/66).
    state.refs = Some(Arc::new(crate::refs::RefBadges::default()));
    (state, merge_id)
}

/// The view the last `ShowRows` asks for.
pub(crate) fn asked_view(commands: &[Command]) -> View {
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

pub(crate) fn core(event: Event) -> AppEvent {
    AppEvent::Core(event)
}

pub(super) fn facts(path: &str, branch: &str) -> RepositoryFacts {
    RepositoryFacts {
        path: PathBuf::from(path),
        bare: false,
        head: Head::Branch(branch.as_bytes().to_vec()),
        head_commit: None,
        mailmap: mergewright_git::mailmap::Mailmap::default(),
        commit_graph: crate::repository::CommitGraphFact::Present { commits: 1 },
    }
}

/// Job `id`'s answer to the first request, with `n` rows laid out. The
/// rows themselves do not matter here, only whether they land, so they are
/// copies of one row.
pub(crate) fn rows(id: JobId, n: usize) -> AppEvent {
    let row = Row {
        drawn_parents: Vec::new(),
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
            fold: [].into(),
            sought: None,
        })),
    )
}

/// The state of an application with a walk in flight.
pub(crate) fn awaiting(id: JobId) -> AppState {
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
pub(crate) fn progress(state: &AppState) -> (Phase, usize) {
    let history = state.history.as_ref().expect("a walk was asked for");
    (history.phase.clone(), history.rows)
}

pub(crate) fn request(id: u64, wanted: Wanted) -> RowRequest {
    // What a walk starts in: merges folded by default (ease-pass 06).
    RowRequest::new(id, wanted, View::folded())
}

pub(crate) fn sized(columns: u16, rows: u16) -> AppEvent {
    core(Event::Resize { columns, rows })
}

/// Job `id`'s answer to request `request`: `span` rows, `laid_out` so far.
pub(crate) fn rows_for(id: JobId, request: u64, span: Range<usize>, laid_out: usize) -> AppEvent {
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
            fold: [].into(),
            sought: None,
        })),
    )
}

/// A provisional walk and the topological sort it asked for, started as
/// job `sort`.
pub(crate) fn sorting(walk: JobId, sort: JobId) -> AppState {
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

pub(crate) fn order() -> std::sync::Arc<[mergewright_core::commit_handle::CommitHandle]> {
    vec![mergewright_core::commit_handle::CommitHandle(7)].into()
}

/// [`order`] as a sort's answer, from a sort nobody waited for.
pub(crate) fn sorted_order() -> SortedOrder {
    SortedOrder {
        order: order(),
        took: std::time::Duration::ZERO,
    }
}
