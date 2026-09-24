//! Building the conflict resolver's [`DocView`] (m6-conflicts/04).
//!
//! **The one sanitisation site for a conflicted file.** Every byte here —
//! the file's own lines, the base's label, the path in the title — came
//! off disk or out of git, so every one goes through `graft_term`
//! (`docs/design/08-architecture.md` §8.2). `graft_ui::conflict_resolver`
//! takes `SanitizedText` and nothing else, which is what makes that a rule
//! the compiler keeps rather than a habit.

use std::path::Path;

use graft_git::conflict::{Block, ConflictFile, Projection, Region, Take};
use graft_term::{SanitizedText, sanitize_line, sanitize_path_bytes};
use graft_ui::conflict_resolver::{
    BlockNumbers, BlockState, BlockView, DocView, Layout, RegionView, SIDE_BY_SIDE_MIN, TakeView,
    layout,
};

use crate::update::AppState;
use crate::update::resolver::{ResolverState, WholeFileShape, side_names, whole_file_keys};

/// Everything the resolver draws for the file it has open, or `None` when
/// it is closed or still waiting for its opening read.
#[must_use]
pub fn resolver_view(state: &AppState, columns: u16) -> Option<DocView> {
    let open = state.resolver.as_ref()?;
    if let Some(whole) = open.whole_file.as_ref() {
        return Some(whole_file_view(state, open, whole, columns));
    }
    let file = open.file.as_ref()?;
    let paths = crate::update::banner::unmerged_paths(state);
    let index = paths
        .iter()
        .position(|path| path == &open.path)
        .map_or(1, |at| at + 1);
    let blocks = file.blocks();
    Some(DocView {
        title: sanitize_line(&resolver_title(
            &open.path,
            index,
            paths.len().max(1),
            Some((file.open(), blocks.len())),
            columns >= SIDE_BY_SIDE_MIN,
        )),
        names: side_names(state),
        regions: regions(file),
        cursor_block: open.cursor_block,
        base_shown: open.base_shown,
        notice: open
            .notice
            .as_ref()
            .map(|(text, error)| (sanitize_line(text), *error)),
    })
}

/// A whole-file shape's own card (m6-conflicts/05): one region, no blocks,
/// no cursor to draw a gutter for.
fn whole_file_view(
    state: &AppState,
    open: &ResolverState,
    whole: &WholeFileShape,
    columns: u16,
) -> DocView {
    let paths = crate::update::banner::unmerged_paths(state);
    let index = paths
        .iter()
        .position(|path| path == &open.path)
        .map_or(1, |at| at + 1);
    DocView {
        title: sanitize_line(&resolver_title(
            &open.path,
            index,
            paths.len().max(1),
            // A card has no marker blocks to count: `0 of 0 conflicts
            // open` would read as nothing left to do (m6-conflicts/07 F8).
            None,
            columns >= SIDE_BY_SIDE_MIN,
        )),
        names: side_names(state),
        regions: vec![RegionView::WholeFile {
            how: sanitize_line(whole.how.words()),
            keys: sanitize_line(&whole_file_keys(
                whole.how,
                whole.binary,
                &side_names(state),
            )),
            lines: whole_file_lines(whole),
            note: whole_file_note(whole),
        }],
        cursor_block: None,
        base_shown: false,
        notice: open
            .notice
            .as_ref()
            .map(|(text, error)| (sanitize_line(text), *error)),
    }
}

/// A whole-file card's plain-text tail: what is on disk, as far as the
/// card read it (`jobs::conflict::glimpse`) — never more than
/// `jobs::conflict::CARD_LINES` lines, so sanitising them per frame is
/// bounded however large the file is. None for a binary file.
fn whole_file_lines(whole: &WholeFileShape) -> Vec<SanitizedText> {
    whole.on_disk.as_ref().map_or_else(Vec::new, |on_disk| {
        on_disk
            .head
            .iter()
            .map(|line| whole_file_line(line))
            .collect()
    })
}

