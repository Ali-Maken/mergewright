//! Building the conflict resolver's [`DocView`] (m6-conflicts/04).
//!
//! **The one sanitisation site for a conflicted file.** Every byte here —
//! the file's own lines, the base's label, the path in the title — came
//! off disk or out of git, so every one goes through `mergewright_term`
//! (`docs/design/08-architecture.md` §8.2). `mergewright_ui::conflict_resolver`
//! takes `SanitizedText` and nothing else, which is what makes that a rule
//! the compiler keeps rather than a habit.

use std::path::Path;

use mergewright_git::conflict::{Block, ConflictFile, Projection, Region, Take};
use mergewright_git::status::UnmergedHow;
use mergewright_term::{SanitizedText, sanitize_line, sanitize_path_bytes};
use mergewright_ui::conflict_resolver::{
    Arrangement, BlockNumbers, BlockState, BlockView, DocView, FileListView, Layout, Placement,
    RegionView, ResolverAreas, SideNames, TakeView, layout, placement, resolver_areas,
};
use ratatui::layout::Rect;

mod file_list;
mod solved;
#[cfg(test)]
mod tests;
mod whole_file;

pub use file_list::file_list_view;
use whole_file::{done_card, whole_file_view};

use crate::update::AppState;
use crate::update::resolver::{DoneShape, side_names, start_over_refusal};

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
    let restart = start_over_refusal(state)
        .is_none()
        .then(|| start_over_words(&state.keymap))
        .flatten();
    let restart = restart.as_deref();
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
        regions: {
            let mut regions = regions(file);
            // A done file is read-only: its card says so, not this line.
            if open.done.is_none() {
                regions.extend(solved::solved_line(state, &open.path, file));
            }
            regions
        },
        cursor_block: open.cursor_block,
        base_shown: open.base_shown,
        notice: open
            .notice
            .as_ref()
            .map(|(text, error)| (sanitize_line(text), *error)),
        finish: finish_row(state),
    })
}

/// The finish row (m6-conflicts/60): every file done, the summary not open,
/// and `c` would act — so the row names `c` as the next action, in the
/// operation's words. `None` while the status read is out (the notice says
/// it is checking), while a problem notice stands (it outranks the row),
/// and wherever `c` would refuse or has nothing to carry on (a stash pop, a
/// bisect).
#[must_use]
pub(crate) fn finish_row(state: &AppState) -> Option<mergewright_ui::conflict_resolver::FinishRow> {
    use mergewright_git::state::InProgress;
    let open = state.resolver.as_ref()?;
    if state.finish.is_some()
        || open.notice.as_ref().is_some_and(|(_, problem)| *problem)
        // The status read is out (`status_read::owed`): the notice says
        // it is checking until it lands.
        || state.status_read.is_some()
        || crate::update::banner::banner_view(state).is_none()
        || crate::update::banner::continue_refusal(state).is_some()
    {
        return None;
    }
    let kind = state.in_progress.as_ref()?.kind;
    // `c` from his keymap (m8/10g); the row is only that key's clause, so
    // an unbound `c` draws no row at all.
    let c = state
        .keymap
        .spell_keys(&[mergewright_ui::keymap::ActionId("resolver.finish")])?;
    let (full, short) = match kind {
        InProgress::Rebase | InProgress::CherryPick | InProgress::Revert => (
            format!("{c} continue the {}", kind.name()),
            format!("{c} continue"),
        ),
        // A merge's way on is the list's last row, `✓ Review the merge
        // y` (ease-pass/11): the notice row names no finish there.
        InProgress::Merge | InProgress::Bisect => return None,
    };
    let row = |keys: &str| sanitize_line(&format!("✓ every file done — {keys}"));
    Some(mergewright_ui::conflict_resolver::FinishRow {
        full: row(&full),
        short: row(&short),
    })
}

/// The open document, laid out in the arrangement `placed` belongs to at
/// `columns` (the document's width) — the same rows the widget will draw,
/// so a key that scrolls and a click that lands agree with the picture by
/// construction (`mergewright_ui::conflict_resolver::row_at`). Both `placed` and
/// `columns` come from [`resolver_split`], the draw's as much as the
/// mouse's: the frame's width is measured once, there (m6-conflicts/14 S5).
#[must_use]
pub fn resolver_layout(state: &AppState, placed: Placement, columns: u16) -> Option<Layout> {
    resolver_view(state, columns).map(|doc| layout(&doc, arrangement(placed), columns))
}

/// The arrangement the open file is drawn in (ease-pass/11, /11b): a strip
/// list leaves one card; a column list leaves the inline document, its open
/// clashes side by side, at every width (three panes retired).
#[must_use]
pub fn arrangement(placed: Placement) -> Arrangement {
    match placed {
        Placement::Strip { .. } => Arrangement::Card,
        Placement::Column { .. } => Arrangement::Inline,
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
        /// `d starts it over`, spelled from his keymap, where `d` opens the
        /// box (`update::resolver::start_over_refusal`, m6-conflicts/28):
        /// the row offers it only then, and never an unbound key (m8/10g).
        restart: Option<&'a str>,
    },
}

/// `d starts it over`, `d` from `keys` (m8/10g); `None` when unbound.
fn start_over_words(keys: &mergewright_ui::keymap::Keymap) -> Option<String> {
    keys.key_verb(
        &[mergewright_ui::keymap::ActionId("resolver.start_over")],
        "starts it over",
    )
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
            let offer = restart.map_or_else(String::new, |words| format!(" \u{b7} {words}"));
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
    let split = resolver_areas(resolver_area(state, banner_rows), placed);
    (files, placed, split)
}

/// The whole resolver's rectangle, list and document together: the frame
/// under the banner, over the resolver's own bar.
#[must_use]
pub fn resolver_area(state: &AppState, banner_rows: u16) -> Rect {
    Rect {
        x: 0,
        y: banner_rows,
        width: state.size.columns,
        height: state
            .size
            .rows
            .saturating_sub(banner_rows)
            // The hint bar / status line the resolver draws under itself.
            .saturating_sub(1),
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
    let side =
        |lines: &[mergewright_git::conflict::Line]| lines.iter().map(text_of).collect::<Vec<_>>();
    BlockView {
        // What the pick wrote, for its `◀ ▶ ◆` rows (ease-pass/11).
        kept: match state {
            BlockState::Open => Vec::new(),
            BlockState::Took(TakeView::Ours) => side(&block.ours),
            BlockState::Took(TakeView::Theirs) => side(&block.theirs),
            BlockState::Took(TakeView::Both) => {
                let mut both = side(&block.ours);
                both.extend(side(&block.theirs));
                both
            }
        },
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
fn text_of(line: &mergewright_git::conflict::Line) -> SanitizedText {
    let raw = line.0.as_slice();
    let content = raw
        .strip_suffix(b"\n")
        .map_or(raw, |cut| cut.strip_suffix(b"\r").unwrap_or(cut));
    sanitize_line(&String::from_utf8_lossy(content))
}

/// `mergewright_git`'s take, as the widget names it.
fn take_view(take: Take) -> TakeView {
    match take {
        Take::Ours => TakeView::Ours,
        Take::Theirs => TakeView::Theirs,
        Take::Both => TakeView::Both,
    }
}
