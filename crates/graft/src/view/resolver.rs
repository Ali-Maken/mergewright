//! Building the conflict resolver's [`DocView`] (m6-conflicts/04).
//!
//! **The one sanitisation site for a conflicted file.** Every byte here —
//! the file's own lines, git's marker labels, the path in the title — came
//! off disk or out of git, so every one goes through `graft_term`
//! (`docs/design/08-architecture.md` §8.2). `graft_ui::conflict_resolver`
//! takes `SanitizedText` and nothing else, which is what makes that a rule
//! the compiler keeps rather than a habit.

use std::path::Path;

use graft_git::conflict::{Block, ConflictFile, Region, Take};
use graft_git::state::{InProgress, InProgressDetail};
use graft_term::{SanitizedText, sanitize_line, sanitize_path_bytes};
use graft_ui::conflict_resolver::{
    BlockState, BlockView, DocView, Layout, RegionView, SIDE_BY_SIDE_MIN, TakeView, layout,
};

use crate::update::AppState;
use crate::update::resolver::{ResolverState, WholeFileShape, whole_file_keys};

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
            file.open(),
            blocks.len(),
            columns >= SIDE_BY_SIDE_MIN,
        )),
        explainer: resolver_explainer(state).as_deref().map(sanitize_line),
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
            0,
            0,
            columns >= SIDE_BY_SIDE_MIN,
        )),
        explainer: resolver_explainer(state).as_deref().map(sanitize_line),
        regions: vec![RegionView::WholeFile {
            how: sanitize_line(whole.how.words()),
            keys: sanitize_line(whole_file_keys(whole.how, whole.binary)),
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
#[must_use]
pub fn resolver_title(
    path: &Path,
    file_index: usize,
    files: usize,
    open: usize,
    total: usize,
    wide: bool,
) -> String {
    let name = path.display();
    if wide {
        format!("{name} · file {file_index} of {files} · {open} of {total} conflicts open")
    } else {
        format!("{name} · {open}/{total} open")
    }
}

/// **Which side is which**, in the words of the operation that made the
/// conflict — the one thing a rebase gets wrong for everybody, because git
/// swaps `ours` and `theirs` round compared with a merge.
///
/// Wider than the ticket's `(&InProgressDetail)`: naming the branch a
/// rebase is landing on means looking it up in the refs, which an
/// `InProgressDetail` does not carry (it has the `onto` commit's id, not a
/// name). No operation is not always "nothing to explain": a conflicted
/// `git stash pop` leaves an unmerged index with no rebase/merge state at
/// all (`KNOWN_GAPS.md` row 229, no `InProgressDetail`), and the resolver
/// can still be open on it — the only way that happens with `in_progress`
/// `None`, so a resolver open with no operation *is* one (m6-conflicts/05,
/// scope 4). `None` only when the resolver itself is closed.
#[must_use]
pub fn resolver_explainer(state: &AppState) -> Option<String> {
    let Some(detail) = state.in_progress.as_ref() else {
        return state
            .resolver
            .is_some()
            .then(|| "ours = your working tree · theirs = the stash".to_owned());
    };
    Some(match detail.kind {
        InProgress::Rebase => format!(
            "ours = {}, what you are rebasing onto · theirs = the commit being replayed",
            onto_name(state, detail)
        ),
        InProgress::Merge => format!(
            "ours = {} · theirs = the branch being merged in",
            current_branch(state).unwrap_or_else(|| "this branch".to_owned())
        ),
        InProgress::CherryPick | InProgress::Revert | InProgress::Bisect => format!(
            "ours = {} · theirs = the commit being applied",
            current_branch(state).unwrap_or_else(|| "this branch".to_owned())
        ),
    })
}

/// What the rebase is landing on, named: a branch on that commit if the
/// refs know one, else its abbreviated id, else the phrase that says what
/// it is without pretending to know which.
fn onto_name(state: &AppState, detail: &InProgressDetail) -> String {
    let Some(onto) = detail.onto else {
        return "the branch you are rebasing onto".to_owned();
    };
    branch_on(state, onto).unwrap_or_else(|| {
        let full = onto.to_string();
        full.chars().take(SHORT_OID).collect()
    })
}

/// Hex digits of a commit id shown when no ref names it — the seven every
/// other line in Graft shows (`update::preview::short`).
const SHORT_OID: usize = 7;

/// The first local branch name on `commit`, if the refs are in.
fn branch_on(state: &AppState, commit: graft_core::oid::Oid) -> Option<String> {
    state
        .refs
        .as_deref()?
        .on(commit)
        .iter()
        .find_map(|label| match label {
            crate::refs::RefLabel::Branch { name, .. } => {
                Some(sanitize_path_bytes(name).as_str().to_owned())
            }
            _ => None,
        })
}

/// The branch `HEAD` is on, if there is one.
fn current_branch(state: &AppState) -> Option<String> {
    let refs = state.refs.as_deref()?;
    refs.on(refs.head()?).iter().find_map(|label| match label {
        crate::refs::RefLabel::Branch {
            name,
            current: true,
            ..
        } => Some(sanitize_path_bytes(name).as_str().to_owned()),
        _ => None,
    })
}

/// The document's regions, in file order, with every byte sanitised.
fn regions(file: &ConflictFile) -> Vec<RegionView> {
    let total = file.blocks().len();
    let mut out = Vec::with_capacity(file.regions.len());
    let mut line = 1usize;
    let mut index = 0usize;
    for region in &file.regions {
        match region {
            Region::Plain(lines) => {
                out.push(RegionView::Plain {
                    first_line: line,
                    lines: lines.iter().map(text_of).collect(),
                });
                line += lines.len();
            }
            Region::Conflict(block) => {
                out.push(RegionView::Block(block_view(
                    block,
                    index,
                    total,
                    line,
                    BlockState::Open,
                )));
                index += 1;
                line += conflict_lines(block);
            }
            Region::Resolved { block, take } => {
                out.push(RegionView::Block(block_view(
                    block,
                    index,
                    total,
                    line,
                    BlockState::Took(take_view(*take)),
                )));
                index += 1;
                line += taken_lines(block, *take);
            }
        }
    }
    out
}

/// One block, drawn.
fn block_view(
    block: &Block,
    index: usize,
    of: usize,
    first_line: usize,
    state: BlockState,
) -> BlockView {
    BlockView {
        index,
        of,
        state,
        ours_label: label(block.ours_label.as_deref(), "ours"),
        theirs_label: label(block.theirs_label.as_deref(), "theirs"),
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

/// How many rendered lines an open block takes, so the next region's first
/// line number is the file's own (`ConflictFile::blocks`' arithmetic, which
/// is not public).
fn conflict_lines(block: &Block) -> usize {
    let base = block.base.as_ref().map_or(0, |base| base.lines.len() + 1);
    3 + block.ours.len() + base + block.theirs.len()
}

/// The same for a block a take has collapsed.
fn taken_lines(block: &Block, take: Take) -> usize {
    match take {
        Take::Ours => block.ours.len(),
        Take::Theirs => block.theirs.len(),
        Take::Both => block.ours.len() + block.theirs.len(),
    }
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

    use super::{resolver_explainer, resolver_title, resolver_view};
    use crate::update::AppState;
    use crate::update::resolver::{OpenedFrom, ResolverState};
    use graft_git::state::{InProgress, InProgressDetail};

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
                    .chain([
                        block.ours_label.as_str().to_owned(),
                        block.theirs_label.as_str().to_owned(),
                    ])
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

    /// A rebase swaps `ours` and `theirs` round compared with a merge, which
    /// is the one thing everybody gets wrong. The explainer says which.
    #[test]
    fn explainer_names_the_sides() {
        let mut state = open_on(b"a\n<<<<<<< HEAD\nx\n=======\ny\n>>>>>>> f\n", "both.txt");
        state.in_progress = Some(InProgressDetail {
            kind: InProgress::Rebase,
            step: None,
            head_name: None,
            onto: None,
        });
        assert_eq!(
            resolver_explainer(&state).as_deref(),
            Some(
                "ours = the branch you are rebasing onto, what you are rebasing onto · \
                 theirs = the commit being replayed"
            )
        );

        // No operation, resolver still open: the only way that happens is a
        // conflicted `git stash pop` (m6-conflicts/05, scope 4), so this is
        // the stash explainer, not "no sides to explain".
        state.in_progress = None;
        assert_eq!(
            resolver_explainer(&state).as_deref(),
            Some("ours = your working tree · theirs = the stash")
        );

        state.resolver = None;
        assert_eq!(
            resolver_explainer(&state),
            None,
            "the resolver is closed, nothing to explain"
        );
    }

    /// The title is the only place the file's own count is drawn, so it says
    /// where in the list this file is and how much of it is left.
    #[test]
    fn the_title_says_where_you_are() {
        let path = std::path::Path::new("src/auth.rs");
        assert_eq!(
            resolver_title(path, 2, 5, 1, 3, true),
            "src/auth.rs · file 2 of 5 · 1 of 3 conflicts open"
        );
        assert_eq!(
            resolver_title(path, 2, 5, 1, 3, false),
            "src/auth.rs · 1/3 open",
            "the short form is what fits under 100 columns"
        );
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