/// The unnumbered line under a card's text: a binary file's size in place
/// of its bytes, or how much of a long file the card left unread.
fn whole_file_note(whole: &WholeFileShape) -> Option<SanitizedText> {
    let on_disk = whole.on_disk.as_ref()?;
    if whole.binary {
        return Some(sanitize_line(&format!(
            "binary · {}",
            size_words(on_disk.size)
        )));
    }
    on_disk.cut.then(|| {
        sanitize_line(&format!(
            "… first {} lines of {} · e opens the whole file",
            on_disk.head.len(),
            size_words(on_disk.size)
        ))
    })
}

/// One line of a whole-file card's plain-text tail: raw bytes off disk,
/// through the same sanitiser as every other byte this widget draws.
fn whole_file_line(line: &[u8]) -> SanitizedText {
    sanitize_line(&String::from_utf8_lossy(line))
}

/// A file size the way a person reads one: `812 B`, `64.0 KB`, `4.2 MB`.
#[must_use]
pub fn size_words(bytes: u64) -> String {
    const UNITS: [&str; 4] = ["KB", "MB", "GB", "TB"];
    if bytes < 1024 {
        return format!("{bytes} B");
    }
    let bytes = u128::from(bytes);
    let mut divisor: u128 = 1024;
    let mut unit = 0;
    while bytes >= divisor * 1024 && unit + 1 < UNITS.len() {
        divisor *= 1024;
        unit += 1;
    }
    // Tenths of a unit, rounded, in integers: no float casts to lint.
    let tenths = (bytes * 10 + divisor / 2) / divisor;
    format!("{}.{} {}", tenths / 10, tenths % 10, UNITS[unit])
}

/// The open document, laid out at `columns` — the same rows the widget will
/// draw, so a key that scrolls and a click that lands agree with the
/// picture by construction (`graft_ui::conflict_resolver::row_at`).
#[must_use]
pub fn resolver_layout(state: &AppState, columns: u16) -> Option<Layout> {
    resolver_view(state, columns).map(|doc| layout(&doc, columns))
}

/// The title row: `src/auth.rs · file 1 of 3 · 2 of 3 conflicts open`, or
/// the short form when the screen is too narrow to carry all three.
/// `blocks` is `(open, total)`, or `None` for a whole-file card, which has
/// no blocks and so no count clause.
#[must_use]
pub fn resolver_title(
    path: &Path,
    file_index: usize,
    files: usize,
    blocks: Option<(usize, usize)>,
    wide: bool,
) -> String {
    let name = path.display();
    match (blocks, wide) {
        (Some((open, total)), true) => {
            format!("{name} · file {file_index} of {files} · {open} of {total} conflicts open")
        }
        (Some((open, total)), false) => format!("{name} · {open}/{total} open"),
        (None, true) => format!("{name} · file {file_index} of {files}"),
        (None, false) => name.to_string(),
    }
}

/// The document's regions, in file order, with every byte sanitised. Each
/// line is numbered by its own projection (ADR-0043 §5): plain lines by the
/// result, an open block's sides by ours and theirs.
fn regions(file: &ConflictFile) -> Vec<RegionView> {
    let at = file.blocks();
    let ours = file.first_lines(Projection::Ours);
    let theirs = file.first_lines(Projection::Theirs);
    let result = file.first_lines(Projection::Result);
    let mut out = Vec::with_capacity(file.regions.len());
    let mut index = 0usize;
    for (r, region) in file.regions.iter().enumerate() {
        let numbers = BlockNumbers {
            ours: ours[r],
            theirs: theirs[r],
            result: result[r],
        };
        let (block, state) = match region {
            Region::Plain(lines) => {
                out.push(RegionView::Plain {
                    first_line: result[r],
                    lines: lines.iter().map(text_of).collect(),
                });
                continue;
            }
            Region::Conflict(block) => (block, BlockState::Open),
            Region::Resolved { block, take } => (block, BlockState::Took(take_view(*take))),
        };
        // `blocks()` has one entry per non-plain region, in order.
        let first_line = at.get(index).map_or(numbers.result, |at| at.first_line);
        out.push(RegionView::Block(block_view(
            block,
            index,
            at.len(),
            first_line,
            numbers,
            state,
        )));
        index += 1;
    }
    out
}

