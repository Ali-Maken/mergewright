//! The detail pane's view model: what a selected commit, a comparison or the
//! working tree shows beside the graph.
//!
//! Types only, plus the file list's cursor and folding. `ui` learns nothing
//! about git here: every string arrives as `SanitizedText`, every path already
//! sanitised, and the app assembles these on a job thread. The widgets that
//! draw them come later (`detail-pane-frame`, `diff-widget`).

use std::ops::Range;
use std::sync::Arc;

use graft_core::mistake_radar::Severity;
use graft_term::sanitize::SanitizedText;

use crate::syntax::SyntaxSpan;

/// What the pane is about.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum Subject {
    /// Nothing selected yet.
    #[default]
    None,
    /// One commit, against its first parent.
    Commit {
        /// Short SHA.
        sha: SanitizedText,
    },
    /// Two commits compared (`x`), since they split (m7-14).
    Compare(CompareHeader),
    /// The working tree against `HEAD`.
    WorkingTree {
        /// Files staged.
        staged: u32,
        /// Files changed, not staged.
        unstaged: u32,
        /// Files git does not track.
        untracked: u32,
    },
}

/// A compare's header (m7-14): the two sides by name and what the read
/// found. Names are the row's first local branch, else its first remote,
/// else its short sha — sanitised like every string here.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompareHeader {
    /// The pinned side (`x`), `B` in the header's words.
    pub base: SanitizedText,
    /// The cursor's side, `C` in the header's words.
    pub head: SanitizedText,
    /// What is known so far.
    pub state: CompareState,
}

/// Where a compare's read stands, and what it found.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CompareState {
    /// The span or the file list is still coming.
    Reading,
    /// The cursor is on the base itself.
    IsBase,
    /// `head` has commits of its own since it left `base`.
    Since {
        /// Commits on `head` since the split.
        commits: usize,
        /// The file list's totals; `None` until it lands.
        tally: Option<FileTally>,
    },
    /// `head` is already in `base`: nothing of its own.
    AlreadyIn,
    /// No common ancestor: the diff is plain two-dot.
    NoSharedHistory,
    /// The read failed, and why.
    Failed(SanitizedText),
}

/// A compare's file totals: binary files count to `files` only.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FileTally {
    /// Files changed.
    pub files: usize,
    /// Lines added.
    pub added: usize,
    /// Lines removed.
    pub removed: usize,
}

/// What a compare's file-list read learnt about the two sides (m7-14):
/// carried on [`FileListView::compare`] so the header can word it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompareSummary {
    /// Listed from the split; `commits` on the head side since it.
    Since {
        /// Where the two split: the diff's old side, for anything that
        /// reads it (a blame of an old-side line starts here).
        split: graft_core::oid::Oid,
        /// Commits reachable from head and not from the split.
        commits: usize,
    },
    /// No shared history: listed two-dot, `base..head`.
    NoSharedHistory,
}

/// The pane's header block (DESIGN.md §1.1).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DetailHeader {
    /// What the pane is about.
    pub subject: Subject,
    /// The subject line.
    pub summary: SanitizedText,
    /// Author name and email.
    pub author: SanitizedText,
    /// Author date, absolute.
    pub author_date: SanitizedText,
    /// Author date, relative ("3 days ago").
    pub relative: SanitizedText,
    /// Shown only when the committer differs from the author.
    pub committer: Option<SanitizedText>,
    /// `Co-authored-by` trailers.
    pub co_authors: Vec<SanitizedText>,
    /// Parent short SHAs.
    pub parents: Vec<SanitizedText>,
    /// What the diff is against, when it is not the obvious first parent.
    pub against: Option<SanitizedText>,
    /// Branches and tags holding the commit.
    pub contained: ContainedIn,
    /// Mistake Radar lines.
    pub radar: Vec<RadarLine>,
}

/// Which branches and tags hold the commit.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum ContainedIn {
    #[default]
    /// Not asked yet.
    NotAsked,
    /// The query is running.
    Checking,
    /// `more` counts names past the ones listed.
    Known {
        /// Local and remote branches.
        branches: Vec<SanitizedText>,
        /// Tags.
        tags: Vec<SanitizedText>,
        /// Names past the ones listed.
        more: usize,
    },
    /// Why the query failed.
    Failed(SanitizedText),
}

