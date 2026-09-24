//! The resolver view's tests, and the frame helper `update`'s tests draw
//! through.

/// The whole frame as `view` draws it at `state.size`, one string per row
/// — for `update`'s tests, which must not name the terminal library
/// themselves (`update::tests::update_cannot_reach_the_world`).
pub(crate) fn drawn_for_tests(state: &AppState) -> Vec<String> {
    let (columns, rows) = (state.size.columns, state.size.rows);
    let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(columns, rows))
        .expect("TestBackend reports its size");
    terminal
        .draw(|frame| crate::view::view(&crate::session::for_tests::session(), state, frame))
        .expect("TestBackend cannot fail to draw");
    let buffer = terminal.backend().buffer();
    (0..rows)
        .map(|y| (0..columns).map(|x| buffer[(x, y)].symbol()).collect())
        .collect()
}

use std::sync::Arc;

use super::{Identity, resolver_title, resolver_view};
use crate::update::AppState;
use crate::update::resolver::fixtures::{commit, oid, operation, refs};
use crate::update::resolver::{OpenedFrom, ResolverState, WholeFileShape};
use graft_git::state::InProgress;
use ratatui::Terminal;
use ratatui::backend::TestBackend;

/// The whole frame as `view` draws it, one string per row — with
/// `state.size` the terminal's, as the app keeps it (seeded at startup,
/// updated on every resize): the resolver's split reads it.
fn frame(state: &AppState, columns: u16, rows: u16) -> String {
    let state = &AppState {
        size: crate::update::Size { columns, rows },
        ..state.clone()
    };
    let mut terminal =
        Terminal::new(TestBackend::new(columns, rows)).expect("TestBackend reports its size");
    terminal
        .draw(|frame| crate::view::view(&crate::session::for_tests::session(), state, frame))
        .expect("TestBackend cannot fail to draw");
    let buffer = terminal.backend().buffer();
    (0..buffer.area.height)
        .map(|y| {
            (0..buffer.area.width)
                .map(|x| buffer[(x, y)].symbol())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Git's own words that must never reach the screen as a side's name.
const RAW: [&str; 6] = [
    "HEAD",
    "OURS",
    "THEIRS",
    "ours =",
    "(yours)",
    "Stashed changes",
];

/// A resolver open on `bytes`, at `path`.
fn open_on(bytes: &[u8], path: &str) -> AppState {
    let file = graft_git::conflict::parse(bytes).expect("a conflict file");
    AppState {
        working_tree: Some(Arc::new(graft_git::status::WorkingTreeStatus {
            entries: vec![graft_git::status::StatusEntry {
                path: path.into(),
                original_path: None,
                staged: None,
                unstaged: Some(graft_git::status::Change::Modified),
                untracked: false,
                unmerged: true,
                unmerged_how: None,
                submodule: false,
            }],
        })),
        resolver: Some(ResolverState {
            path: path.into(),
            opened_from: OpenedFrom::Pane,
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
        }),
        ..AppState::default()
    }
}

/// Repo bytes reach the screen through here and nowhere else, so an
/// escape sequence in a conflicted file cannot repaint the terminal and
/// a newline in a path cannot forge a row.
#[test]
fn resolver_view_sanitises() {
    let bytes =
        b"plain\n<<<<<<< \x1b[31mHEAD\nours \x1b[2Jline\n=======\ntheirs\n>>>>>>> feature\n";
    let state = open_on(bytes, "src/with\na newline.rs");
    let view = resolver_view(&state, 120).expect("a document");

    assert!(
        !view.title.as_str().contains('\n') && !view.title.as_str().contains('\u{1b}'),
        "the title carries a control character: {:?}",
        view.title.as_str()
    );
    let drawn: Vec<String> = view
        .regions
        .iter()
        .flat_map(|region| match region {
            graft_ui::conflict_resolver::RegionView::Plain { lines, .. } => {
                lines.iter().map(|line| line.as_str().to_owned()).collect()
            }
            graft_ui::conflict_resolver::RegionView::Block(block) => block
                .ours
                .iter()
                .chain(block.theirs.iter())
                .map(|line| line.as_str().to_owned())
                .collect::<Vec<_>>(),
            graft_ui::conflict_resolver::RegionView::WholeFile { .. } => Vec::new(),
        })
        .collect();
    assert!(
        !drawn.iter().any(|line| line.contains('\u{1b}')),
        "an escape sequence reached the screen: {drawn:?}"
    );
    assert!(
        drawn.iter().any(|line| line.contains("line")),
        "the line itself was dropped, not cleaned: {drawn:?}"
    );
}

/// Two one-line blocks git wrote for a stopped merge, `HEAD`'s and
/// git's raw labels included — which must not be drawn.
const TWO_BLOCKS: &[u8] = b"one\n<<<<<<< HEAD\ntwo from main\n=======\nTWO from feature\n>>>>>>> feature\nthree\n<<<<<<< Updated upstream\nfour\n=======\nFOUR\n>>>>>>> Stashed changes\n";

fn assert_no_raw_label(drawn: &str) {
    for raw in RAW {
        assert!(!drawn.contains(raw), "{raw:?} was drawn:\n{drawn}");
    }
}

/// ADR-0043 §4: each side has one name, the same on every card, and
/// git's own labels never reach the frame.
#[test]
fn one_name_per_side() {
    let feature = commit(oid(3), "Add OAuth provider");
    let cases = [
        (
            InProgress::Merge,
            Some(feature.clone()),
            "yours · main",
            "theirs · feature",
        ),
        (
            InProgress::Rebase,
            Some(feature.clone()),
            "upstream · main",
            "yours · 0303030 Add OAuth provider",
        ),
        (
            InProgress::CherryPick,
            Some(feature.clone()),
            "yours · main",
            "picked · 0303030 Add OAuth provider",
        ),
    ];
    for (kind, theirs, ours_name, theirs_name) in cases {
        let mut state = open_on(TWO_BLOCKS, "notes.txt");
        state.refs = Some(refs(false));
        state.in_progress = Some(operation(kind, theirs));
        for columns in [80, 120] {
            let drawn = frame(&state, columns, 30);
            assert!(drawn.contains(ours_name), "{kind:?} {columns}:\n{drawn}");
            assert!(drawn.contains(theirs_name), "{kind:?} {columns}:\n{drawn}");
            assert_no_raw_label(&drawn);
        }
    }

    // Stash pop: the resolver open with no operation in progress. Below
    // 100 columns the labels stack, one side per row: `stashed` must
    // then be a row of its own, not a tail on ours' row.
    let mut state = open_on(TWO_BLOCKS, "notes.txt");
    state.refs = Some(refs(false));
    for columns in [80, 120] {
        let drawn = frame(&state, columns, 30);
        assert!(drawn.contains("yours · main"), "{columns}:\n{drawn}");
        assert!(!drawn.contains("stashed ·"), "{columns}:\n{drawn}");
        assert_no_raw_label(&drawn);
        if columns == 80 {
            assert!(
                drawn.lines().any(|row| row.trim_end().ends_with("stashed")
                    && !row.contains("yours")
                    && row.trim_start().starts_with("stashed")),
                "`stashed` is not on a label row of its own at 80:\n{drawn}"
            );
        } else {
            assert!(drawn.contains("│stashed"), "{columns}:\n{drawn}");
        }
    }
}

/// A head file naming a commit git cannot read: the operation still
/// draws, and theirs' label says why it has no name rather than hiding
/// the failure (review m6-amend-10-22, Minor 2).
#[test]
fn an_unreadable_theirs_is_named_as_such() {
    let mut state = open_on(TWO_BLOCKS, "notes.txt");
    state.refs = Some(refs(false));
    let mut merge = operation(InProgress::Merge, None);
    merge.theirs = Err(graft_git::state::IncomingError {
        file: "MERGE_HEAD",
        reason: "bad object".to_owned(),
    });
    state.in_progress = Some(merge);
    let drawn = frame(&state, 120, 30);
    assert!(drawn.contains("yours · main"), "{drawn}");
    assert!(
        drawn.contains("theirs · could not read MERGE_HEAD"),
        "{drawn}"
    );
}

/// Decision 2: HEAD with no branch on it reads as its short id.
#[test]
fn detached_head_names_the_sha() {
    let mut state = open_on(TWO_BLOCKS, "notes.txt");
    state.refs = Some(refs(true));
    state.in_progress = Some(operation(InProgress::Merge, Some(commit(oid(3), "x"))));
    let drawn = frame(&state, 120, 30);
    assert!(drawn.contains("yours · 0909090"), "{drawn}");
    assert!(drawn.contains("theirs · feature"), "{drawn}");
}

/// A `DU` file has no markers to read a label from; its card still names
/// theirs by its one name.
#[test]
fn whole_file_card_names_theirs_without_markers() {
    let mut state = open_on(b"x\n", "gone.txt");
    state.refs = Some(refs(false));
    state.in_progress = Some(operation(InProgress::Merge, Some(commit(oid(3), "x"))));
    if let Some(open) = state.resolver.as_mut() {
        open.file = None;
        open.whole_file = Some(WholeFileShape {
            how: graft_git::status::UnmergedHow::DeletedByUs,
            binary: false,
            on_disk: None,
            start_over: crate::jobs::conflict::StartOver::Cannot {
                why: graft_ops::operation::Refusal::NotMarkerShape {
                    path: std::path::PathBuf::from("gone.txt"),
                    how: graft_git::status::UnmergedHow::DeletedByUs,
                }
                .explain(),
            },
        });
    }
    let drawn = frame(&state, 120, 30);
    assert!(drawn.contains("t keeps theirs · feature"), "{drawn}");
    assert_no_raw_label(&drawn);
}

/// The identity row names the file and how much of it is left; where
/// it is in the list is the list's to say (m6-conflicts/13).
#[test]
fn the_title_says_what_is_left() {
    let path = std::path::Path::new("notes.txt");
    assert_eq!(
        resolver_title(path, Identity::Blocks { open: 1, of: 2 }),
        "notes.txt · 2 conflicts · 1 open"
    );
    assert_eq!(
        resolver_title(path, Identity::Blocks { open: 1, of: 1 }),
        "notes.txt · 1 conflict · 1 open"
    );
}

/// A whole-file or binary card has no marker blocks, so a count clause
/// reads as nothing left to do on a file that still needs `o` or `t`
/// (acceptance finding F8). The card's row says what each side did.
#[test]
fn a_card_title_says_what_each_side_did() {
    for binary in [false, true] {
        let mut state = open_on(b"a\n<<<<<<< HEAD\nx\n=======\ny\n>>>>>>> f\n", "gone.txt");
        state.refs = Some(refs(false));
        state.in_progress = Some(operation(InProgress::Merge, Some(commit(oid(3), "x"))));
        let open = state.resolver.as_mut().expect("open");
        open.file = None;
        open.cursor_block = None;
        open.whole_file = Some(crate::update::resolver::WholeFileShape {
            how: graft_git::status::UnmergedHow::DeletedByThem,
            binary,
            on_disk: None,
            start_over: crate::jobs::conflict::StartOver::Cannot {
                why: graft_ops::operation::Refusal::NotMarkerShape {
                    path: std::path::PathBuf::from("gone.txt"),
                    how: graft_git::status::UnmergedHow::DeletedByThem,
                }
                .explain(),
            },
        });
        let card = resolver_view(&state, 120).expect("a card");
        let words = "gone.txt · modified on yours · main, deleted on theirs · feature";
        if binary {
            assert_eq!(
                card.title.as_str(),
                "gone.txt · binary · modified on yours · main, deleted on theirs · feature"
            );
        } else {
            assert_eq!(card.title.as_str(), words);
        }
        assert!(!card.title.as_str().contains("0 of 0"), "binary {binary}");
    }
}
