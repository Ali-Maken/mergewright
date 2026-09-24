//! The resolver's file list (m6-conflicts/13): which rows, in which state,
//! where the cursor is, what a click does, and where the last file lands.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use graft_core::event::{Event, Modifiers, MouseButton, MouseInput, MouseKind};
use graft_core::job::JobId;
use graft_git::state::InProgress;
use graft_git::status::{StatusEntry, UnmergedHow, WorkingTreeStatus};
use graft_ops::conflict::ConflictSet;
use graft_ui::conflict_resolver::{HEADER_ROWS, Placement, file_at};

use super::{WALK, resolver_on_screen, sticky};
use crate::update::detail::Loading;
use crate::update::resolver::{
    ListedFile, RowState, WholeFileShape, file_cursor, listed_files, resolved_advance,
};
use crate::update::{AppEvent, AppState, Command, Product, Status, update};

fn entry(path: &str, unmerged: bool) -> StatusEntry {
    StatusEntry {
        path: path.into(),
        original_path: None,
        staged: None,
        unstaged: Some(graft_git::status::Change::Modified),
        untracked: false,
        unmerged,
        unmerged_how: unmerged.then_some(UnmergedHow::BothModified),
        submodule: false,
    }
}

fn status(entries: &[(&str, bool)]) -> Arc<WorkingTreeStatus> {
    Arc::new(WorkingTreeStatus {
        entries: entries
            .iter()
            .map(|(path, unmerged)| entry(path, *unmerged))
            .collect(),
    })
}

fn set(paths: &[&str]) -> Loading<Arc<ConflictSet>> {
    Loading::Ready(Arc::new(ConflictSet {
        paths: paths.iter().map(PathBuf::from).collect(),
        recorded: true,
    }))
}

fn merging(state: &mut AppState) {
    state.in_progress = Some(crate::update::resolver::fixtures::operation(
        InProgress::Merge,
        None,
    ));
}

fn states(state: &AppState) -> Vec<(String, RowState)> {
    listed_files(state)
        .into_iter()
        .map(|ListedFile { path, state }| (path.display().to_string(), state))
        .collect()
}

/// The resolver on `conflicted.rs` (two open blocks) in a merge whose set
/// is `paths`, `unmerged` of them still unmerged.
fn listing(paths: &[&str], unmerged: &[&str]) -> AppState {
    let mut state = resolver_on_screen(WALK);
    merging(&mut state);
    let entries: Vec<(&str, bool)> = paths
        .iter()
        .map(|path| (*path, unmerged.contains(path)))
        .collect();
    state.working_tree = Some(status(&entries));
    let open = state.resolver.as_mut().expect("open");
    open.files = set(paths);
    state
}

/// A left click at `column`, `row`, and the commands it sent — less the
/// graph's merge probe, which the fixture's selected merge asks for on any
/// first event and which is nothing to do with the resolver.
fn click(state: &AppState, column: u16, row: u16) -> (AppState, Vec<Command>) {
    let (state, commands) = update(
        state,
        AppEvent::Core(Event::Mouse(MouseInput {
            kind: MouseKind::Down(MouseButton::Left),
            column,
            row,
            modifiers: Modifiers::NONE,
        })),
    );
    let commands = commands
        .into_iter()
        .filter(|command| !matches!(command, Command::EvilProbe(..)))
        .collect();
    (state, commands)
}

/// The screen cell where the list draws file `index`.
fn cell_of(state: &AppState, index: usize) -> (u16, u16) {
    let (files, placed, split) =
        crate::view::resolver::resolver_split(state, super::super::banner_rows(state));
    for dy in 0..split.list.height {
        for dx in 0..split.list.width {
            if file_at(&files, placed, split.list.width, split.list.height, dx, dy) == Some(index) {
                return (split.list.x + dx, split.list.y + dy);
            }
        }
    }
    panic!("file {index} is not drawn: {files:?}")
}