/// One Mistake Radar line in the header.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RadarLine {
    /// A finding on this commit.
    Finding {
        /// How loudly it shows.
        severity: Severity,
        /// The finding's title.
        title: SanitizedText,
    },
    /// A merge not yet probed for evil changes.
    EvilUnchecked,
    /// The evil-merge probe is running.
    EvilChecking,
    /// The merge carries no changes of its own.
    EvilClean,
    /// The merge differs from a clean re-merge only in files both sides
    /// changed: a hand-resolved conflict, not a mistake. Shown so the pane
    /// does not read as "nothing happened here" when something did.
    EvilResolved {
        /// How many files the resolution touched.
        files: usize,
    },
    /// The evil-merge probe could not run, and why.
    EvilNotCheckable(SanitizedText),
    /// The force-push probe could not run, and why.
    ForcePushNotCheckable(SanitizedText),
    /// The radar's default-branch context could not be read, and why: no
    /// finding that needs it was checked. Only while the read has failed —
    /// never while it is pending or ready.
    NotChecked(SanitizedText),
}

/// A changed file's status, as the list shows it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FileStatusView {
    /// Added in this change.
    Added,
    /// Deleted in this change.
    Deleted,
    /// Content changed.
    Modified,
    /// Moved, maybe edited.
    Renamed,
    /// Copied from another path.
    Copied,
    /// File type changed (file, symlink, submodule).
    TypeChanged,
    /// In conflict: the porcelain `XY` status pair, already rendered as
    /// words (`both modified`, `deleted by them`, `added by us`, …) — the
    /// CONFLICTS row draws `how` in place of the usual letter and
    /// added/removed counts (m5-history-and-ghost-preview/04).
    Unmerged {
        /// The conflict in words.
        how: SanitizedText,
    },
    /// Not tracked by git.
    Untracked,
}

/// Which of the changes pane's three groups a working-tree row is in
/// (m3-working-tree-undo-and-stash/04). Membership is not a label: it decides
/// which diff the row shows (`Staged` → index against `HEAD`, the other two →
/// working tree against the index) and which way a write goes — a row under
/// `Staged` unstages, anything else stages.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WorkingTreeSection {
    /// Unmerged paths: a conflict from a stopped merge/rebase/cherry-pick.
    /// Drawn first, ahead of every other section, so a conflict is never
    /// mistaken for an ordinary staged/unstaged change
    /// (m5-history-and-ghost-preview/04).
    Unmerged,
    /// In the index, waiting to be committed.
    Staged,
    /// Changed in the working tree, not in the index.
    Unstaged,
    /// Not tracked by git at all.
    Untracked,
}

impl WorkingTreeSection {
    /// The section heading's word, as the pane prints it.
    #[must_use]
    pub fn title(self) -> &'static str {
        match self {
            Self::Unmerged => "CONFLICTS",
            Self::Staged => "STAGED",
            Self::Unstaged => "UNSTAGED",
            Self::Untracked => "UNTRACKED",
        }
    }
}

/// What a row of the file list is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FileRowKind {
    /// A changes-pane group heading (`STAGED (2)`); the files of that group
    /// follow it at a greater depth. `FileRow::name` carries the word.
    Section {
        /// Which group this heads.
        section: WorkingTreeSection,
        /// Files under it.
        count: usize,
    },
    /// The `✉ message` entry: the commit body, `lines` long.
    Message {
        /// Body lines.
        lines: usize,
    },
    /// A directory; its children follow it at a greater depth.
    Dir {
        /// Children hidden.
        collapsed: bool,
    },
    /// A file; line counts are `None` for binary files.
    File {
        /// What happened to it.
        status: FileStatusView,
        /// Lines added.
        added: Option<u32>,
        /// Lines removed.
        removed: Option<u32>,
    },
}

/// One row of the file tree.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileRow {
    /// Nesting depth, 0 at the top.
    pub depth: u8,
    /// The last path component (directories end in `/`).
    pub name: SanitizedText,
    /// What the row is.
    pub kind: FileRowKind,
    /// Index into the git file list; `None` for the message and directories.
    pub entry: Option<u32>,
}

