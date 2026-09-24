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
use graft_git::status::UnmergedHow;
use graft_term::{SanitizedText, sanitize_line, sanitize_path_bytes};
use graft_ui::conflict_resolver::{
    Arrangement, BlockNumbers, BlockState, BlockView, DocView, FileListView, FileRow, FileState,
    Layout, Placement, RegionView, ResolverAreas, SideNames, TakeView, layout, placement,
    resolver_areas,
};
use ratatui::layout::Rect;

#[cfg(test)]
mod tests;
mod whole_file;

#[cfg(test)]
pub(crate) use tests::drawn_for_tests;
use whole_file::{done_card, whole_file_view};

use crate::update::AppState;
use crate::update::detail::Loading;
use crate::update::resolver::{
    DoneShape, RowState, file_cursor, listed_files, side_names, start_over_refusal,
};

/// Everything the resolver draws for the file it has open, laid out for a
/// document `columns` wide, or `None` when it is closed or still waiting
/// for its opening read.
#[must_use]
pub fn resolver_view(state: &AppState, columns: u16) -> Option<DocView> {
    let open = state.resolver.as_ref()?;
    if let Some(whole) = open.whole_file.as_ref() {
        return Some(whole_file_view(state, open, whole, columns));
    }
    if let Some(done) = open.done.as_ref()
        && open.file.is_none()
    {
        return Some(done_card(state, open, &done.shape));
    }
    let file = open.file.as_ref()?;
    let blocks = file.blocks();
    let restart = start_over_refusal(state).is_none();
    let identity = match open.done.as_ref().map(|done| &done.shape) {
        Some(DoneShape::Text { unstaged } | DoneShape::Binary { unstaged }) => Identity::Done {
            unstaged: *unstaged,
            restart,
        },
        Some(DoneShape::Removed | DoneShape::Unreadable { .. }) => Identity::Done {
            unstaged: false,
            restart,
        },
        None => Identity::Blocks {
            open: file.open(),
            of: blocks.len(),
        },
    };
    Some(DocView {
        title: sanitize_line(&resolver_title(&open.path, identity)),
        files: file_list_view(state),
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

/// The open document, laid out in the arrangement `placed` belongs to at
/// `columns` (the document's width) — the same rows the widget will draw,
/// so a key that scrolls and a click that lands agree with the picture by
/// construction (`graft_ui::conflict_resolver::row_at`). Both `placed` and
/// `columns` come from [`resolver_split`], the draw's as much as the
/// mouse's: the frame's width is measured once, there (m6-conflicts/14 S5).
#[must_use]
pub fn resolver_layout(state: &AppState, placed: Placement, columns: u16) -> Option<Layout> {
    resolver_view(state, columns).map(|doc| layout(&doc, arrangement(state, placed), columns))
}

/// The arrangement the open file is drawn in: `placed`'s, except that a
/// done file is always one card (m6-conflicts/25). Its sides are gone — the
/// three panes would draw the file on disk under `yours`, `result` and
/// `theirs` alike, claiming it for a side that may have deleted it. The
/// list keeps its column.
#[must_use]
pub fn arrangement(state: &AppState, placed: Placement) -> Arrangement {
    if state
        .resolver
        .as_ref()
        .is_some_and(|open| open.done.is_some())
    {
        Arrangement::Card
    } else {
        placed.arrangement()
    }
}

/// What the file identity row says after the path.
#[derive(Clone, Copy, Debug)]
pub enum Identity<'a> {
    /// A document: its blocks, and how many are still open.
    Blocks {
        /// Blocks still open.
        open: usize,
        /// Blocks in the file.
        of: usize,
    },
    /// A whole-file card: what happened to the file on each side.
    Whole(&'a str),
    /// A done file, read-only (m6-conflicts/25); `unstaged` when the file
    /// on disk is not what was staged.
    Done {
        /// The status shows a working-tree change for it.
        unstaged: bool,
        /// `d` opens the box (`update::resolver::start_over_refusal`,
        /// m6-conflicts/28): the row offers it only then.
        restart: bool,
    },
}

/// The file identity row (m6-conflicts/13): `notes.txt · 2 conflicts · 1
/// open`, or `gone.txt · modified on yours · main, deleted on theirs ·
/// feature`. The widget draws its own `▸` before it; where the file is in
/// the list is the list's to say.
#[must_use]
pub fn resolver_title(path: &Path, identity: Identity<'_>) -> String {
    let name = path.display();
    match identity {
        Identity::Blocks { open, of } => {
            let noun = if of == 1 { "conflict" } else { "conflicts" };
            format!("{name} · {of} {noun} · {open} open")
        }
        Identity::Whole(how) => format!("{name} · {how}"),
        Identity::Done { unstaged, restart } => {
            let offer = if restart { " · d starts it over" } else { "" };
            let edited = if unstaged {
                " · edited since done"
            } else {
                ""
            };
            format!("{name} · done ✓{offer}{edited}")
        }
    }
}

/// What happened to a whole-file shape's file on each side, each side by
/// its one name: `modified on yours · main, deleted on theirs · feature`.
#[must_use]
pub fn how_words(how: UnmergedHow, binary: bool, names: &SideNames) -> String {
    let (ours, theirs) = (names.ours.as_str(), names.theirs.as_str());
    let words = match how {
        UnmergedHow::BothModified => format!("modified on {ours}, modified on {theirs}"),
        UnmergedHow::BothAdded => format!("added on {ours}, added on {theirs}"),
        UnmergedHow::BothDeleted => format!("deleted on {ours}, deleted on {theirs}"),
        UnmergedHow::AddedByUs => format!("added on {ours}"),
        UnmergedHow::AddedByThem => format!("added on {theirs}"),
        UnmergedHow::DeletedByUs => format!("deleted on {ours}, modified on {theirs}"),
        UnmergedHow::DeletedByThem => format!("modified on {ours}, deleted on {theirs}"),
    };
    if binary {
        format!("binary · {words}")
    } else {
        words
    }
}

/// The file list, where it goes, and the resolver's area split between it
/// and the document, for a frame of `state.size` with `banner_rows` above
/// the resolver and its own bar below — the same split
/// `view::frame::draw_resolver` draws with, for the keys and the mouse.
#[must_use]
pub fn resolver_split(
    state: &AppState,
    banner_rows: u16,
) -> (FileListView, Placement, ResolverAreas) {
    let files = file_list_view(state);
    let placed = placement(&files, state.size.columns);
    let area = Rect {
        x: 0,
        y: banner_rows,
        width: state.size.columns,
        height: state
            .size
            .rows
            .saturating_sub(banner_rows)
            // The hint bar / status line the resolver draws under itself.
            .saturating_sub(1),
    };
    let split = resolver_areas(area, placed);
    (files, placed, split)
}

/// The resolver's file list, sanitised: [`listed_files`] in the widget's
/// terms. The draw and the mouse both build it here.
#[must_use]
pub fn file_list_view(state: &AppState) -> FileListView {
    let Some(open) = state.resolver.as_ref() else {
        return FileListView::default();
    };
    let names = side_names(state);
    FileListView {
        rows: listed_files(state)
            .into_iter()
            .map(|file| FileRow {
                name: sanitize_line(&file.path.display().to_string()),
                state: match file.state {
                    RowState::Open { open, of } => FileState::Open { open, of },
                    RowState::AllTaken => FileState::AllTaken,
                    RowState::Done => FileState::Done,
                    RowState::WholeFile { how } => FileState::WholeFile {
                        how: sanitize_line(&how_words(how, false, &names)),
                    },
                    RowState::Binary => FileState::Binary,
                    RowState::Unread => FileState::Unread,
                },
            })
            .collect(),
        cursor: file_cursor(state),
        unavailable: matches!(open.files, Loading::Failed(_)),
    }
}

/// The document's regions, in file order, with every byte sanitised. Each
/// region carries its first line in every projection (ADR-0043 §5): the
/// card numbers plain lines by the result, the three panes each pane by its
/// own.
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
                    numbers,
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