#[test]
fn row_states_follow_the_document_and_the_status() {
    let mut state = listing(
        &[
            "done.txt",
            "conflicted.rs",
            "card.txt",
            "bin.png",
            "half.rs",
            "taken.rs",
            "never.rs",
        ],
        &[
            "conflicted.rs",
            "card.txt",
            "bin.png",
            "half.rs",
            "taken.rs",
            "never.rs",
        ],
    );
    let open = state.resolver.as_mut().expect("open");
    // Visited and closed while this resolver was open.
    open.seen.insert(
        "card.txt".into(),
        RowState::WholeFile {
            how: UnmergedHow::DeletedByThem,
        },
    );
    open.seen.insert("bin.png".into(), RowState::Binary);
    open.seen
        .insert("half.rs".into(), RowState::Open { open: 1, of: 2 });
    open.seen.insert("taken.rs".into(), RowState::AllTaken);
    assert_eq!(
        states(&state),
        vec![
            ("done.txt".to_owned(), RowState::Done),
            (
                "conflicted.rs".to_owned(),
                RowState::Open { open: 2, of: 2 }
            ),
            (
                "card.txt".to_owned(),
                RowState::WholeFile {
                    how: UnmergedHow::DeletedByThem
                }
            ),
            ("bin.png".to_owned(), RowState::Binary),
            ("half.rs".to_owned(), RowState::Open { open: 1, of: 2 }),
            ("taken.rs".to_owned(), RowState::AllTaken),
            ("never.rs".to_owned(), RowState::Unread),
        ]
    );
}

/// The open file's own row follows its document: every block taken; a
/// card; a binary card.
#[test]
fn the_open_files_row_follows_its_document() {
    let state = listing(&["conflicted.rs"], &["conflicted.rs"]);
    let mut all_taken = state.clone();
    let open = all_taken.resolver.as_mut().expect("open");
    let mut file = (**open.file.as_ref().expect("a document")).clone();
    for block in 0..2 {
        file.take(block, graft_git::conflict::Take::Ours)
            .expect("an open block takes");
    }
    open.file = Some(Arc::new(file));
    assert_eq!(states(&all_taken)[0].1, RowState::AllTaken);
    for (binary, want) in [
        (
            false,
            RowState::WholeFile {
                how: UnmergedHow::DeletedByUs,
            },
        ),
        (true, RowState::Binary),
    ] {
        let mut card = state.clone();
        let open = card.resolver.as_mut().expect("open");
        open.file = None;
        open.whole_file = Some(WholeFileShape {
            how: UnmergedHow::DeletedByUs,
            binary,
            on_disk: None,
        });
        assert_eq!(states(&card)[0].1, want, "binary {binary}");
    }
}

/// Before the status arrives nothing is known about the files not open.
#[test]
fn before_the_status_only_the_open_file_is_known() {
    let mut state = listing(
        &["done.txt", "conflicted.rs", "never.rs"],
        &["conflicted.rs"],
    );
    state.working_tree = None;
    let unknown = states(&state);
    assert_eq!(unknown[1].1, RowState::Open { open: 2, of: 2 });
    for (path, row) in unknown.iter().filter(|(path, _)| path != "conflicted.rs") {
        assert_eq!(*row, RowState::Unread, "{path}");
    }
}

/// A take landing records the file's state, so leaving it keeps its count.
#[test]
fn a_file_left_half_done_keeps_its_count() {
    let state = listing(
        &["conflicted.rs", "later.rs"],
        &["conflicted.rs", "later.rs"],
    );
    let (state, _) = super::press(&state, graft_core::event::KeyCode::Char('o'));
    let job = state
        .resolver
        .as_ref()
        .and_then(|open| open.job)
        .expect("the take is in flight");
    let file = state
        .resolver
        .as_ref()
        .and_then(|open| open.file.as_ref())
        .map(|file| (**file).clone())
        .expect("a document");
    let mut taken = file;
    taken
        .take(0, graft_git::conflict::Take::Ours)
        .expect("block 0 takes");
    let (state, _) = update(
        &state,
        AppEvent::Produced(job, Product::Taken(Ok(Arc::new(taken)))),
    );
    let (later_x, later_y) = cell_of(&state, 1);
    let (state, _) = click(&state, later_x, later_y);
    assert_eq!(super::open_path(&state), Path::new("later.rs"));
    assert_eq!(
        states(&state)[0],
        (
            "conflicted.rs".to_owned(),
            RowState::Open { open: 1, of: 2 }
        )
    );
}