/// The changed files as a folding tree, with its cursor.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct FileListView {
    /// Every row, folded or not, in tree order.
    pub rows: Vec<FileRow>,
    /// Index into `rows`; always a visible row.
    pub cursor: usize,
    /// First row on screen; the widget keeps the cursor in view.
    pub top: usize,
    /// Files changed.
    pub files: u32,
    /// Lines added across all files.
    pub added: u64,
    /// Lines removed across all files.
    pub removed: u64,
    /// The commit body the `✉ message` row opens; empty without that row.
    pub message: Vec<SanitizedText>,
    /// For a compare's list, how the two sides relate (m7-14); `None`
    /// for every other list.
    pub compare: Option<CompareSummary>,
}

impl FileListView {
    /// Rows not hidden inside a collapsed directory, with their index in `rows`.
    pub fn visible(&self) -> impl Iterator<Item = (usize, &FileRow)> {
        let mut hidden_below: Option<u8> = None;
        self.rows.iter().enumerate().filter(move |(_, row)| {
            if let Some(depth) = hidden_below {
                if row.depth > depth {
                    return false;
                }
                hidden_below = None;
            }
            if let FileRowKind::Dir { collapsed: true } = row.kind {
                hidden_below = Some(row.depth);
            }
            true
        })
    }

    /// Fold or unfold the directory at `row`; anything else is left alone.
    /// A cursor left inside the fold moves up to the directory.
    pub fn toggle_dir(&mut self, row: usize) {
        let Some(FileRow {
            kind: FileRowKind::Dir { collapsed },
            ..
        }) = self.rows.get_mut(row)
        else {
            return;
        };
        *collapsed = !*collapsed;
        if !self.visible().any(|(index, _)| index == self.cursor) {
            self.cursor = row;
        }
    }

    /// Move the cursor by `delta` visible rows, clamped to the ends.
    pub fn move_cursor(&mut self, delta: isize) {
        let visible: Vec<usize> = self.visible().map(|(index, _)| index).collect();
        let Some(last) = visible.len().checked_sub(1) else {
            return;
        };
        let here = visible
            .iter()
            .position(|&index| index == self.cursor)
            .unwrap_or(0);
        let target = here.saturating_add_signed(delta).min(last);
        self.cursor = visible[target];
    }

    /// The git file-list index under the cursor, if the cursor is on a file.
    pub fn selected_entry(&self) -> Option<u32> {
        self.rows.get(self.cursor).and_then(|row| row.entry)
    }

    /// Which changes-pane group `row` is in: the nearest
    /// [`FileRowKind::Section`] at or above it. `None` on a list with no
    /// section rows at all — every list but the changes pane's.
    #[must_use]
    pub fn section_at(&self, row: usize) -> Option<WorkingTreeSection> {
        self.rows
            .get(..=row)?
            .iter()
            .rev()
            .find_map(|row| match row.kind {
                FileRowKind::Section { section, .. } => Some(section),
                _ => None,
            })
    }

    /// The group the cursor is in.
    #[must_use]
    pub fn selected_section(&self) -> Option<WorkingTreeSection> {
        self.section_at(self.cursor)
    }

    /// Every git file-list index under the section row at `row`, in row
    /// order. Empty for any other kind of row, so a caller can ask without
    /// checking first.
    #[must_use]
    pub fn section_entries(&self, row: usize) -> Vec<u32> {
        if !matches!(
            self.rows.get(row).map(|r| &r.kind),
            Some(FileRowKind::Section { .. })
        ) {
            return Vec::new();
        }
        self.rows[row + 1..]
            .iter()
            .take_while(|r| !matches!(r.kind, FileRowKind::Section { .. }))
            .filter_map(|r| r.entry)
            .collect()
    }
}

/// What kind of line a diff line is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LineKindView {
    /// Unchanged.
    Context,
    /// Only on the new side.
    Added,
    /// Only on the old side.
    Removed,
    /// The `@@` line opening a hunk.
    HunkHeading,
}

