//! The resolver's file list (m6-conflicts/13): which rows, in which state,
//! where the cursor is, what a click does, and where the last file lands.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use graft_core::event::{Event, KeyCode, Modifiers, MouseButton, MouseInput, MouseKind};
use graft_core::job::JobId;
use graft_git::state::InProgress;
use graft_git::status::{StatusEntry, UnmergedHow, WorkingTreeStatus};
use graft_ops::conflict::ConflictSet;
use graft_ui::conflict_resolver::{HEADER_ROWS, Placement, file_at};

use super::{WALK, resolver_on_screen, sticky};
use crate::jobs::conflict::StartOver;
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
        submodule: None,
    }
}

pub(super) fn status(entries: &[(&str, bool)]) -> Arc<WorkingTreeStatus> {
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
pub(super) fn listing(paths: &[&str], unmerged: &[&str]) -> AppState {
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
pub(super) fn click(state: &AppState, column: u16, row: u16) -> (AppState, Vec<Command>) {
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
        // In git's path order (m6-conflicts/25).
        vec![
            ("bin.png".to_owned(), RowState::Binary),
            (
                "card.txt".to_owned(),
                RowState::WholeFile {
                    how: UnmergedHow::DeletedByThem
                }
            ),
            (
                "conflicted.rs".to_owned(),
                RowState::Open { open: 2, of: 2 }
            ),
            ("done.txt".to_owned(), RowState::Done),
            ("half.rs".to_owned(), RowState::Open { open: 1, of: 2 }),
            ("never.rs".to_owned(), RowState::Unread),
            ("taken.rs".to_owned(), RowState::AllTaken),
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
            start_over: crate::jobs::conflict::StartOver::Cannot {
                why: graft_ops::operation::Refusal::NotMarkerShape {
                    path: std::path::PathBuf::from("gone.txt"),
                    how: UnmergedHow::DeletedByUs,
                }
                .explain(&crate::update::op_keys::OpKeys(
                    graft_ui::keymap::Keymap::shipped(),
                )),
            },
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
    // Sorted, `conflicted.rs` is first.
    assert_eq!(unknown[0].1, RowState::Open { open: 2, of: 2 });
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
    // Sorted: `a.rs`, `b.rs`, `conflicted.rs`.
    assert_eq!(file_cursor(&state), 2);
    let reread = JobId::new(7_001);
    state.resolver.as_mut().expect("open").set_job = Some(reread);
    // The list stays while the re-read is out.
    assert_eq!(listed_files(&state).len(), 3);
    let (state, _) = update(
        &state,
        AppEvent::Produced(
            reread,
            Product::ConflictSet(Ok(Arc::new(ConflictSet {
                paths: ["b.rs", "a.rs", "conflicted.rs", "0.rs"]
                    .iter()
                    .map(PathBuf::from)
                    .collect(),
                recorded: true,
            }))),
        ),
    );
    // `0.rs` sorts in above it.
    assert_eq!(file_cursor(&state), 3, "the cursor moved with its path");
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
        let (_, placed, split) =
            crate::view::resolver::resolver_split(&state, super::super::banner_rows(&state));
        let laid = crate::view::resolver::resolver_layout(&state, placed, split.doc.width)
            .expect("a document");
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

/// The resolver stays on the last file (m6-conflicts/31, P8): the status
/// line says `<file> done ✓ · every clash is solved · y reviews the merge`. Nothing
/// opens on its own (ease-pass/05b dropped m6-conflicts/60's owed open).
#[test]
fn after_the_last_file_of_a_merge_the_status_says_every_file_done() {
    let state = last_file_lands(Some(InProgress::Merge));
    assert_eq!(super::open_path(&state), Path::new("conflicted.rs"));
    assert!(!state.banner_focus);
    let open = state.resolver.as_ref().expect("open");
    assert_eq!(open.notice, None, "no status read out: nothing to check");
    assert_eq!(
        state.status,
        Status::Done {
            what: "conflicted.rs done".to_owned(),
            also: None,
            keys: Some("every clash is solved · y reviews the merge".to_owned()),
        }
    );
}

/// Rewritten for m6-conflicts/60 (was
/// `after_the_last_file_of_a_rebase_the_notice_says_c_continues`): no
/// summary is owed — `c` writes — so the status line names the key.
#[test]
fn after_the_last_file_of_a_rebase_the_status_says_c_continues() {
    for (kind, keys) in [
        (
            InProgress::Rebase,
            "every file done · c continues the rebase",
        ),
        (
            InProgress::CherryPick,
            "every file done · c continues the cherry-pick",
        ),
        (
            InProgress::Revert,
            "every file done · c continues the revert",
        ),
    ] {
        let state = last_file_lands(Some(kind));
        assert_eq!(super::open_path(&state), Path::new("conflicted.rs"));
        assert!(!state.banner_focus, "{kind:?}");
        let open = state.resolver.as_ref().expect("open");
        assert_eq!(open.notice, None, "{kind:?}");
        assert_eq!(
            state.status,
            Status::Done {
                what: "conflicted.rs done".to_owned(),
                also: None,
                keys: Some(keys.to_owned()),
            },
            "{kind:?}"
        );
    }
}

#[test]
fn after_the_last_stash_file_the_status_says_every_file_done() {
    let state = last_file_lands(None);
    assert!(state.resolver.is_none());
    assert!(!state.banner_focus, "no banner: back on the graph");
    assert_eq!(state.status, Status::done("every file done"));
}

/// A file done, another left: the resolver lands on it, says so on its
/// notice row, and asks for the set again.
#[test]
fn a_done_file_lands_on_the_next_one_not_done() {
    let mut state = listing(
        &["a.txt", "conflicted.rs", "done.txt", "e.txt"],
        &["a.txt", "conflicted.rs", "e.txt"],
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
        Path::new("e.txt"),
        "done.txt skipped"
    );
    let open = state.resolver.as_ref().expect("open");
    assert_eq!(
        open.notice,
        Some(("conflicted.rs done ✓ · ▸ e.txt".to_owned(), false))
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

/// Review m6-amend-13, Major 1: `d`'s box is up over the resolver, and a
/// click lands on another file's row. The box owns the input, as it owns
/// every key: the click does nothing. Before, it closed the resolver
/// (`reopen` took it before `open_refusal` refused) and left the box up
/// over the graph.
#[test]
fn a_list_click_under_the_box_leaves_both_as_they_were() {
    let state = listing(
        &["conflicted.rs", "later.rs"],
        &["conflicted.rs", "later.rs"],
    );
    let (x, y) = cell_of(&state, 1);
    let (boxed, _) = super::press(&state, graft_core::event::KeyCode::Char('d'));
    assert!(boxed.confirm.is_some(), "d put its box up");
    let (after, commands) = click(&boxed, x, y);
    assert!(commands.is_empty(), "{commands:?}");
    assert_eq!(after.resolver, boxed.resolver, "the resolver is as it was");
    assert_eq!(after.confirm, boxed.confirm, "the box is as it was");
    assert_eq!(after.status, boxed.status, "nothing said");

    // The switch itself, reached any other way, refuses without closing:
    // `reopen` decides before it takes the resolver.
    let mut switched = boxed.clone();
    let mut commands = Vec::new();
    let answered =
        crate::update::resolver::select_file(&mut switched, &mut commands, 1, &mut || {
            JobId::new(9_900)
        });
    assert!(answered);
    assert!(commands.is_empty(), "{commands:?}");
    assert_eq!(switched.resolver, boxed.resolver, "still on conflicted.rs");
    assert_eq!(switched.confirm, boxed.confirm);
    assert_eq!(sticky(&switched).0, "finish the box first");
}

// ------------------------------------ m6-conflicts/25: stable order, done open

#[test]
fn the_list_is_in_gits_byte_order() {
    let paths = ["a/b", "a.txt", "B.txt", "a.b"];
    let state = listing(&paths, &paths);
    let order: Vec<String> = states(&state).into_iter().map(|(path, _)| path).collect();
    // Byte-wise: `B` (0x42) before `a`, `.` (0x2e) before `/` (0x2f) — so
    // `a.b` and `a.txt` before `a/b`, which `Path`'s own order would not do.
    assert_eq!(
        order,
        ["B.txt", "a.b", "a.txt", "a/b", "conflicted.rs"].map(str::to_owned)
    );
}

#[test]
fn a_file_keeps_its_row_when_it_is_done() {
    let three = ["a.rs", "conflicted.rs", "z.rs"];
    let mut state = listing(&three, &three);
    let before: Vec<String> = states(&state).into_iter().map(|(path, _)| path).collect();
    assert_eq!(file_cursor(&state), 1);
    // The middle file is staged: the status stops calling it unmerged, and a
    // set re-read lands in ops' order — unmerged first, so it comes last.
    state.working_tree = Some(status(&[
        ("a.rs", true),
        ("conflicted.rs", false),
        ("z.rs", true),
    ]));
    let reread = JobId::new(7_200);
    state.resolver.as_mut().expect("open").set_job = Some(reread);
    let (state, _) = update(
        &state,
        AppEvent::Produced(
            reread,
            Product::ConflictSet(Ok(Arc::new(ConflictSet {
                paths: ["a.rs", "z.rs", "conflicted.rs"]
                    .iter()
                    .map(PathBuf::from)
                    .collect(),
                recorded: true,
            }))),
        ),
    );
    let after: Vec<(String, RowState)> = states(&state);
    assert_eq!(
        after
            .iter()
            .map(|(path, _)| path.clone())
            .collect::<Vec<_>>(),
        before
    );
    assert_eq!(after[1].1, RowState::Done);
    assert_eq!(file_cursor(&state), 1, "the cursor did not move");
}

#[test]
fn extra_paths_sort_into_place() {
    // A `Ready` set of `b.rs` and `d.rs`; `c.rs` unmerged but not in it; the
    // open `conflicted.rs` in neither.
    let mut state = listing(&["b.rs", "d.rs"], &["b.rs", "d.rs"]);
    state.working_tree = Some(status(&[("b.rs", true), ("c.rs", true), ("d.rs", true)]));
    let order: Vec<String> = states(&state).into_iter().map(|(path, _)| path).collect();
    assert_eq!(
        order,
        ["b.rs", "c.rs", "conflicted.rs", "d.rs"].map(str::to_owned)
    );
}

/// `conflicted.rs` open, `gone.txt` done at row 1: a click on it, and the
/// read the click queued answered with `answer`.
fn open_gone(
    answer: Result<crate::jobs::conflict::Opening, crate::jobs::conflict::OpenFailure>,
) -> AppState {
    let state = listing(&["conflicted.rs", "gone.txt"], &["conflicted.rs"]);
    assert_eq!(states(&state)[1].1, RowState::Done);
    let (x, y) = cell_of(&state, 1);
    let (state, commands) = click(&state, x, y);
    let read = commands
        .iter()
        .find_map(|command| match command {
            Command::OpenConflict(id, path) if path == Path::new("gone.txt") => Some(*id),
            _ => None,
        })
        .unwrap_or_else(|| panic!("a done row opens: {commands:?}"));
    let (state, _) = update(
        &state,
        AppEvent::Produced(read, Product::ConflictOpened(answer)),
    );
    state
}

fn done_text(unstaged: bool, text: &str) -> crate::jobs::conflict::Opening {
    let file = graft_git::conflict::parse(text.as_bytes()).expect("plain text parses");
    crate::jobs::conflict::Opening::Done(
        Arc::new(graft_ops::conflict::DoneFile::Text { file, unstaged }),
        StartOver::Can {
            kind: graft_ops::operation::RestartKind::Markers,
        },
    )
}

/// The resolver's identity row, as drawn: the line starting `▸ gone.txt ·`.
fn identity_row(state: &AppState) -> String {
    let drawn = super::frame_text(state);
    drawn
        .lines()
        .find(|line| line.trim_start().starts_with("▸ gone.txt ·"))
        .map_or_else(
            || panic!("no identity row:\n{drawn}"),
            |line| line.trim().to_owned(),
        )
}

#[test]
fn a_click_on_a_done_file_opens_it_read_only() {
    let state = open_gone(Ok(done_text(false, "settled\n")));
    let open = state.resolver.as_ref().expect("the resolver stays open");
    assert_eq!(open.path, Path::new("gone.txt"));
    assert_eq!(
        open.done.as_ref().map(|done| &done.shape),
        Some(&crate::update::resolver::DoneShape::Text { unstaged: false })
    );
    assert_eq!(
        identity_row(&state),
        "▸ gone.txt · done ✓ · d starts it over"
    );
    assert!(super::frame_text(&state).contains("settled"));

    let edited = open_gone(Ok(done_text(true, "settled, then edited\n")));
    assert!(
        identity_row(&edited).ends_with("· edited since done"),
        "{}",
        identity_row(&edited)
    );
}

#[test]
fn a_done_card_for_removed_binary_and_unreadable() {
    use graft_ops::conflict::DoneFile;
    for (answer, words) in [
        (
            Ok(crate::jobs::conflict::Opening::Done(
                Arc::new(DoneFile::Removed),
                StartOver::Can {
                    kind: graft_ops::operation::RestartKind::Markers,
                },
            )),
            "done ✓ · removed",
        ),
        (
            Ok(crate::jobs::conflict::Opening::Done(
                Arc::new(DoneFile::Binary { unstaged: false }),
                StartOver::Can {
                    kind: graft_ops::operation::RestartKind::Markers,
                },
            )),
            "done ✓ · binary",
        ),
        (
            Err(crate::jobs::conflict::OpenFailure {
                words: "the file is 9 bytes, which is more than the resolver handles".to_owned(),
                not_unmerged: false,
                done: Some(StartOver::Can {
                    kind: graft_ops::operation::RestartKind::Markers,
                }),
            }),
            "done ✓ · could not read: the file is 9 bytes",
        ),
    ] {
        let state = open_gone(answer);
        let open = state.resolver.as_ref().expect("the resolver stays open");
        assert_eq!(open.path, Path::new("gone.txt"));
        assert!(open.done.is_some());
        let drawn = super::frame_text(&state);
        assert!(drawn.contains(words), "{words}:\n{drawn}");
        assert!(drawn.contains("d starts it over"), "{drawn}");
    }
}

#[test]
fn an_unmerged_open_failure_still_closes() {
    let state = listing(
        &["conflicted.rs", "later.rs"],
        &["conflicted.rs", "later.rs"],
    );
    let (x, y) = cell_of(&state, 1);
    let (state, commands) = click(&state, x, y);
    let read = commands
        .iter()
        .find_map(|command| match command {
            Command::OpenConflict(id, _) => Some(*id),
            _ => None,
        })
        .expect("the read is queued");
    let (state, _) = update(
        &state,
        AppEvent::Produced(
            read,
            Product::ConflictOpened(Err(crate::jobs::conflict::OpenFailure {
                words: "could not read the file: denied".to_owned(),
                not_unmerged: false,
                done: None,
            })),
        ),
    );
    assert!(
        state.resolver.is_none(),
        "the unmerged route closes as ever"
    );
    assert_eq!(
        sticky(&state),
        // m8/10j ruling 4: the close lands him on the graph, where `e`
        // edits nothing and `M` is the Map — so no key is named.
        ("could not read the file: denied", None)
    );
}

#[test]
fn a_done_file_refuses_in_words() {
    // Long enough that `PgDn` has somewhere to scroll.
    let long = (0..200).fold(String::new(), |mut text, n| {
        text.push_str(&format!("line {n}\n"));
        text
    });
    let state = open_gone(Ok(done_text(false, &long)));
    for code in [
        KeyCode::Char('o'),
        KeyCode::Char('t'),
        KeyCode::Char('b'),
        KeyCode::Enter,
        KeyCode::Char('B'),
        KeyCode::Char('M'),
        KeyCode::Char('O'),
        KeyCode::Char('T'),
    ] {
        let (after, commands) = super::press(&state, code);
        assert!(commands.is_empty(), "{code:?}: {commands:?}");
        assert_eq!(
            sticky(&after),
            ("gone.txt is done · d starts it over", None),
            "{code:?}"
        );
        assert_eq!(after.resolver, state.resolver, "{code:?}");
    }
    // `j` steps files since m6-conflicts/17; `PgDn` is the scroll now.
    let (after, _) = super::press(&state, KeyCode::PageDown);
    // A page is the document window's height less one row of overlap
    // (`scroll::scroll_key`): 41 in this fixture's window.
    assert_eq!(
        after.resolver.as_ref().expect("open").top,
        41,
        "PgDn scrolls"
    );
}

#[test]
fn d_on_a_done_file_starts_it_over() {
    let state = open_gone(Ok(done_text(false, "settled\n")));
    let (state, _) = super::press(&state, KeyCode::Char('d'));
    assert!(state.confirm.is_some(), "today's box");
    let (state, _) = super::press(&state, KeyCode::Right);
    let (mut state, commands) = super::press(&state, KeyCode::Enter);
    assert!(
        commands.iter().any(|command| matches!(
            command,
            Command::Operate(_, op, ..)
                if matches!(**op, graft_ops::operation::Operation::RestartConflict(_))
        )),
        "{commands:?}"
    );
    let restart = state
        .resolver
        .as_ref()
        .and_then(|open| open.restarting)
        .expect("the restart is the resolver's");
    // `operate::operated` clears the running write before the resolver
    // hears of it; this calls the landing directly, so it clears it too.
    state.operating = None;
    let mut commands = Vec::new();
    let mut ids = 7_300;
    crate::update::resolver::restart_landed(&mut state, &mut commands, restart, &mut || {
        ids += 1;
        JobId::new(ids)
    });
    let read = commands
        .iter()
        .find_map(|command| match command {
            Command::OpenConflict(id, path) if path == Path::new("gone.txt") => Some(*id),
            _ => None,
        })
        .unwrap_or_else(|| panic!("the file is read again: {commands:?}"));
    let file = graft_git::conflict::parse(b"<<<<<<< ours\na\n=======\nb\n>>>>>>> theirs\n")
        .expect("markers parse");
    let (state, _) = update(
        &state,
        AppEvent::Produced(
            read,
            Product::ConflictOpened(Ok(crate::jobs::conflict::Opening::Unmerged(Arc::new(
                crate::jobs::conflict::OpenedFile {
                    file: Some(file),
                    how: UnmergedHow::BothModified,
                    binary: false,
                    on_disk: None,
                    unreadable: None,
                    start_over: None,
                },
            )))),
        ),
    );
    let open = state.resolver.as_ref().expect("open");
    assert_eq!(open.done, None);
    assert_eq!(open.file.as_ref().map(|file| file.open()), Some(1));
    assert_eq!(open.cursor_block, Some(0), "its open block is shown");
}

#[test]
fn an_unmerged_document_resolved_outside_still_closes() {
    // The resolver on a conflicted document; `e`, and the editor (or a
    // mergetool) resolves and stages it. The re-read now routes to the done
    // side — and that is the old `not_unmerged` answer, words and all
    // (TESTING.md's M5 mergetool check), not a done card.
    let state = resolver_on_screen(WALK);
    let path = state.resolver.as_ref().expect("open").path.clone();
    let (state, _) = super::press(&state, KeyCode::Char('e'));
    let (state, commands) = update(
        &state,
        AppEvent::Produced(JobId::new(7_400), Product::HandedOver(Ok(()))),
    );
    let read = commands
        .iter()
        .find_map(|command| match command {
            Command::OpenConflict(id, _) => Some(*id),
            _ => None,
        })
        .expect("the hand-back re-reads");
    let (state, _) = update(
        &state,
        AppEvent::Produced(
            read,
            Product::ConflictOpened(Ok(done_text(false, "settled\n"))),
        ),
    );
    assert!(state.resolver.is_none(), "{:?}", state.resolver);
    assert!(
        matches!(&state.status, Status::Done { what: line, .. } if *line == format!("{} resolved outside", path.display())),
        "{:?}",
        state.status
    );
}

#[test]
fn a_done_file_handed_back_stays_open() {
    let state = open_gone(Ok(done_text(false, "settled\n")));
    let (state, commands) = super::press(&state, KeyCode::Char('e'));
    assert!(
        commands
            .iter()
            .any(|command| matches!(command, Command::OpenExternalEditor { .. })),
        "{commands:?}"
    );
    let (state, commands) = update(
        &state,
        AppEvent::Produced(JobId::new(7_400), Product::HandedOver(Ok(()))),
    );
    let read = commands
        .iter()
        .find_map(|command| match command {
            Command::OpenConflict(id, _) => Some(*id),
            _ => None,
        })
        .expect("the hand-back re-reads");
    let (state, _) = update(
        &state,
        AppEvent::Produced(
            read,
            Product::ConflictOpened(Ok(done_text(true, "edited\n"))),
        ),
    );
    let open = state.resolver.as_ref().expect("the resolver is open");
    assert_eq!(
        open.done.as_ref().map(|done| &done.shape),
        Some(&crate::update::resolver::DoneShape::Text { unstaged: true })
    );
}

mod start_over;