#[test]
fn the_cursor_stays_on_its_path_across_a_reread() {
    let mut state = listing(
        &["a.rs", "conflicted.rs", "b.rs"],
        &["a.rs", "conflicted.rs", "b.rs"],
    );
    assert_eq!(file_cursor(&state), 1);
    let reread = JobId::new(7_001);
    state.resolver.as_mut().expect("open").set_job = Some(reread);
    // The list stays while the re-read is out.
    assert_eq!(listed_files(&state).len(), 3);
    let (state, _) = update(
        &state,
        AppEvent::Produced(
            reread,
            Product::ConflictSet(Ok(Arc::new(ConflictSet {
                paths: ["b.rs", "a.rs", "conflicted.rs"]
                    .iter()
                    .map(PathBuf::from)
                    .collect(),
                recorded: true,
            }))),
        ),
    );
    assert_eq!(file_cursor(&state), 2, "the cursor moved with its path");
    let view = crate::view::resolver::file_list_view(&state);
    assert_eq!(view.rows[view.cursor].name.as_str(), "conflicted.rs");

    // An answer to a read that is no longer the newest is dropped.
    let (state, _) = update(
        &state,
        AppEvent::Produced(
            JobId::new(6_999),
            Product::ConflictSet(Ok(Arc::new(ConflictSet {
                paths: vec!["zzz.rs".into()],
                recorded: true,
            }))),
        ),
    );
    assert!(
        !states(&state).iter().any(|(path, _)| path == "zzz.rs"),
        "a stale answer replaced the list"
    );
}

#[test]
fn a_click_on_the_document_still_picks_its_block() {
    let long: Vec<String> = (0..11)
        .map(|i| format!("src/module_{i:02}/handler.rs"))
        .collect();
    let mut many: Vec<&str> = vec!["conflicted.rs"];
    many.extend(long.iter().map(String::as_str));
    for (paths, rows) in [(vec!["conflicted.rs", "later.rs"], 1u16), (many, 3u16)] {
        let state = listing(&paths, &paths);
        let files = crate::view::resolver::file_list_view(&state);
        assert_eq!(
            graft_ui::conflict_resolver::placement(&files, state.size.columns),
            Placement::Strip { rows },
            "{} files",
            paths.len()
        );
        let laid =
            crate::view::resolver::resolver_layout(&state, state.size.columns).expect("a document");
        let header = u16::try_from(laid.block_row[1]).expect("a small document");
        // Banner, strip, its rule, the title and notice rows, then the
        // document from its first row.
        let row = 1 + rows + 1 + HEADER_ROWS + header;
        let (after, commands) = click(&state, 10, row);
        assert_eq!(
            after.resolver.as_ref().and_then(|open| open.cursor_block),
            Some(1),
            "{rows}-row strip: the click on row {row} missed block 1's header"
        );
        assert!(commands.is_empty(), "{commands:?}");
        // The rule row under the strip is nobody's.
        let (after, commands) = click(&state, 10, 1 + rows);
        assert_eq!(after.resolver, state.resolver, "the rule row did something");
        assert!(commands.is_empty());
    }
}

#[test]
fn a_click_on_a_file_opens_it() {
    let state = listing(
        &["conflicted.rs", "later.rs"],
        &["conflicted.rs", "later.rs"],
    );
    let (x, y) = cell_of(&state, 1);
    let (after, commands) = click(&state, x, y);
    assert_eq!(super::open_path(&after), Path::new("later.rs"));
    assert!(
        commands
            .iter()
            .any(|command| matches!(command, Command::OpenConflict(_, path) if path == Path::new("later.rs"))),
        "{commands:?}"
    );
    let open = after.resolver.as_ref().expect("open");
    assert_eq!(
        open.files,
        state.resolver.as_ref().expect("open").files,
        "the list is kept"
    );
    assert_eq!(open.cursor_block, None, "the new file's read is still out");
    assert_eq!(open.top, 0);
    assert_eq!(
        open.seen.get(Path::new("conflicted.rs")),
        Some(&RowState::Open { open: 2, of: 2 }),
        "the file left keeps its count"
    );
    assert_eq!(file_cursor(&after), 1, "the list's ▸ moved");

    // The open file's own row: answered, nothing sent.
    let (x, y) = cell_of(&state, 0);
    let (again, commands) = click(&state, x, y);
    assert!(commands.is_empty(), "{commands:?}");
    assert_eq!(again.resolver, state.resolver);
}

#[test]
fn a_click_mid_take_is_refused_in_words() {
    let state = listing(
        &["conflicted.rs", "later.rs"],
        &["conflicted.rs", "later.rs"],
    );
    let (x, y) = cell_of(&state, 1);
    for (reading, base, words) in [
        (Some(false), false, "still writing the last take"),
        (Some(true), false, "still reading the file"),
        (None, true, "still loading the base"),
    ] {
        let mut busy = state.clone();
        let open = busy.resolver.as_mut().expect("open");
        if let Some(reading) = reading {
            open.job = Some(JobId::new(8_000));
            open.reading = reading;
        }
        if base {
            open.base_job = Some(JobId::new(8_001));
        }
        let (after, commands) = click(&busy, x, y);
        assert_eq!(super::open_path(&after), Path::new("conflicted.rs"));
        assert!(commands.is_empty(), "{commands:?}");
        assert_eq!(sticky(&after).0, words);
    }
}