/// One block, drawn.
fn block_view(
    block: &Block,
    index: usize,
    of: usize,
    first_line: usize,
    numbers: BlockNumbers,
    state: BlockState,
) -> BlockView {
    BlockView {
        index,
        of,
        state,
        ours: block.ours.iter().map(text_of).collect(),
        base: block.shown_base().map(|(base, shared)| {
            (
                label(
                    base.label.as_deref(),
                    if shared {
                        "merged common ancestor (shared)"
                    } else {
                        "merged common ancestor"
                    },
                ),
                base.lines.iter().map(text_of).collect(),
            )
        }),
        theirs: block.theirs.iter().map(text_of).collect(),
        first_line,
        numbers,
    }
}

/// A marker's label, or the stand-in for git's bare `<<<<<<<` form.
fn label(raw: Option<&[u8]>, fallback: &str) -> SanitizedText {
    match raw {
        Some(bytes) => sanitize_path_bytes(bytes),
        None => sanitize_line(fallback),
    }
}

/// One line of the file, without the terminator it is stored with: a `\n`
/// or `\r\n` drawn into a cell is a control character, not content.
fn text_of(line: &graft_git::conflict::Line) -> SanitizedText {
    let raw = line.0.as_slice();
    let content = raw
        .strip_suffix(b"\n")
        .map_or(raw, |cut| cut.strip_suffix(b"\r").unwrap_or(cut));
    sanitize_line(&String::from_utf8_lossy(content))
}

/// `graft_git`'s take, as the widget names it.
fn take_view(take: Take) -> TakeView {
    match take {
        Take::Ours => TakeView::Ours,
        Take::Theirs => TakeView::Theirs,
        Take::Both => TakeView::Both,
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::{resolver_title, resolver_view};
    use crate::update::AppState;
    use crate::update::resolver::fixtures::{commit, oid, operation, refs};
    use crate::update::resolver::{OpenedFrom, ResolverState, WholeFileShape};
    use graft_git::state::InProgress;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    /// The whole frame as `view` draws it, one string per row.
    fn frame(state: &AppState, columns: u16, rows: u16) -> String {
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
            });
        }
        let drawn = frame(&state, 120, 30);
        assert!(drawn.contains("t keeps theirs · feature"), "{drawn}");
        assert_no_raw_label(&drawn);
    }

    /// The title is the only place the file's own count is drawn, so it says
    /// where in the list this file is and how much of it is left.
    #[test]
    fn the_title_says_where_you_are() {
        let path = std::path::Path::new("src/auth.rs");
        assert_eq!(
            resolver_title(path, 2, 5, Some((1, 3)), true),
            "src/auth.rs · file 2 of 5 · 1 of 3 conflicts open"
        );
        assert_eq!(
            resolver_title(path, 2, 5, Some((1, 3)), false),
            "src/auth.rs · 1/3 open",
            "the short form is what fits under 100 columns"
        );
    }

    /// A whole-file or binary card has no marker blocks, so a count clause
    /// reads `0 of 0 conflicts open` — "nothing to do" on a file that still
    /// needs `o` or `t` (acceptance finding F8). The card's title drops it.
    #[test]
    fn a_card_title_has_no_count() {
        for binary in [false, true] {
            let mut state = open_on(b"a\n<<<<<<< HEAD\nx\n=======\ny\n>>>>>>> f\n", "gone.txt");
            let open = state.resolver.as_mut().expect("open");
            open.file = None;
            open.cursor_block = None;
            open.whole_file = Some(crate::update::resolver::WholeFileShape {
                how: graft_git::status::UnmergedHow::DeletedByThem,
                binary,
                on_disk: None,
            });
            let wide = resolver_view(&state, 120).expect("a card");
            assert_eq!(
                wide.title.as_str(),
                "gone.txt · file 1 of 1",
                "binary {binary}"
            );
            let narrow = resolver_view(&state, 80).expect("a card");
            assert_eq!(narrow.title.as_str(), "gone.txt", "binary {binary}");
        }
    }
}

#[cfg(test)]
mod size_tests {
    use super::size_words;

    #[test]
    fn sizes_read_the_way_a_person_reads_them() {
        assert_eq!(size_words(812), "812 B");
        assert_eq!(size_words(64 * 1024), "64.0 KB");
        assert_eq!(size_words(4_404_019), "4.2 MB");
        assert_eq!(size_words(3 * 1024 * 1024 * 1024), "3.0 GB");
    }
}