/// One drawable diff line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LineView {
    /// What the line is.
    pub kind: LineKindView,
    /// Line number on the old side.
    pub old_no: Option<u32>,
    /// Line number on the new side.
    pub new_no: Option<u32>,
    /// Sanitised; a CRLF line ends in `␍`.
    pub text: SanitizedText,
    /// Char ranges that changed against the paired line (word tint).
    pub words: Vec<Range<usize>>,
    /// Syntax highlighting over `text`.
    pub syntax: Vec<SyntaxSpan>,
    /// Index of the paired line in the same hunk's `lines`.
    pub pair: Option<u32>,
}

/// One hunk; `lines[0]` is its `HunkHeading`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HunkView {
    /// The text after `@@ … @@`.
    pub heading: SanitizedText,
    /// The lines, heading first.
    pub lines: Vec<LineView>,
}

/// How the diff is laid out.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum DiffMode {
    /// One column, old above new.
    #[default]
    Unified,
    /// Old left, new right.
    SideBySide,
}

/// The diff options as the pane words them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DiffOptionsView {
    /// Context lines around each change.
    pub context: u8,
    /// Whitespace-only changes hidden.
    pub ignore_whitespace: bool,
    /// Changed words tinted inside paired lines.
    pub word_tint: bool,
}

impl Default for DiffOptionsView {
    fn default() -> Self {
        Self {
            context: 3,
            ignore_whitespace: false,
            word_tint: true,
        }
    }
}

/// One file's diff, ready to draw.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiffView {
    /// The file's path.
    pub path: SanitizedText,
    /// Where it came from, for a rename or copy.
    pub old_path: Option<SanitizedText>,
    /// What happened to it.
    pub status: FileStatusView,
    /// The text diff.
    pub hunks: Vec<HunkView>,
    /// Why there is no text diff, for a binary file.
    pub binary: Option<SanitizedText>,
    /// `(shown, total)` lines when the diff was cut short.
    pub truncated: Option<(usize, usize)>,
    /// Not tracked by git: no diff to show.
    pub untracked: bool,
}

/// What a row of the drawn diff body is
/// (m3-working-tree-undo-and-stash/04). The body is a flat list of rows —
/// one heading per hunk, then that hunk's lines — and the diff cursor
/// indexes it. This is the same flattening `detail_pane::diff_body` draws,
/// so a cursor row and a drawn row are always the same row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BodyRow {
    /// A hunk's `@@` heading.
    Heading {
        /// Index into [`DiffView::hunks`].
        hunk: usize,
    },
    /// One content line of a hunk.
    Line {
        /// Index into [`DiffView::hunks`].
        hunk: usize,
        /// Index into that [`HunkView::lines`] — the heading `LineView` at
        /// `lines[0]` is never addressed, so this is always `>= 1`.
        line: usize,
    },
    /// The "… N more lines · L load all" footer.
    Truncation,
}

impl DiffView {
    /// Content lines of one hunk: its `lines` without the `HunkHeading` the
    /// assembler puts at `lines[0]` (which `heading` already stands for).
    fn content_lines(hunk: &HunkView) -> impl Iterator<Item = (usize, &LineView)> {
        hunk.lines
            .iter()
            .enumerate()
            .filter(|(_, line)| line.kind != LineKindView::HunkHeading)
    }

    /// How many rows the body draws: one heading plus its content lines per
    /// hunk, plus the truncation footer when there is one.
    #[must_use]
    pub fn body_len(&self) -> usize {
        let lines: usize = self
            .hunks
            .iter()
            .map(|hunk| 1 + Self::content_lines(hunk).count())
            .sum();
        lines + usize::from(self.truncated.is_some())
    }

    /// What body row `row` is, or `None` past the end.
    #[must_use]
    pub fn body_row(&self, row: usize) -> Option<BodyRow> {
        let mut left = row;
        for (index, hunk) in self.hunks.iter().enumerate() {
            if left == 0 {
                return Some(BodyRow::Heading { hunk: index });
            }
            left -= 1;
            let mut lines = Self::content_lines(hunk);
            let count = Self::content_lines(hunk).count();
            if left < count {
                let (line, _) = lines.nth(left)?;
                return Some(BodyRow::Line { hunk: index, line });
            }
            left -= count;
        }
        if left == 0 && self.truncated.is_some() {
            Some(BodyRow::Truncation)
        } else {
            None
        }
    }
}