/// `⏎`'s stage landing on the last file not done.
fn last_file_lands(kind: Option<InProgress>) -> AppState {
    let mut state = listing(&["gone.txt", "conflicted.rs"], &["conflicted.rs"]);
    state.in_progress = kind.map(|kind| crate::update::resolver::fixtures::operation(kind, None));
    state.banner_focus = false;
    let staged = JobId::new(4_500);
    state.resolver.as_mut().expect("open").staging = Some(staged);
    let mut commands = Vec::new();
    let mut ids = 4_500;
    resolved_advance(&mut state, &mut commands, staged, &mut || {
        ids += 1;
        JobId::new(ids)
    });
    state
}

#[test]
fn after_the_last_file_the_banner_says_c_finishes() {
    let state = last_file_lands(Some(InProgress::Merge));
    assert!(state.resolver.is_none());
    assert!(state.banner_focus);
    assert_eq!(
        state.status,
        Status::Done("every file done ✓ · c finishes the merge".to_owned())
    );
}

#[test]
fn after_the_last_file_a_rebase_says_c_continues() {
    for (kind, words) in [
        (
            InProgress::Rebase,
            "every file done ✓ · c continues the rebase",
        ),
        (
            InProgress::CherryPick,
            "every file done ✓ · c continues the cherry-pick",
        ),
        (
            InProgress::Revert,
            "every file done ✓ · c continues the revert",
        ),
    ] {
        let state = last_file_lands(Some(kind));
        assert!(state.resolver.is_none(), "{kind:?}");
        assert!(state.banner_focus, "{kind:?}");
        assert_eq!(state.status, Status::Done(words.to_owned()));
    }
}

#[test]
fn after_the_last_stash_file_the_status_says_every_file_done() {
    let state = last_file_lands(None);
    assert!(state.resolver.is_none());
    assert!(!state.banner_focus, "no banner: back on the graph");
    assert_eq!(state.status, Status::Done("every file done ✓".to_owned()));
}

/// A file done, another left: the resolver lands on it, says so on its
/// notice row, and asks for the set again.
#[test]
fn a_done_file_lands_on_the_next_one_not_done() {
    let mut state = listing(
        &["a.txt", "conflicted.rs", "done.txt", "c.txt"],
        &["a.txt", "conflicted.rs", "c.txt"],
    );
    let staged = JobId::new(4_600);
    state.resolver.as_mut().expect("open").staging = Some(staged);
    let mut commands = Vec::new();
    let mut ids = 4_600;
    resolved_advance(&mut state, &mut commands, staged, &mut || {
        ids += 1;
        JobId::new(ids)
    });
    assert_eq!(
        super::open_path(&state),
        Path::new("c.txt"),
        "done.txt skipped"
    );
    let open = state.resolver.as_ref().expect("open");
    assert_eq!(
        open.notice,
        Some(("conflicted.rs done ✓ · ▸ c.txt".to_owned(), false))
    );
    assert!(
        commands
            .iter()
            .any(|command| matches!(command, Command::ReadConflictSet(_))),
        "{commands:?}"
    );
}

#[test]
fn an_unreadable_set_says_so() {
    let mut state = listing(
        &["conflicted.rs", "later.rs"],
        &["conflicted.rs", "later.rs"],
    );
    let read = JobId::new(7_100);
    let open = state.resolver.as_mut().expect("open");
    open.files = Loading::Pending(read);
    open.set_job = Some(read);
    let (state, _) = update(
        &state,
        AppEvent::Produced(read, Product::ConflictSet(Err("git broke".to_owned()))),
    );
    assert_eq!(
        states(&state),
        vec![
            (
                "conflicted.rs".to_owned(),
                RowState::Open { open: 2, of: 2 }
            ),
            ("later.rs".to_owned(), RowState::Unread),
        ],
        "rows from the status"
    );
    let drawn = super::frame_text(&state);
    assert!(
        drawn.contains("files: git's list unavailable · the unmerged ones below"),
        "{drawn}"
    );
}