/// What the pane's body shows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BodyView {
    /// Nothing to show, and why.
    Empty(SanitizedText),
    /// The commit body.
    Message {
        /// Body lines.
        lines: Vec<SanitizedText>,
    },
    /// A read is running, and what it is.
    Loading(SanitizedText),
    /// A read failed, and why.
    Failed(SanitizedText),
    /// One file's diff.
    Diff(Arc<DiffView>),
}

/// Which region has the keys.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum DetailFocus {
    /// The graph.
    #[default]
    Graph,
    /// The file list.
    Files,
    /// The diff body.
    Diff,
}

/// Everything the detail pane draws.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DetailView {
    /// The header block.
    pub header: DetailHeader,
    /// The file list, once read.
    pub files: Option<Arc<FileListView>>,
    /// The body.
    pub body: BodyView,
    /// First body line on screen.
    pub body_top: usize,
    /// The line cursor's body row, when the body has one: any diff
    /// (m7-power-tools/20b); on the changes pane `Space` and `d` act on the
    /// row under it (m3-working-tree-undo-and-stash/04). `None` with no diff
    /// on show.
    pub body_cursor: Option<usize>,
    /// Which region has the keys.
    pub focus: DetailFocus,
    /// Unified or side by side.
    pub mode: DiffMode,
    /// The diff options in force.
    pub options: DiffOptionsView,
    /// The explain panel, when open.
    pub explain: Option<ExplainView>,
}

/// A Mistake Radar explanation, as the explain panel draws it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExplainView {
    /// The finding's title.
    pub title: SanitizedText,
    /// How loudly it shows.
    pub severity: Severity,
    /// What happened and what to do.
    pub paragraphs: Vec<SanitizedText>,
    /// A small drawing of the shape.
    pub sketch: Vec<SanitizedText>,
}

#[cfg(test)]
mod tests {
    use graft_term::sanitize::sanitize_line;

    use super::*;

    fn row(depth: u8, name: &str, kind: FileRowKind, entry: Option<u32>) -> FileRow {
        FileRow {
            depth,
            name: sanitize_line(name),
            kind,
            entry,
        }
    }

    fn file() -> FileRowKind {
        FileRowKind::File {
            status: FileStatusView::Modified,
            added: Some(1),
            removed: Some(1),
        }
    }

    fn tree() -> FileListView {
        FileListView {
            rows: vec![
                row(0, "a/", FileRowKind::Dir { collapsed: false }, None),
                row(1, "b/", FileRowKind::Dir { collapsed: false }, None),
                row(2, "c.rs", file(), Some(0)),
                row(1, "d.rs", file(), Some(1)),
                row(0, "e.rs", file(), Some(2)),
            ],
            ..FileListView::default()
        }
    }

    fn visible_names(list: &FileListView) -> Vec<&str> {
        list.visible().map(|(_, row)| row.name.as_str()).collect()
    }

    #[test]
    fn folding_a_directory_hides_only_its_children() {
        let mut list = tree();
        list.toggle_dir(1);
        assert_eq!(visible_names(&list), ["a/", "b/", "d.rs", "e.rs"]);
        list.toggle_dir(0);
        assert_eq!(visible_names(&list), ["a/", "e.rs"]);
    }

    #[test]
    fn the_cursor_skips_folded_rows_and_stops_at_the_ends() {
        let mut list = tree();
        list.toggle_dir(1);
        list.move_cursor(2);
        assert_eq!(list.cursor, 3);
        assert_eq!(list.selected_entry(), Some(1));
        list.move_cursor(10);
        assert_eq!(list.cursor, 4);
        list.move_cursor(-10);
        assert_eq!(list.cursor, 0);
        assert_eq!(list.selected_entry(), None);
    }

    #[test]
    fn folding_over_the_cursor_moves_it_to_the_directory() {
        let mut list = tree();
        list.cursor = 2;
        list.toggle_dir(1);
        assert_eq!(list.cursor, 1);
    }
}
